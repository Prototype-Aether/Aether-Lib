pub mod receivethread;
pub mod sendthread;

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::SystemTime;

use crate::acknowledgement::{AcknowledgementCheck, AcknowledgementList};
use crate::error::AetherError;
use crate::link::receivethread::ReceiveThread;
use crate::link::sendthread::SendThread;
use crate::packet::PType;
use crate::packet::Packet;

pub const WINDOW_SIZE: u8 = 20;
pub const ACK_WAIT_TIME: u64 = 1000;
pub const POLL_TIME_US: u64 = 100;
pub const TIMEOUT: u64 = 10_000;
pub const RETRY_DELAY: u64 = 100;
pub const MAX_RETRIES: i16 = 10;

pub fn needs_ack(packet: &Packet) -> bool {
    match packet.flags.p_type {
        PType::Data => true,
        PType::AckOnly => false,
        _ => false,
    }
}

pub struct Link {
    ack_list: Arc<Mutex<AcknowledgementList>>,
    ack_check: Arc<Mutex<AcknowledgementCheck>>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    output_queue: Arc<Mutex<VecDeque<Packet>>>,
    thread_handles: Vec<JoinHandle<()>>,
    send_seq: Arc<Mutex<u32>>,
    recv_seq: Arc<Mutex<u32>>,
    stop_flag: Arc<Mutex<bool>>,
    batch_empty: Arc<Mutex<bool>>,
    read_timeout: Option<Duration>,
}

impl Link {
    pub fn new(socket: UdpSocket, peer_addr: SocketAddr, send_seq: u32, recv_seq: u32) -> Link {
        let socket = Arc::new(socket);
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("Unable to set timeout");

        let primary_queue = Arc::new(Mutex::new(VecDeque::new()));
        let output_queue = Arc::new(Mutex::new(VecDeque::new()));

        let stop_flag = Arc::new(Mutex::new(false));
        let batch_empty = Arc::new(Mutex::new(false));
        Link {
            ack_list: Arc::new(Mutex::new(AcknowledgementList::new(recv_seq))),
            ack_check: Arc::new(Mutex::new(AcknowledgementCheck::new(send_seq))),
            peer_addr,
            socket,
            primary_queue,
            output_queue,
            send_seq: Arc::new(Mutex::new(send_seq)),
            recv_seq: Arc::new(Mutex::new(recv_seq)),
            thread_handles: Vec::new(),
            stop_flag,
            batch_empty,
            read_timeout: None,
        }
    }

    pub fn start(&mut self) {
        // Create data structure for the send thread
        let mut send_thread_data = SendThread::new(
            self.socket.clone(),
            self.peer_addr,
            self.primary_queue.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
            self.send_seq.clone(),
            self.batch_empty.clone(),
        );

        // Start the send thread
        let send_thread = thread::spawn(move || {
            send_thread_data.start();
        });

        // Create data strcuture for the receive thread
        let mut recv_thread_data = ReceiveThread::new(
            self.socket.clone(),
            self.peer_addr,
            self.output_queue.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
            self.recv_seq.clone(),
        );

        // Start the receive thread
        let recv_thread = thread::spawn(move || {
            recv_thread_data.start();
        });

        // Push the threads' join handles to join when stopping the link
        self.thread_handles.push(send_thread);
        self.thread_handles.push(recv_thread);
    }

    pub fn stop(&mut self) {
        // Set the stop flag
        let mut flag_lock = self.stop_flag.lock().expect("Unable to lock stop flag");
        *flag_lock = true;

        // Unlock stop flag
        drop(flag_lock);

        // Join each thread
        while match self.thread_handles.pop() {
            Some(handle) => {
                handle.join().expect("Thread failed to join");
                true
            }
            None => false,
        } {}
    }

    pub fn send(&self, buf: Vec<u8>) {
        // Lock seq number
        let mut seq_lock = self.send_seq.lock().expect("Unable to lock seq");
        // Increase sequence number
        (*seq_lock) += 1;

        let seq: u32 = *seq_lock;

        // Unlock seq
        drop(seq_lock);

        // Create a new packet to be sent
        let mut packet = Packet::new(PType::Data, seq);
        packet.append_payload(buf);

        // Lock the primary queue
        let mut queue_lock = self
            .primary_queue
            .lock()
            .expect("Unable to lock primary queue");

        // Push the new packet onto the primary queue
        (*queue_lock).push_back(packet);
    }

