use crate::acknowledgement::Acknowledgement;
use crate::util::compile_u32;

use std::convert::From;
use std::convert::TryInto;
use std::vec::Vec;

#[derive(Debug, Clone)]
pub enum PType {
    Data,
    AckOnly,
    Initiation,
    KeyExchange,
    Extended,
}

impl From<PType> for u8 {
    fn from(p_type: PType) -> u8 {
        match p_type {
            PType::Data => 0,
            PType::AckOnly => 1,
            PType::Initiation => 2,
            PType::KeyExchange => 7,
            PType::Extended => 15,
        }
    }
}

impl From<u8> for PType {
    fn from(p_type: u8) -> PType {
        match p_type {
            0 => PType::Data,
            1 => PType::AckOnly,
            2 => PType::Initiation,
            7 => PType::KeyExchange,
            _ => PType::Extended,
        }
    }
}

impl PartialEq for PType {
    fn eq(&self, other: &Self) -> bool {
        (self.clone() as u8) == (other.clone() as u8)
    }
}

#[derive(Debug)]
pub struct PacketFlags {
    pub p_type: PType,
    pub ack: bool,
    pub enc: bool,
}

impl PacketFlags {
    pub fn get_byte(&self) -> u8 {
        let mut byte: u8 = 0;
        byte |= (self.p_type.clone() as u8) << 4;
        if self.ack {
            byte |= 1 << 3;
        }
        if self.enc {
            byte |= 1 << 2;
        }
        byte
    }
}

#[derive(Debug)]
pub struct PacketMeta {
    pub delay_ms: u64,
    pub retry_count: i16,
}

#[derive(Debug)]
pub struct Packet {
    pub flags: PacketFlags,
    pub sequence: u32,
    pub ack: Acknowledgement,
    pub payload: Vec<u8>,
    pub is_meta: bool,
    pub meta: PacketMeta,
}

impl Packet {
    /// Create a new Packet
    ///
    /// # Arguments
    ///
    /// * `id`    -   A u32 representing the id of the packet
    /// * `sequence` - A u32 representing the sequence number of the packet
    pub fn new(p_type: PType, sequence: u32) -> Packet {
        Packet {
            flags: PacketFlags {
                p_type,
                ack: false,
                enc: false,
            },
            sequence,
            ack: Acknowledgement {
                ack_begin: 0,
                ack_end: 0,
                miss_count: 0,
                miss: Vec::new(),
            },
            payload: Vec::new(),
            is_meta: false,
            meta: PacketMeta {
                delay_ms: 0,
                retry_count: 0,
            },
        }
    }

    pub fn set_meta(&mut self, meta: PacketMeta) {
        self.is_meta = true;
        self.meta = meta;
    }

    /// Add ack struct into the packet
    ///
    /// # Arguments
    ///
    /// * `ack`    -   A Acknowledgement struct
    pub fn add_ack(&mut self, ack: Acknowledgement) {
        self.ack = ack;
        self.flags.ack = true;
    }
    ///Append payload Vec<u8> to the packet
    /// also assigns the length of the packet
    ///
    /// # Arguments
    ///
    /// * `payload`    -   Vec<u8> representing the payload of the packet
    pub fn append_payload(&mut self, payload: Vec<u8>) {
        self.payload.extend(payload);
    }
    /// Compile the data in the packet into packet struct
    ///
    /// # Arguments
    ///
    /// * 'self' - The Packet struct
    pub fn compile(&self) -> Vec<u8> {
        // Vector to store final compiled packet structure
        let mut packet_vector = Vec::<u8>::new();

        // Packet ID converting u32 to u8(vector)
        // let slice_id = compile_u32(self.id);
        // packet_vector.extend(slice_id);

        // Packet Sequence converting u32 to u8(vector)
        let slice_sequence = compile_u32(self.sequence);
        packet_vector.extend(slice_sequence);

        // Packet Ack Begin converting u32 to u8(vector)
        let slice_ack_begin = compile_u32(self.ack.ack_begin);
        packet_vector.extend(slice_ack_begin);

        packet_vector.push(self.ack.ack_end);

        packet_vector.push(self.flags.get_byte());

        packet_vector.push(self.ack.miss_count);

        let slice_miss = self.ack.miss.clone();
        packet_vector.extend(slice_miss);

        let slice_payload = self.payload.clone();
        packet_vector.extend(slice_payload);

        // currently the packet_vector is a vector of u8 but we have to convert into string and then into bytes
        packet_vector
    }
}
impl From<u8> for PacketFlags {
    fn from(byte: u8) -> Self {
        let mut flags = PacketFlags {
            p_type: PType::Data,
            ack: false,
            enc: false,
        };
        flags.p_type = PType::from((byte >> 4) & 0x0F);
        if (byte >> 3) & 0x01 == 1 {
            flags.ack = true;
        }
        if (byte >> 2) & 0x01 == 1 {
            flags.enc = true;
        }
        flags
    }
}

