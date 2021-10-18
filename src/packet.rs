use std::convert::From;
use std::vec::Vec;

pub struct Packet {
    pub id: u32,
    pub sequence: u32,
    pub ack_begin: u32,
    pub ack_end: u8,
    pub miss_count: u8,
    pub miss: Vec<u8>,
    pub length: u16,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(id: u32, sequence: u32) -> Packet {
        Packet {
            id,
            sequence,
            ack_begin: 0,
            ack_end: 0,
            miss_count: 0,
            miss: Vec::new(),
            length: 0,
            payload: Vec::new(),
        }
    }

    // compile the packet structure into raw bytes
    pub fn compile() -> Vec<u8> {
        Vec::new()
    }
}

impl From<Vec<u8>> for Packet {
    // Create a packet structure from the received raw bytes
    fn from(bytes: Vec<u8>) -> Packet {
        Packet::new(0, 0)
    }
}
