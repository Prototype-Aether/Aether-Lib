use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::link::WINDOW_SIZE;
use crate::packet::Packet;

pub struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
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
