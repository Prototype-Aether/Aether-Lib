pub mod acknowledgment;
pub mod packet;

#[cfg(test)]
mod packet_test {
    use crate::packet;

    #[test]
    fn range_test() {
        let pack = packet::Packet::new(0, 0);
        assert!(pack.ack_begin <= pack.ack_end.into());
        assert!(pack.miss_count as u32 <= (pack.ack_end as u32 - pack.ack_begin));
    }

    #[test]
    fn compile_test() {
        let pack = packet::Packet::new(52, 32);

        let compiled = pack.compile();

        let pack_out = packet::Packet::from(compiled);

        assert!(pack.id == pack_out.id);
        assert!(pack.sequence == pack_out.sequence);
        assert!(pack.ack_begin == pack_out.ack_begin);
        assert!(pack.ack_end == pack_out.ack_end);
    }
}
