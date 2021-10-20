use std::collections::HashMap;

pub struct Acknowledgment {
    pub ack_begin: u32,
    pub ack_end: u8,
    pub miss_count: u8,
    pub miss: Vec<u8>,
}

// Acknowledgments received
pub struct AcknowledgmentCheck {
    begin: u32,
    list: HashMap<u32, bool>,
}

impl AcknowledgmentCheck {
    pub fn new(begin: u32) -> AcknowledgmentCheck {
        AcknowledgmentCheck {
            begin,
            list: HashMap::new(),
        }
    }

    fn update_begin(&mut self) {
        while self.check(&(self.begin + 1)) {
            self.list.remove(&self.begin);
            self.begin += 1;
        }
    }

    pub fn insert(&mut self, ack: u32) {
        if ack > self.begin {
            self.list.insert(ack, true);
        }
        self.update_begin();
    }

    pub fn check(&self, ack: &u32) -> bool {
        if *ack <= self.begin {
            return true;
        }

        match self.list.get(ack) {
            None => false,
            Some(v) => *v,
        }
    }
}

// Acknowledgments to be sent
pub struct AcknowledgmentList {
    list: HashMap<u8, bool>,
    ack_begin: u32,
    ack_end: u8,
}

impl AcknowledgmentList {
    pub fn new(ack_begin: u32) -> AcknowledgmentList {
        let mut list: HashMap<u8, bool> = HashMap::new();
        list.insert(0, true);
        AcknowledgmentList {
            list,
            ack_begin,
            ack_end: 0,
        }
    }

    pub fn check(&self, ack: &u32) -> bool {
        if *ack == self.ack_begin {
            return true;
        } else if self.ack_begin <= *ack && *ack <= (self.ack_begin + self.ack_end as u32) {
            let n = (*ack - self.ack_begin) as u8;
            return match self.list.get(&n) {
                None => false,
                Some(v) => *v,
            };
        } else {
            return false;
        }
    }

    pub fn insert(&mut self, ack: u32) {
        if ack < self.ack_begin {
            panic!("ack too old");
        }

        if ack > (0xff + self.ack_begin) {
            panic!("ack too large");
        }

        let n = (ack - self.ack_begin) as u8;

        if n > self.ack_end {
            self.ack_end = n as u8;
        }

        self.list.insert(n, true);
    }

    pub fn get(&self) -> Acknowledgment {
        let mut miss: Vec<u8> = Vec::new();

        for i in 0..(self.ack_end + 1) {
            match self.list.get(&i) {
                None => miss.push(i),
                Some(false) => miss.push(i),
                Some(true) => (),
            }
        }

        Acknowledgment {
            ack_begin: self.ack_begin,
            ack_end: self.ack_end,
            miss_count: miss.len() as u8,
            miss,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.get().miss_count == 0
    }
}

#[cfg(test)]
mod tests {
    mod ack_check {
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

    mod ack_list {
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
}
