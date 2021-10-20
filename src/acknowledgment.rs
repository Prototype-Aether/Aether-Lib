use std::collections::HashMap;

pub struct Acknowledment {
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

    pub fn get(&self) -> Acknowledment {
        let mut miss: Vec<u8> = Vec::new();

        for i in 0..(self.ack_end + 1) {
            match self.list.get(&i) {
                None => miss.push(i),
                Some(false) => miss.push(i),
                Some(true) => (),
            }
        }

        Acknowledment {
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
