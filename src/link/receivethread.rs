//use rand::{thread_rng, Rng};
use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

use crossbeam::channel::Sender;

use crate::acknowledgement::{AcknowledgementCheck, AcknowledgementList};
use crate::config::Config;
use crate::link::needs_ack;
use crate::packet::PType;
use crate::packet::Packet;

/// Data structure to facilitate ordering of incoming packets by their sequence number.
pub struct OrderList {
    /// Last sequence number till which the packets are ordered.
    seq: u32,
    /// [`HashMap`] of packets by their sequence numbers
    list: HashMap<u32, Packet>,
}

impl OrderList {
    /// Creates a new [`OrderList`] with the starting sequence number `seq`.
    pub fn new(seq: u32) -> OrderList {
        OrderList {
            seq,
            list: HashMap::new(),
        }
    }

    /// Insert a packet into the [`OrderList`]
    /// # Arguments
    /// * `packet` - The packet to be inserted
    /// # Returns
    /// * `VecDeque` - The list of packets that are sequnced till now
    /// # Errors
    /// * [`Err(0)`] - If the packet received has already been sequenced before
    /// * [`Err(1)`] - If no sequnce of packets can be returned till now ???.
    pub fn insert(&mut self, packet: Packet) -> Result<VecDeque<Packet>, u8> {
        match (self.seq).cmp(&(packet.sequence - 1)) {
            Ordering::Less => {
                self.list.insert(packet.sequence, packet);
                Err(1)
            }
            Ordering::Equal => {
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
            }
            _ => Err(0),
        }
    }
}

/// Data structure to group data used by the receive thread
pub struct ReceiveThread {
    /// The socket used to receive packets
    socket: Arc<UdpSocket>,
    /// Address of the other peer
    _peer_addr: SocketAddr,
    /// Reference to the output queue from [`crate::link::Link`]
    receive_queue: Sender<Packet>,
    /// Reference to the stop flag from [`crate::link::Link`]
    stop_flag: Arc<Mutex<bool>>,
    /// Reference to the [`AcknowledgementList`] from [`crate::link::Link`]
    ack_list: Arc<Mutex<AcknowledgementList>>,
    /// Reference to the [`AcknowledgementCheck`] from [`crate::link::Link`]
    ack_check: Arc<Mutex<AcknowledgementCheck>>,
    /// [`OrderList`] used to order received packets by their sequence number
    order_list: OrderList,
    /// Reference to receive sequence from [`crate::link::Link`]
    _recv_seq: Arc<Mutex<u32>>,
    /// Current configuration for Aether
    config: Config,
}

impl ReceiveThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        receive_queue: Sender<Packet>,
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
            receive_queue,
            stop_flag,
            ack_check,
            ack_list,
            _recv_seq: recv_seq,
            order_list: OrderList::new(seq),
            config,
        }
    }

    pub fn start(&mut self) {
        let mut buf = [0; 2048];
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
    }

    fn check_ack(&self, packet: &Packet) -> bool {
        let ack_lock = self.ack_list.lock().expect("Unable to lack ack list");
        (*ack_lock).check(&packet.sequence)
    }

    fn send_ack(&self, packet: &Packet) {
        if needs_ack(packet) {
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
            Ok(mut packets) => {
                while let Some(p) = packets.pop_front() {
                    self.receive_queue
                        .send(p)
                        .expect("Unable to push to output queue");
                }
            }
            Err(1) => (),
            Err(0) => panic!("Sequence number too old"),
            _ => panic!("Unexpected error"),
        }
    }
}
