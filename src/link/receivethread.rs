use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::packet::Packet;

pub struct ReceiveThread {
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    output_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
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
        if packet.payload.len() > 0 {
            let mut output_lock = self.output_queue.lock().expect("Cannot lock output queue");
            (*output_lock).push_back(packet);
        }
    }
}
