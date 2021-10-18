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

// Acknowledgments to be sent
pub struct AcknowledgmentList {}
