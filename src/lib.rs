/*mod packet {
    pub struct UDPPacket {
        pub id: u32,
        pub sequence: u32,
        pub ack: u32,
        pub length: usize,
        pub payload: String,
    }
}
*/

pub mod packet;

use packet::UDPPacket;
use std::collections::VecDeque;

#[allow(dead_code)]
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
        let s_packet = packet::UDPPacket {
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
