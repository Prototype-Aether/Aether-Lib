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
    fn compile_u32(nu32: u32) -> Vec<u8> {
        let mut u32_vec = Vec::<u8>::new();
        u32_vec.push((nu32 >> 24) as u8);
        u32_vec.push((nu32 >> 16) as u8);
        u32_vec.push((nu32 >> 8) as u8);
        u32_vec.push(nu32 as u8);
        u32_vec
    }
    pub fn compile(self) -> Vec<u8> {
        
        let packet_default = Packet{
            id: self.id,
            sequence: self.sequence,
            ack_begin: self.ack_begin,
            ack_end: self.ack_end,
            miss_count: self.miss_count,
            miss: self.miss,
            length: self.length,
            payload: self.payload,
        };

        // <Uncomment the below code for testing>
        // let packet_default=Packet {
        //     id: 8008,
        //     sequence: 8008,
        //     ack_begin: 0,
        //     ack_end: 0,
        //     miss_count: 0,
        //     miss: vec![1,2,3,4,5,6],
        //     length: 10,
        //     payload: vec![1,2,3,4,5,6,7,8,9,10],
        // };

        // Vector to store final compiled packet structure
        let mut packet_vector= Vec::<u8>::new();
        // Packet ID converting u32 to u8(vector)
        let slice_id = Packet::compile_u32(packet_default.id);
        packet_vector.extend(slice_id);
        // Packet Sequence converting u32 to u8(vector)
        let slice_sequence = Packet::compile_u32(packet_default.sequence);
        packet_vector.extend(slice_sequence);
        // Packet Ack Begin converting u32 to u8(vector)
        let slice_ack_begin = Packet::compile_u32(packet_default.ack_begin);
        packet_vector.extend(slice_ack_begin);
        // Packet Ack End converting u8 to u8(vector)
        packet_vector.push(packet_default.ack_end);
        // Packet Miss Count converting u8 to u8(vector)
        packet_vector.push(packet_default.miss_count);
        // Packet Miss converting u8 to u8(vector)
        packet_vector.extend(packet_default.miss);
        // Packet Length converting u16 to u8(vector)
        let slice_length = Packet::compile_u16(packet_default.length);
        packet_vector.extend(slice_length);
        // Packet Payload converting u8 to u8(vector)
        packet_vector.extend(packet_default.payload);

        // currently the packet_vector is a vector of u8 but we have to convert into string and then into bytes
        packet_vector
    }
}

impl From<Vec<u8>> for Packet {
    // Create a packet structure from the received raw bytes
    fn from(bytes: Vec<u8>) -> Packet {
        Packet::new(0, 0)
    }
}
