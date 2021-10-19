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

#[cfg(test)]
mod ack_test {
    use crate::acknowledgment::AcknowledgmentCheck;

    #[test]
    fn false_positive() {
        let values = [16, 1024, 99, 45];

        let check = [19, 32, 63, 6000];

        let mut ack_check = AcknowledgmentCheck::new(0);

        for v in values {
            ack_check.insert(v);
        }

        for c in check {
            assert!(!ack_check.check(&c));
        }
    }

    #[test]
    fn true_negatives() {
        let values = [16, 1024, 99, 45];

        let mut ack_check = AcknowledgmentCheck::new(0);

        for v in values {
            ack_check.insert(v);
        }

        for c in values {
            assert!(ack_check.check(&c));
        }
    }
}
