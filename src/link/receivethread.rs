//use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::acknowledgement::{AcknowledgementCheck, AcknowledgementList};
use crate::config::Config;
use crate::link::needs_ack;
use crate::packet::PType;
use crate::packet::Packet;

pub struct OrderList {
    seq: u32,
    list: HashMap<u32, Packet>,
}

impl OrderList {
    pub fn new(seq: u32) -> OrderList {
        OrderList {
            seq,
            list: HashMap::new(),
        }
    }

    pub fn insert(&mut self, packet: Packet) -> Result<VecDeque<Packet>, u8> {
        if packet.sequence > self.seq + 1 {
            self.list.insert(packet.sequence, packet);
            Err(1)
        } else if packet.sequence == self.seq + 1 {
            let mut result: VecDeque<Packet> = VecDeque::new();
            result.push_back(packet);

            self.seq += 1;

            loop {
                match self.list.remove(&(self.seq + 1)) {
                    Some(n_packet) => {
                        self.seq += 1;
                        result.push_back(n_packet);
                    }
                    None => break Ok(result),
                }
            }
        } else {
            Err(0)
        }
    }
}

pub struct ReceiveThread {
    socket: Arc<UdpSocket>,
    _peer_addr: SocketAddr,
    output_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgementList>>,
    ack_check: Arc<Mutex<AcknowledgementCheck>>,

    order_list: OrderList,

    _recv_seq: Arc<Mutex<u32>>,

    config: Config,
}

impl ReceiveThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        output_queue: Arc<Mutex<VecDeque<Packet>>>,
        stop_flag: Arc<Mutex<bool>>,
        ack_check: Arc<Mutex<AcknowledgementCheck>>,
        ack_list: Arc<Mutex<AcknowledgementList>>,
        recv_seq: Arc<Mutex<u32>>,
        config: Config,
    ) -> ReceiveThread {
        let recv_lock = recv_seq.lock().expect("Unable to lock recv_seq");
        let seq = *recv_lock;

        drop(recv_lock);

        ReceiveThread {
            socket,
            _peer_addr: peer_addr,
            output_queue,
            stop_flag,
            ack_check,
            ack_list,
            _recv_seq: recv_seq,
            order_list: OrderList::new(seq),
            config,
        }
    }

    pub fn start(&mut self) {
        let mut buf = [0; 512];
        //println!("Starting receive thread...");
        let mut now = SystemTime::now();
        loop {
            // If stop flag is set stop the thread
            let flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
            if *flag_lock {
                break;
            }

            // Unlock flag
            drop(flag_lock);

            /* Simulate packet loss
            if thread_rng().gen_range(0..100) < 99 {
                continue;
            }*/

            let size = match self.socket.recv(&mut buf) {
                Ok(result) => result,
                _ => 0,
            };

            if size > 0 {
                now = SystemTime::now();
                let packet = Packet::from(buf[..size].to_vec());
                let exists = self.check_ack(&packet);
                self.recv_ack(&packet);
                self.send_ack(&packet);
                if !exists {
                    self.output(packet);
                }
            } else {
                let elapsed = now.elapsed().expect("unable to get system time");
                if elapsed.as_millis() > self.config.link.timeout.into() {
                    let mut flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
                    *flag_lock = true;
                }
            }
        }
        //println!("Stopping receive thread...");
    }

    fn check_ack(&self, packet: &Packet) -> bool {
        let ack_lock = self.ack_list.lock().expect("Unable to lack ack list");
        (*ack_lock).check(&packet.sequence)
    }

    fn send_ack(&self, packet: &Packet) {
        if needs_ack(&packet) {
            let mut ack_lock = self.ack_list.lock().expect("Unable to lack ack list");
            (*ack_lock).insert(packet.sequence);
        }
    }

    fn recv_ack(&self, packet: &Packet) {
        let mut ack_lock = self.ack_check.lock().expect("unable to lock ack check");
        (*ack_lock).acknowledge(packet.ack.clone());
    }

    fn output(&mut self, packet: Packet) {
        match packet.flags.p_type {
            PType::AckOnly => (),
            _ => self.order_output(packet),
        }
    }

    fn order_output(&mut self, packet: Packet) {
        match self.order_list.insert(packet) {
            Ok(mut packets) => loop {
                match packets.pop_front() {
                    Some(p) => {
                        let mut output_lock =
                            self.output_queue.lock().expect("Cannot lock output queue");
                        (*output_lock).push_back(p);
                    }
                    None => break,
                }
            },
            Err(1) => (),
            Err(0) => panic!("Sequence number too old"),
            _ => panic!("Unexpected error"),
        }
    }
}
