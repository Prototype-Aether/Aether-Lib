use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::packet::Packet;

const WINDOW_SIZE: u8 = 20;

pub struct Link {
    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    output_queue: Arc<Mutex<VecDeque<Packet>>>,
    thread_handles: Vec<JoinHandle<()>>,
    send_seq: u32,
    recv_seq: u32,
    stop_flag: Arc<Mutex<bool>>,
}

struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
}

struct ReceiveThread {
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    output_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
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
        Link {
            ack_list: Arc::new(Mutex::new(AcknowledgmentList::new(recv_seq))),
            ack_check: Arc::new(Mutex::new(AcknowledgmentCheck::new(send_seq))),
            peer_addr,
            socket,
            primary_queue,
            output_queue,
            send_seq,
            recv_seq,
            thread_handles: Vec::new(),
            stop_flag,
        }
    }

    pub fn start(&mut self) {
        // Create data structure for the send thread
        let mut send_thread_data = SendThread::new(
            self.socket.clone(),
            self.peer_addr.clone(),
            self.primary_queue.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
        );

        // Start the send thread
        let send_thread = thread::spawn(move || {
            send_thread_data.start();
        });

        // Create data strcuture for the receive thread
        let recv_thread_data = ReceiveThread::new(
            self.socket.clone(),
            self.peer_addr.clone(),
            self.output_queue.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
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

    pub fn send(&mut self, buf: Vec<u8>) {
        // Increase sequence number
        self.send_seq += 1;

        // Create a new packet to be sent
        let mut packet = Packet::new(10, self.send_seq);
        packet.append_payload(buf);

        // Lock the primary queue
        let mut queue_lock = self
            .primary_queue
            .lock()
            .expect("Unable to lock primary queue");

        // Push the new packet onto the primary queue
        (*queue_lock).push_back(packet);
    }

    pub fn recv(&mut self) -> Result<Vec<u8>, u8> {
        // Pop the next packet from output queue
        loop {
            let mut queue_lock = self.output_queue.lock().expect("Cannot lock output queue");

            let result = queue_lock.pop_front();

            drop(queue_lock);

            // Get payload out of the packet and return
            match result {
                Some(packet) => break Ok(packet.payload),
                None => {
                    thread::sleep(Duration::from_micros(100));
                }
            };
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop();
    }
}

impl SendThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        primary_queue: Arc<Mutex<VecDeque<Packet>>>,
        stop_flag: Arc<Mutex<bool>>,
        ack_check: Arc<Mutex<AcknowledgmentCheck>>,
        ack_list: Arc<Mutex<AcknowledgmentList>>,
    ) -> SendThread {
        SendThread {
            batch_queue: VecDeque::new(),
            socket,
            peer_addr,
            primary_queue,
            stop_flag,
            ack_check,
            ack_list,
        }
    }

    pub fn start(&mut self) {
        println!("Starting send thread...");
        loop {
            // If stop flag is set stop the thread
            let flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
            if *flag_lock {
                break;
            }

            drop(flag_lock);

            match self.batch_queue.pop_front() {
                Some(mut packet) => {
                    if !self.check_ack(&packet) {
                        self.add_ack(&mut packet);
                        self.send(packet);
                    }
                }
                None => self.fetch_window(),
            }
        }

        println!("Stopping send thread...");
    }

    pub fn fetch_window(&mut self) {
        // Lock primary queue and dequeue the packet
        let mut queue = self.primary_queue.lock().expect("Error locking queue");

        for _ in 0..WINDOW_SIZE {
            match (*queue).pop_front() {
                Some(packet) => self.batch_queue.push_back(packet),
                None => break,
            }
        }
    }

    pub fn check_ack(&self, packet: &Packet) -> bool {
        let ack_lock = self.ack_check.lock().expect("Unable to lock ack list");
        (*ack_lock).check(&packet.sequence)
    }

    pub fn add_ack(&self, packet: &mut Packet) {
        let ack_lock = self.ack_list.lock().expect("Unable to lock ack list");
        let ack = (*ack_lock).get();
        packet.add_ack(ack);
    }

    pub fn send(&mut self, packet: Packet) {
        let data = packet.compile();
        self.socket
            .send_to(&data, self.peer_addr)
            .expect("Unable to send data");

        self.batch_queue.push_back(packet);
    }
}

impl ReceiveThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        output_queue: Arc<Mutex<VecDeque<Packet>>>,
        stop_flag: Arc<Mutex<bool>>,
        ack_check: Arc<Mutex<AcknowledgmentCheck>>,
        ack_list: Arc<Mutex<AcknowledgmentList>>,
    ) -> ReceiveThread {
        ReceiveThread {
            socket,
            peer_addr,
            output_queue,
            stop_flag,
            ack_check,
            ack_list,
        }
    }

    pub fn start(&self) {
        let mut buf = [0; 512];
        println!("Starting receive thread...");
        loop {
            // If stop flag is set stop the thread
            let flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
            if *flag_lock {
                break;
            }

            // Unlock flag
            drop(flag_lock);

            let size = match self.socket.recv(&mut buf) {
                Ok(result) => result,
                _ => 0,
            };

            if size > 0 {
                let packet = Packet::from(buf[..size].to_vec());
                self.send_ack(&packet);
                self.recv_ack(&packet);
                self.output(packet);
            }
        }
        println!("Stopping receive thread...");
    }

    fn send_ack(&self, packet: &Packet) {
        let mut ack_lock = self.ack_list.lock().expect("Unable to lack ack list");
        (*ack_lock).insert(packet.sequence);
    }

    fn recv_ack(&self, packet: &Packet) {
        let mut ack_lock = self.ack_check.lock().expect("unable to lock ack check");
        (*ack_lock).acknowledge(packet.ack.clone());
    }

    fn output(&self, packet: Packet) {
        let mut output_lock = self.output_queue.lock().expect("Cannot lock output queue");
        (*output_lock).push_back(packet);
    }
}
