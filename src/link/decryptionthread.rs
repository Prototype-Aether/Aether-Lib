use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crossbeam::channel::{Receiver, RecvTimeoutError, Sender};

use crate::{config::Config, encryption::AetherCipher, error::AetherError, packet::Packet};

pub struct DecryptionThread {
    cipher: AetherCipher,
    receiver: Receiver<Packet>,
    sender: Sender<Packet>,
    stop_flag: Arc<Mutex<bool>>,
    config: Config,
}

impl DecryptionThread {
    pub fn new(
        cipher: AetherCipher,
        receiver: Receiver<Packet>,
        sender: Sender<Packet>,
        stop_flag: Arc<Mutex<bool>>,
        config: Config,
    ) -> DecryptionThread {
        DecryptionThread {
            cipher,
            receiver,
            sender,
            stop_flag,
            config,
        }
    }
    pub fn start(&self) -> Result<(), AetherError> {
        loop {
            match self
                .receiver
                .recv_timeout(Duration::from_micros(self.config.link.poll_time_us))
            {
                Ok(mut packet) => {
                    let encrypted = packet.payload;
                    let decrypted = self.cipher.decrypt_bytes(encrypted.into())?;
                    packet.payload = decrypted;
                    packet.set_enc(false);
                    self.sender.send(packet)?;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(err) => {
                    return Err(AetherError::from(err));
                }
            };

            let flag_lock = self.stop_flag.lock().expect("Error locking stop flag");
            if *flag_lock {
                break;
            }
        }

        Ok(())
    }
}
