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
use crate::config::Config;
use crate::error::AetherError;
use crate::link::receivethread::ReceiveThread;
use crate::link::sendthread::SendThread;
use crate::packet::PType;
use crate::packet::Packet;

pub fn needs_ack(packet: &Packet) -> bool {
    match packet.flags.p_type {
        PType::Data => true,
        PType::AckOnly => false,
        _ => false,
    }
}

// Check if bundling acr mutex vars is possible
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
    config: Config,
}

impl Link {
    pub fn new(
        socket: UdpSocket,
        peer_addr: SocketAddr,
        send_seq: u32,
        recv_seq: u32,
        config: Config,
    ) -> Result<Link, AetherError> {
        let socket = Arc::new(socket);

        // if - let for errors
        match socket.set_read_timeout(Some(Duration::from_secs(1))) {
            Ok(_) => {}
            Err(_) => {
                return Err(AetherError::SetReadTimeout);
            }
        }

        let primary_queue = Arc::new(Mutex::new(VecDeque::new()));
        let output_queue = Arc::new(Mutex::new(VecDeque::new()));

        let stop_flag = Arc::new(Mutex::new(false));
        let batch_empty = Arc::new(Mutex::new(false));
        Ok(Link {
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
            config,
        })
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
            self.config,
        );

        // Start the send thread
        // Check for arc self if stable : https://stackoverflow.com/questions/25462935/what-types-are-valid-for-the-self-parameter-of-a-method
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
            self.config,
        );

        // Start the receive thread
        let recv_thread = thread::spawn(move || {
            recv_thread_data.start();
        });

        // Push the threads' join handles to join when stopping the link
        self.thread_handles.push(send_thread);
        self.thread_handles.push(recv_thread);
    }

    pub fn stop(&mut self) -> Result<(), AetherError> {
        // Set the stop flag
        match self.stop_flag.lock() {
            Ok(mut flag_lock) => {
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
                Ok(())
            }
            Err(_) => Err(AetherError::MutexLock("stop flag")),
        }
    }

    pub fn send(&self, buf: Vec<u8>) -> Result<(), AetherError> {
        // Lock seq number
        match self.send_seq.lock() {
            Ok(mut seq_lock) => {
                // Increase sequence number
                (*seq_lock) += 1;

                let seq: u32 = *seq_lock;

                // Unlock seq
                drop(seq_lock);

                // Create a new packet to be sent
                let mut packet = Packet::new(PType::Data, seq);
                packet.append_payload(buf);

                // Lock the primary queue
                match self.primary_queue.lock() {
                    Ok(mut queue_lock) => {
                        (*queue_lock).push_back(packet);
                        Ok(())
                    }
                    Err(_) => Err(AetherError::MutexLock("primary queue")),
                }

                // Push the new packet onto the primary queue
            }
            Err(_) => Err(AetherError::MutexLock("send queue")),
        }
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
                    Err(AetherError::LinkStopped("recv timeout"))
                } else {
                    // Pop the next packet from output queue
                    loop {
                        match now.elapsed() {
                            Ok(elapsed) => {
                                if elapsed > timeout {
                                    break Err(AetherError::RecvTimeout);
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
                                                        self.config.link.poll_time_us,
                                                    ));
                                                }
                                            };
                                        }
                                        Err(_) => {
                                            break Err(AetherError::MutexLock("output queue"));
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                break Err(AetherError::ElapsedTime(err));
                            }
                        }
                    }
                }
            }
            Err(_) => Err(AetherError::MutexLock("stop flag")),
        }
    }

    pub fn recv(&self) -> Result<Vec<u8>, AetherError> {
        match self.stop_flag.lock() {
            Ok(flag_lock) => {
                let stop = *flag_lock;
                drop(flag_lock);

                let now = SystemTime::now();

                if stop {
                    Err(AetherError::LinkStopped("recv"))
                } else {
                    // Pop the next packet from output queue
                    loop {
                        if let Some(time) = self.read_timeout {
                            match now.elapsed() {
                                Ok(elapsed) => {
                                    if elapsed > time {
                                        break Err(AetherError::LinkTimeout);
                                    }
                                }
                                Err(err) => {
                                    break Err(AetherError::ElapsedTime(err));
                                }
                            }
                        }

                        match self.output_queue.lock() {
                            Ok(mut queue_lock) => {
                                let result = queue_lock.pop_front();

                                drop(queue_lock);

                                // Get payload out of the packet and return
                                match result {
                                    Some(packet) => break Ok(packet.payload),
                                    None => {
                                        thread::sleep(Duration::from_micros(
                                            self.config.link.poll_time_us,
                                        ));
                                    }
                                };
                            }
                            Err(_) => {
                                break Err(AetherError::MutexLock("output queue"));
                            }
                        }
                    }
                }
            }
            Err(_) => Err(AetherError::MutexLock("stop flag")),
        }
    }

    pub fn is_empty(&self) -> Result<bool, AetherError> {
        match self.output_queue.lock() {
            Ok(queue_lock) => {
                let result = (*queue_lock).is_empty();
                drop(queue_lock);

                match self.batch_empty.lock() {
                    Ok(batch_lock) => Ok(result && (*batch_lock)),
                    Err(_) => Err(AetherError::MutexLock("batch empty flag")),
                }
            }
            Err(_) => Err(AetherError::MutexLock("output queue")),
        }
    }

    pub fn wait(&self) -> Result<(), AetherError> {
        loop {
            match self.is_empty() {
                Ok(empty) => {
                    if empty {
                        thread::sleep(Duration::from_millis(self.config.link.ack_wait_time));
                        break Ok(());
                    } else {
                        thread::sleep(Duration::from_micros(self.config.link.poll_time_us));
                    }
                }
                Err(aether_error) => {
                    break Err(aether_error);
                }
            }
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        match self.stop() {
            Ok(_) => {}
            Err(aether_error) => {
                log::error!("{}", aether_error)
            }
        }
    }
}
