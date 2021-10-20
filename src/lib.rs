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
mod ackcheck_test {
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

#[cfg(test)]
mod acklist_test {
    use crate::acknowledgment::AcknowledgmentList;

    #[test]
    fn false_positives() {
        let sequence = 10;
        let mut ack_list = AcknowledgmentList::new(sequence);

        let values = [10, 20, 30, 40];

        let check = [12, 15, 320, 44, 39];

        for v in values {
            ack_list.insert(v);
        }

        for c in check {
            assert!(!ack_list.check(&c));
        }
    }

    #[test]
    fn true_negatives() {
        let sequence = 10;
        let mut ack_list = AcknowledgmentList::new(sequence);

        let values = [10, 20, 30, 40];

        for v in values {
            ack_list.insert(v);
        }

        for c in values {
            assert!(ack_list.check(&c));
        }
    }

    #[test]
    fn missing_test() {
        let sequence = 10;
        let mut ack_list = AcknowledgmentList::new(sequence);

        let misses = [11, 14, 22, 28];

        for v in sequence..(sequence + 20) {
            if !misses.contains(&v) {
                ack_list.insert(v);
            }
        }

        let ack = ack_list.get();

        for m in ack.miss {
            assert!(misses.contains(&(m as u32 + sequence)));
        }
    }

    #[test]
    fn check_complete_test() {
        let sequence = 10;
        let mut ack_list = AcknowledgmentList::new(sequence);

        let values = sequence..(sequence + 20);

        for v in values {
            ack_list.insert(v);
        }

        assert!(ack_list.is_complete());
    }
}
