use std::collections::HashMap;

/// Structure to reperesent the acknowledgment format
#[derive(Debug)]
pub struct Acknowledgment {
    /// The sequence number of the packet from which the acknowledgment begins
    pub ack_begin: u32,

    /// The number of packets that this acknowledgment includes. ACK number of
    /// the last packet to be acknowledged relative to the `ack_begin`
    /// > Note: If the sequence number of a packet is `ack`, the relative sequence
    ///   number to `ack_begin` would be `ack - ack_begin`.
    pub ack_end: u8,

    /// Number of packets from `ack_begin` till `ack_begin + ack_end` that are
    /// not acknowledged
    pub miss_count: u8,

    /// Vector of ack numbers (relative to `ack_begin`) which are missing.
    /// Length of the vector is `miss_count`.
    pub miss: Vec<u8>,
}

impl Clone for Acknowledgment {
    fn clone(&self) -> Acknowledgment {
        Acknowledgment {
            ack_begin: self.ack_begin,
            ack_end: self.ack_end,
            miss_count: self.miss_count,
            miss: self.miss.clone(),
        }
    }
}

pub const MAX_WINDOW: u8 = 3;

/// A checklist to store all acknowledgments received.
/// * Used by sending module to test if a packet has already been acknowledged
///   before sending it.
/// * Used by receiving module to add acknowledgments that have been received
#[derive(Debug)]
pub struct AcknowledgmentCheck {
    /// The sequence number of begining of the list. All sequence numbers below
    /// this have been acknowledged already.
    begin: u32,

    /// A HashMap to determine what all numbers have been acknowledged that are
    /// greater than `begin`
    list: HashMap<u32, bool>,
}

impl AcknowledgmentCheck {
    /// Create a new instance of [`AcknowledgmentCheck`] list
    ///
    /// # Arguments
    ///
    /// * `begin`   -   Initial value of begin sequence number
    pub fn new(begin: u32) -> AcknowledgmentCheck {
        AcknowledgmentCheck {
            begin,
            list: HashMap::new(),
        }
    }

    /// Update value of begin if consequitive values in `list` after begin have
    /// been acknowledged.
    /// This helps keep `check()` more efficient
    fn update_begin(&mut self) {
        while self.check(&(self.begin + 1)) {
            self.list.remove(&(self.begin + 1));
            self.begin += 1;
        }
    }

    /// Add acknowledgment to the list based on the [`Acknowledgment`] recevied
    ///
    /// # Arguments
    ///
    /// * `ack` -   The acknowledgment which is instance of [`Acknowledgment`].
    ///             This will be obtained from the [`Packet`][crate::packet::Packet] received.
    pub fn acknowledge(&mut self, ack: Acknowledgment) {
        // acknowledge everythin below ack.ack_begin
        if self.begin < ack.ack_begin {
            for i in self.begin..(ack.ack_begin + 1) {
                self.insert(i);
            }
        }

        let mut missing: HashMap<u8, bool> = HashMap::new();

        for i in ack.miss {
            missing.insert(i, true);
        }

        for i in 0..(ack.ack_end + 1) {
            match missing.get(&i) {
                None => self.insert(i as u32 + ack.ack_begin),
                Some(false) => self.insert(i as u32 + ack.ack_begin),
                Some(true) => (),
            }
        }
    }

    /// Insert a specific acknowledgment number into the list
    ///
    /// # Arguments
    ///
    /// * `ack` -   The acknowledgment number that was received from the other
    ///             peer
    pub fn insert(&mut self, ack: u32) {
        if ack > self.begin {
            self.list.insert(ack, true);
        }
        self.update_begin();
    }

    /// Check if the packet with the given sequence number has been acknowledged
    ///
    /// # Arguments
    ///
    /// * `ack` -   The sequence number which needs to be matched and check if
    ///             it is present in the list (acknowledged).
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

/// A structure to store the acknowledgments that need to be sent.
/// * Used by receiving module to add acknowledgments for the packets that are received
/// * Used by sending module to get acknowledgments to be sent with the next packet
pub struct AcknowledgmentList {
    /// A `HashMap` to store the sequence numbers of packets from `ack_begin` to
    /// `ack_begin + ack_end` that have been received and need to be acknowledged
    list: HashMap<u32, bool>,

    /// The sequence number of the first packet included in this acknowledgment
    ack_begin: u32,

