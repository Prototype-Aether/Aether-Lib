use std::collections::VecDeque;

mod packet;
use packet::UDPPacket;

pub struct PacketQueue {
    in_queue: VecDeque<UDPPacket>,
    out_queue: VecDeque<UDPPacket>,
}

impl PacketQueue {
    pub fn new() -> PacketQueue {
        PacketQueue {
            in_queue: VecDeque::new(),
            out_queue: VecDeque::new(),
        }
    }

    pub fn send(&mut self, data: String) {
        let s_packet = UDPPacket {
            id: 0,
            sequence: 0,
            ack: 0,
            length: data.len(),
            payload: data,
        };
        self.out_queue.push_back(s_packet);
    }

    pub fn print(&self) {
        for p in &self.out_queue {
            println!("{} : {}", p.id, p.payload);
        }
    }
}
