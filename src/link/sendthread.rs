use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::acknowledgement::{AcknowledgementCheck, AcknowledgementList};
use crate::config::Config;
use crate::link::needs_ack;
use crate::packet::PType;
use crate::packet::Packet;
use crate::packet::PacketMeta;

pub struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    stop_flag: Arc<Mutex<bool>>,

    is_empty: Arc<Mutex<bool>>,

    ack_list: Arc<Mutex<AcknowledgementList>>,
    ack_check: Arc<Mutex<AcknowledgementCheck>>,

    send_seq: Arc<Mutex<u32>>,

    config: Config,
}

impl SendThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        primary_queue: Arc<Mutex<VecDeque<Packet>>>,
        stop_flag: Arc<Mutex<bool>>,
        ack_check: Arc<Mutex<AcknowledgementCheck>>,
        ack_list: Arc<Mutex<AcknowledgementList>>,
        send_seq: Arc<Mutex<u32>>,
        is_empty: Arc<Mutex<bool>>,
        config: Config,
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
            config,
        }
    }

    pub fn start(&mut self) {
        //println!("Starting send thread...");
        loop {
            // If stop flag is set stop the thread
            let flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
            if *flag_lock {
                break;
            }

            drop(flag_lock);

            match self.batch_queue.pop_front() {
                Some(mut packet) => {
                    if packet.is_meta {
                        if !self.batch_queue.is_empty() {
                            // If this is a meta packet check if it requires a delay
                            if packet.meta.delay_ms > 0 {
                                thread::sleep(Duration::from_millis(packet.meta.delay_ms));
                            }

                            // Increase retry count since after this same packets
                            // will be sent again
                            let retry_count = packet.meta.retry_count + 1;

                            if retry_count >= self.config.link.max_retries {
                                // Stop connection if too many retries
                                let mut flag_lock =
                                    self.stop_flag.lock().expect("Error locking stop flag");
                                *flag_lock = true;
                            } else {
                                let mut meta_packet = Packet::new(PType::Extended, 0);

                                meta_packet.set_meta(PacketMeta {
                                    retry_count,
                                    delay_ms: self.config.link.retry_delay,
                                });

                                self.batch_queue.push_back(meta_packet);
                            }
                        }
                    } else if !self.check_ack(&packet) {
                        self.add_ack(&mut packet);
                        self.send(packet);
                    }
                }
                None => {
                    self.fetch_window();
                    let mut empty_lock = self.is_empty.lock().expect("Unable to lock empty bool");

                    let retry_delay = 0;
                    // If still empty
                    if self.batch_queue.is_empty() {
                        (*empty_lock) = true;
                        // Send a ack only packet (with empty payload)
                        self.batch_queue.push_back(self.ack_packet());
                    } else {
                        (*empty_lock) = false;
                    }

                    drop(empty_lock);

                    // At end of each window push a meta packet
                    // This is to keep track of number of retries
                    let mut meta_packet = Packet::new(PType::Extended, 0);

                    // Retry count here is -1 so after trying once it is set to 0
                    meta_packet.set_meta(PacketMeta {
                        retry_count: -1,
                        delay_ms: retry_delay,
                    });

                    self.batch_queue.push_back(meta_packet);
                }
            }
        }

        //println!("Stopping send thread...");
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

        for _ in 0..self.config.link.window_size {
            match (*queue).pop_front() {
                Some(packet) => self.batch_queue.push_back(packet),
                None => break,
            }
        }
    }

    pub fn check_ack(&self, packet: &Packet) -> bool {
        if needs_ack(packet) {
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