    /// The sequence number (relative to `ack_begin`) of the last packet in this
    /// acknowledgment.
    /// > Note: If the sequence number of a packet is `ack`, the relative sequence
    /// number to `ack_begin` would be `ack - ack_begin`.
    ack_end: u8,
}

impl AcknowledgmentList {
    /// Creates a new instance of [`AcknowledgmentList`]
    ///
    /// # Arguments
    ///
    /// * `ack_begin`   -   The `ack_begin` value from which this acknowledgment
    ///                     begins
    pub fn new(ack_begin: u32) -> AcknowledgmentList {
        let mut list: HashMap<u32, bool> = HashMap::new();
        list.insert(ack_begin, true);
        AcknowledgmentList {
            list,
            ack_begin,
            ack_end: 0,
        }
    }

    /// Check if the given sequence number has been added to the list
    ///
    /// # Arguments
    ///
    /// * `ack` -   The sequence number of the packet to check
    pub fn check(&self, ack: &u32) -> bool {
        if *ack <= self.ack_begin {
            true
        } else if self.ack_begin < *ack && *ack <= (self.ack_begin + self.ack_end as u32) {
            match self.list.get(ack) {
                None => false,
                Some(v) => *v,
            }
        } else {
            false
        }
    }

    /// Insert a sequence number into the acknowledgment list
    ///
    /// # Arguments
    ///
    /// * `ack` -   Sequence number of the packet to be added to the acknowledgment
    ///             list
    pub fn insert(&mut self, ack: u32) {
        if ack > (MAX_WINDOW as u32 + self.ack_begin) {
            panic!("ack too large {}\t Diff: {}", ack, ack - self.ack_begin);
        } else if ack > self.ack_begin {
            let n = (ack - self.ack_begin) as u8;

            if n > self.ack_end {
                self.ack_end = n;
            }

            self.list.insert(ack, true);
            self.update_begin();
        }
    }

    /// Update value of begin if consequitive values in `list` after begin have
    /// been acknowledged.
    /// This helps keep `check()` more efficient
    fn update_begin(&mut self) {
        while self.check(&(self.ack_begin + 1)) {
            self.list.remove(&(self.ack_begin + 1));
            self.ack_begin += 1;
            self.ack_end -= 1;
        }
    }

    /// Get an [`Acknowledgment`] structure out of this [`AcknowledgmentList`]
    /// * Used to add the acknowledgment to the next outgoing packet
    pub fn get(&self) -> Acknowledgment {
        let mut miss: Vec<u8> = Vec::new();

        for i in 1..(self.ack_end + 1) {
            match self.list.get(&(i as u32 + self.ack_begin)) {
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

    /// Check if the [`AcknowledgmentList`] is complete. The list is complete when
    /// there are not missing packets between `ack_begin` to `ack_begin + ack_end`.
    /// Thus, all packets within that window have been acknowledged
    pub fn is_complete(&self) -> bool {
        self.get().miss_count == 0
    }
}

#[cfg(test)]
mod tests {
    mod ack_check {
        use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
        #[test]
        fn false_positive_raw() {
            let values = [16, 1024, 99, 45];

            let check = [19, 32, 63, 6000];

            let mut ack_check = AcknowledgmentCheck::new(16);

            for v in values {
                ack_check.insert(v);
            }

            for c in check {
                assert!(!ack_check.check(&c));
            }
        }

        #[test]
        fn true_negatives_raw() {
            let values = [16, 1024, 99, 45];

            let mut ack_check = AcknowledgmentCheck::new(16);

            for v in values {
                ack_check.insert(v);
            }

            for c in values {
                assert!(ack_check.check(&c));
            }
        }

        #[test]
        fn false_positives() {
            let values = [16, 20, 17, 18, 22, 23];

            let check = [19, 21, 63];

            let mut ack_list = AcknowledgmentList::new(16);

            for v in values {
                ack_list.insert(v);
            }

            let mut ack_check = AcknowledgmentCheck::new(16);

            let ack = ack_list.get();

            ack_check.acknowledge(ack);
            for c in check {
                assert!(!ack_check.check(&c));
            }
        }

        #[test]
        fn true_negatives() {
            let values = [16, 17, 18, 20, 21, 22, 32];

            let mut ack_list = AcknowledgmentList::new(16);

            for v in values {
                ack_list.insert(v);
            }

            let mut ack_check = AcknowledgmentCheck::new(16);

            let ack = ack_list.get();

            ack_check.acknowledge(ack);
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
            println!("\n\nIs complete test\n\n");
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
