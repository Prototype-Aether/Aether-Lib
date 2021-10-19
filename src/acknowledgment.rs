use std::collections::HashMap;

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
            Some(_) => true,
        }
    }
}

struct Acknowledment {
    ack: u32,
    ttl: u32,
}

// Acknowledgments to be sent
pub struct AcknowledgmentList {
    miss: Vec<u8>,
    sequence: u32,
    ack_end: u8,
}

impl AcknowledgmentList {
    pub fn new(sequence: u32) -> AcknowledgmentList {
        AcknowledgmentList {
            miss: Vec::new(),
            sequence,
            ack_end: 0,
        }
    }

    pub fn check(&self, ack: u32) {}

    pub fn insert(&mut self, ack: u32) {
        if ack < self.sequence {
            panic!("ack too old");
        }

        if ack > (0xff + self.sequence) {
            panic!("ack too large");
        }

        let n = (ack - self.sequence) as u8;

        if n > self.ack_end {
            let mut new_miss: Vec<u8> = ((self.ack_end + 1)..n).collect();

            self.miss.append(&mut new_miss);

            self.ack_end = n as u8;
        }

        if n < self.ack_end {
            let index = self.miss.iter().position(|&r| r == n).unwrap();
            self.miss.swap_remove(index);
        }
    }

    pub fn get(&mut self) -> (u32, u8, u8, Vec<u8>) {
        (
            self.sequence,
            self.ack_end,
            self.miss.len() as u8,
            self.miss.clone(),
        )
    }

    pub fn is_complete(&self) -> bool {
        self.miss.len() == 0
    }
}