    pub fn set_read_timout(&mut self, timeout: Duration) {
        self.read_timeout = Some(timeout);
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Vec<u8>, AetherError> {
        match self.stop_flag.lock() {
            Ok(flag_lock) => {
                let stop = *flag_lock;
                drop(flag_lock);

                let now = SystemTime::now();

                if stop {
                    let aetherError = AetherError {
                        code: 1001,
                        description: String::from("Link Module Terminated."),
                        cause: None,
                    };
                    //log::error!("{}",aetherError);
                    Err(aetherError)
                } else {
                    // Pop the next packet from output queue
                    loop {
                        match now.elapsed() {
                            Ok(elapsed) => {
                                if elapsed > timeout {
                                    let aetherError = AetherError {
                                        code: 1002,
                                        description: String::from("Function timed out"),
                                        cause: None,
                                    };
                                    log::error!("{}", aetherError);
                                    break Err(aetherError);
                                } else {
                                    match self.output_queue.lock() {
                                        Ok(mut queue_lock) => {
                                            let result = queue_lock.pop_front();

                                            drop(queue_lock);

                                            // Get payload out of the packet and return
                                            match result {
                                                Some(packet) => break Ok(packet.payload),
                                                None => {
                                                    thread::sleep(Duration::from_micros(
                                                        POLL_TIME_US,
                                                    ));
                                                }
                                            };
                                        }
                                        Err(_) => {
                                            let aetherError = AetherError {
                                                code: 1003,
                                                description: String::from("Failed to lock mutex."),
                                                cause: None,
                                            };
                                            log::error!("{}", aetherError);
                                            break Err(aetherError);
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                let aetherError = AetherError {
                                code: 1000,
                                description: String::from(
                                    "Packet arrived before Link Module initialized receive method.",
                                ),
                                cause: None,
                            };
                                log::error!("{}", aetherError);
                                break Err(aetherError);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                let aetherError = AetherError {
                    code: 1003,
                    description: String::from("Failed to lock mutex."),
                    cause: None,
                };
                log::error!("{}", aetherError);
                Err(aetherError)
            }
        }
    }
    pub fn recv(&self) -> Result<Vec<u8>, AetherError> {
        match self.stop_flag.lock() {
            Ok(flag_lock) => {
                let stop = *flag_lock;
                drop(flag_lock);

                let now = SystemTime::now();

                if stop {
                    let aetherError = AetherError {
                        code: 1001,
                        description: String::from("Link Module terminated."),
                        cause: None,
                    };
                    Err(aetherError)
                } else {
                    // Pop the next packet from output queue
                    loop {
                        match self.read_timeout {
                            Some(time) => match now.elapsed() {
                                Ok(elapsed) => {
                                    if elapsed > time {
                                        let aetherError = AetherError {
                                            code: 1002,
                                            description: String::from("Function timed out"),
                                            cause: None,
                                        };
                                        log::error!("{}", aetherError);
                                        break Err(aetherError);
                                    }
                                }
                                Err(_) => {
                                    let aetherError = AetherError {
                                code: 1000,
                                description: String::from(
                                    "Packet arrived before Link Module initialized receive method.",
                                ),
                                cause: None,
                            };
                                    log::error!("{}", aetherError);
                                    break Err(aetherError);
                                }
                            },
                            None => (),
                        }

                        match self.output_queue.lock() {
                            Ok(queue_lock) => {
                                let result = queue_lock.pop_front();

                                drop(queue_lock);

                                // Get payload out of the packet and return
                                match result {
                                    Some(packet) => break Ok(packet.payload),
                                    None => {
                                        thread::sleep(Duration::from_micros(POLL_TIME_US));
                                    }
                                };
                            }
                            Err(_) => {
                                let aetherError = AetherError {
                                    code: 1003,
                                    description: String::from("Failed to lock mutex."),
                                    cause: None,
                                };
                                log::error!("{}", aetherError);
                                break Err(aetherError);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                let aetherError = AetherError {
                    code: 1003,
                    cause: None,
                    description: String::from("Faled to lock mutex."),
                };
                log::error!("{}", aetherError);
                Err(aetherError)
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let queue_lock = self.output_queue.lock().expect("Cannot lock output queue");
        let result = (*queue_lock).is_empty();
        drop(queue_lock);

        let batch_lock = self.batch_empty.lock().expect("Cannot lock batch queue");

        result && (*batch_lock)
    }

    pub fn wait(&self) {
        loop {
            if self.is_empty() {
                thread::sleep(Duration::from_millis(ACK_WAIT_TIME));
                break;
            }
            thread::sleep(Duration::from_micros(POLL_TIME_US));
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop();
    }
}
