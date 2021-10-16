pub struct UDPPacket {
    pub id: u32,
    pub sequence: u32,
    pub ack: u32,
    pub length: usize,
    pub payload: String
}