impl From<Vec<u8>> for Packet {
    // Create a packet structure from the received raw bytes
    // # Arguments
    // *bytes - A vector of u8 representing the raw bytes of the packet
    fn from(bytes: Vec<u8>) -> Packet {
        let mut packet_default = Packet {
            flags: PacketFlags {
                p_type: PType::Data,
                ack: false,
                enc: false,
            },
            sequence: 0,
            ack: Acknowledgement {
                ack_begin: 0,
                ack_end: 0,
                miss_count: 0,
                miss: Vec::new(),
            },
            payload: Vec::new(),
            is_meta: false,
            meta: PacketMeta {
                delay_ms: 0,
                retry_count: 0,
            },
        };

        // Packet ID converting u8 to u32(vector)
        // let id_array = bytes[0..4].try_into().unwrap();
        // packet_default.id = u32::from_be_bytes(id_array);

        // Packet Sequence converting u8 to u32(vector)
        let sequence_array = bytes[0..4].try_into().unwrap();
        packet_default.sequence = u32::from_be_bytes(sequence_array);

        // Packet Ack Begin converting u8 to u32(vector)
        let ack_begin_array = bytes[4..8].try_into().unwrap();
        packet_default.ack.ack_begin = u32::from_be_bytes(ack_begin_array);

        packet_default.ack.ack_end = bytes[8];

        packet_default.flags = PacketFlags::from(bytes[9]);

        packet_default.ack.miss_count = bytes[10];

        packet_default.ack.miss = bytes[11..11 + packet_default.ack.miss_count as usize].to_vec();

        let payload_start = 11 + packet_default.ack.miss_count as usize;
        let payload_length = bytes.len() - payload_start;
        // Packet Length converting u8 to u16(vector)
        // let length_array = bytes[11 + packet_default.ack.miss_count as usize
        //     ..13 + packet_default.ack.miss_count as usize]
        //     .try_into()
        //     .unwrap();
        // packet_default.length = u16::from_be_bytes(length_array);

        packet_default.payload = bytes[payload_start..payload_start + payload_length].to_vec();

        packet_default
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::PType;
    use crate::{acknowledgement::AcknowledgementList, packet};

    #[test]
    fn range_test() {
        let pack = packet::Packet::new(PType::Data, 0);
        assert!(pack.ack.ack_begin <= pack.ack.ack_end.into());
        assert!(pack.ack.miss_count as u32 <= (pack.ack.ack_end as u32 - pack.ack.ack_begin));
    }

    #[test]
    fn compile_test() {
        let mut pack = packet::Packet::new(PType::Data, 32850943);
        let mut ack_list = AcknowledgementList::new(329965);
        ack_list.insert(329966);
        ack_list.insert(329967);
        ack_list.insert(329969);
        ack_list.insert(329970);

        pack.add_ack(ack_list.get());
        pack.append_payload(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let compiled = pack.compile();

        let pack_out = packet::Packet::from(compiled);

        assert_eq!(pack.sequence, pack_out.sequence);

        assert_eq!(pack.flags.p_type, pack_out.flags.p_type);
        assert_eq!(pack.flags.ack, pack_out.flags.ack);
        assert_eq!(pack.flags.enc, pack_out.flags.enc);

        assert_eq!(pack.ack.ack_begin, pack_out.ack.ack_begin);
        assert_eq!(pack.ack.ack_end, pack_out.ack.ack_end);
        assert_eq!(pack.ack.miss_count, pack_out.ack.miss_count);
        assert_eq!(pack.ack.miss, pack_out.ack.miss);

        assert_eq!(pack.payload, pack_out.payload);
    }
}
