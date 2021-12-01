use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::link::{needs_ack, WINDOW_SIZE};
use crate::packet::PType;
use crate::packet::Packet;

pub struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    is_empty: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,

    send_seq: Arc<Mutex<u32>>,
}

impl SendThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        primary_queue: Arc<Mutex<VecDeque<Packet>>>,
        stop_flag: Arc<Mutex<bool>>,
        ack_check: Arc<Mutex<AcknowledgmentCheck>>,
        ack_list: Arc<Mutex<AcknowledgmentList>>,
        send_seq: Arc<Mutex<u32>>,
        is_empty: Arc<Mutex<bool>>,
    ) -> SendThread {
        SendThread {
            batch_queue: VecDeque::new(),
            socket,
            peer_addr,
            primary_queue,
            stop_flag,
            ack_check,
            ack_list,
            send_seq,
            is_empty,
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
                None => {
                    self.fetch_window();
                    let mut empty_lock = self.is_empty.lock().expect("Unable to lock empty bool");

                    // If still empty
                    if self.batch_queue.is_empty() {
                        (*empty_lock) = true;
                        // Send a ack only packet (with empty payload)
                        self.batch_queue.push_back(self.ack_packet());
                    } else {
                        (*empty_lock) = false;
                    }

                    drop(empty_lock);
                }
            }
        }

        println!("Stopping send thread...");
    }

    pub fn is_empty(&self) -> bool {
        let empty_lock = self.is_empty.lock().expect("Unable to lock empty bool");
        *empty_lock
    }

    pub fn ack_packet(&self) -> Packet {
        // Lock seq number
        let seq_lock = self.send_seq.lock().expect("Unable to lock seq");
        // Increase sequence number

        let seq: u32 = *seq_lock;

        // Create a new packet to be sent
        Packet::new(PType::AckOnly, seq)
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
        if needs_ack(&packet) {
            let ack_lock = self.ack_check.lock().expect("Unable to lock ack list");
            (*ack_lock).check(&packet.sequence)
        } else {
            false
        }
    }

    pub fn add_ack(&self, packet: &mut Packet) {
        let ack_lock = self.ack_list.lock().expect("Unable to lock ack list");
        let ack = (*ack_lock).get();
        packet.add_ack(ack);
    }

    pub fn send(&mut self, packet: Packet) {
        let data = packet.compile();

        let result = self
            .socket
            .send_to(&data, self.peer_addr)
            .expect("Unable to send data");

        if result == 0 {
            panic!("Cannot sent");
        }

        if needs_ack(&packet) {
            self.batch_queue.push_back(packet);
        }
    }
}
