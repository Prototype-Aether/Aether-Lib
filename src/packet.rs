use crate::acknowledgment::Acknowledgment;
use crate::util::{compile_u16, compile_u32};

use std::convert::From;
use std::convert::TryInto;
use std::vec::Vec;

pub struct Packet {
    pub id: u32,
    pub sequence: u32,
    pub ack: Acknowledgment,
    pub length: u16,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(id: u32, sequence: u32) -> Packet {
        Packet {
            id,
            sequence,
            ack: Acknowledgment {
                ack_begin: 0,
                ack_end: 0,
                miss_count: 0,
                miss: Vec::new(),
            },
            length: 0,
            payload: Vec::new(),
        }
    }

    pub fn add_ack(&mut self, ack: Acknowledgment) {
        self.ack = ack;
    }

    pub fn append_payload(&mut self, payload: Vec<u8>) {
        self.payload.extend(payload);
        self.length = self.payload.len() as u16;
    }

    pub fn compile(&self) -> Vec<u8> {
        // Vector to store final compiled packet structure
        let mut packet_vector = Vec::<u8>::new();

        // Packet ID converting u32 to u8(vector)
        let slice_id = compile_u32(self.id);
        packet_vector.extend(slice_id);

        // Packet Sequence converting u32 to u8(vector)
        let slice_sequence = compile_u32(self.sequence);
        packet_vector.extend(slice_sequence);

        // Packet Ack Begin converting u32 to u8(vector)
        let slice_ack_begin = compile_u32(self.ack.ack_begin);
        packet_vector.extend(slice_ack_begin);

        packet_vector.push(self.ack.ack_end);

        packet_vector.push(self.ack.miss_count);

        let slice_miss = self.ack.miss.clone();
        packet_vector.extend(slice_miss);

        // Packet Length converting u16 to u8(vector)
        let slice_length = compile_u16(self.length);
        packet_vector.extend(slice_length);

        let slice_payload = self.payload.clone();
        packet_vector.extend(slice_payload);

        // currently the packet_vector is a vector of u8 but we have to convert into string and then into bytes
        packet_vector
    }
}

impl From<Vec<u8>> for Packet {
    // Create a packet structure from the received raw bytes
    fn from(bytes: Vec<u8>) -> Packet {
        let mut packet_default = Packet {
            id: 0,
            sequence: 0,
            ack: Acknowledgment {
                ack_begin: 0,
                ack_end: 0,
                miss_count: 0,
                miss: Vec::new(),
            },
            length: 0,
            payload: Vec::new(),
        };

        // Packet ID converting u8 to u32(vector)
        let id_array = bytes[0..4].try_into().unwrap();
        packet_default.id = u32::from_be_bytes(id_array);

        // Packet Sequence converting u8 to u32(vector)
        let sequence_array = bytes[4..8].try_into().unwrap();
        packet_default.sequence = u32::from_be_bytes(sequence_array);

        // Packet Ack Begin converting u8 to u32(vector)
        let ack_begin_array = bytes[8..12].try_into().unwrap();
        packet_default.ack.ack_begin = u32::from_be_bytes(ack_begin_array);

        packet_default.ack.ack_end = bytes[12];

        packet_default.ack.miss_count = bytes[13];

        packet_default.ack.miss = bytes[14..14 + packet_default.ack.miss_count as usize].to_vec();

        // Packet Length converting u8 to u16(vector)
        let length_array = bytes[14 + packet_default.ack.miss_count as usize
            ..16 + packet_default.ack.miss_count as usize]
            .try_into()
            .unwrap();
        packet_default.length = u16::from_be_bytes(length_array);

        packet_default.payload = bytes[16 + packet_default.ack.miss_count as usize
            ..16 + packet_default.ack.miss_count as usize + packet_default.length as usize]
            .to_vec();

        packet_default
    }
}

#[cfg(test)]
mod tests {
    use crate::{acknowledgment::AcknowledgmentList, packet};

    #[test]
    fn range_test() {
        let pack = packet::Packet::new(0, 0);
        assert!(pack.ack.ack_begin <= pack.ack.ack_end.into());
        assert!(pack.ack.miss_count as u32 <= (pack.ack.ack_end as u32 - pack.ack.ack_begin));
    }

    #[test]
    fn compile_test() {
        let mut pack = packet::Packet::new(52, 32);
        let mut ack_list = AcknowledgmentList::new(65);
        ack_list.insert(66);
        ack_list.insert(67);
        ack_list.insert(69);
        ack_list.insert(70);

        pack.add_ack(ack_list.get());
        pack.append_payload(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let compiled = pack.compile();

        let pack_out = packet::Packet::from(compiled);

        assert_eq!(pack.id, pack_out.id);
        assert_eq!(pack.sequence, pack_out.sequence);
        assert_eq!(pack.ack.ack_begin, pack_out.ack.ack_begin);
        assert_eq!(pack.ack.ack_end, pack_out.ack.ack_end);
        assert_eq!(pack.ack.miss_count, pack_out.ack.miss_count);
        assert_eq!(pack.ack.miss, pack_out.ack.miss);
        assert_eq!(pack.length, pack_out.length);
        assert_eq!(pack.payload, pack_out.payload);
    }
}
