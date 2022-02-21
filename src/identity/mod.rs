use std::{fs, path::PathBuf};

use openssl::{
    pkey::Private,
    rsa::{Padding, Rsa},
};

use crate::error::AetherError;
use home::home_dir;

const RSA_SIZE: u32 = 1024;

pub struct Id {
    rsa: Rsa<Private>,
}

impl Id {
    pub fn new() -> Result<Id, AetherError> {
        Ok(Id {
            rsa: Rsa::generate(RSA_SIZE)?,
        })
    }

    pub fn get_private_key_path() -> PathBuf {
        match home_dir() {
            Some(mut home) => {
                home.push(".config/aether/private_key.pem");
                home
            }
            None => PathBuf::from("./private_key.pem"),
        }
    }

    pub fn get_public_key_path() -> PathBuf {
        match home_dir() {
            Some(mut home) => {
                home.push(".config/aether/public_key.pem");
                home
            }
            None => PathBuf::from("./public_key.pem"),
        }
    }

    pub fn save(&self) -> Result<(), AetherError> {
        let rsa_public = self.rsa.public_key_to_pem()?;
        let rsa_private = self.rsa.private_key_to_pem()?;

        if let Err(err) = fs::write(Self::get_private_key_path(), rsa_private) {
            Err(AetherError::FileWrite(err))
        } else if let Err(err) = fs::write(Self::get_public_key_path(), rsa_public) {
            Err(AetherError::FileWrite(err))
        } else {
            Ok(())
        }
    }

    pub fn load() -> Result<Id, AetherError> {
        let private_pem = match fs::read(Self::get_private_key_path()) {
            Ok(data) => data,
            Err(err) => return Err(AetherError::FileRead(err)),
        };

        let rsa = Rsa::private_key_from_pem(&private_pem)?;

        Ok(Id { rsa })
    }

    pub fn load_or_generate() -> Result<Id, AetherError> {
        match Self::load() {
            Ok(id) => Ok(id),
            Err(AetherError::FileRead(err)) => {
                println!("Error reading key: {}", err);
                let new_id = Self::new()?;
                match new_id.save() {
                    Ok(()) => Ok(new_id),
                    Err(err) => Err(err),
                }
            }
            Err(err) => Err(err),
        }
    }

    pub fn public_key_to_base64(&self) -> Result<String, AetherError> {
        let public_key_der = self.rsa.public_key_to_der()?;
        Ok(base64::encode(public_key_der))
    }

    pub fn private_key_to_base64(&self) -> Result<String, AetherError> {
        let private_key_der = self.rsa.private_key_to_der()?;
        Ok(base64::encode(private_key_der))
    }

    pub fn encrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        self.rsa.public_encrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf.to_vec())
    }

    pub fn decrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        let size = self.rsa.private_decrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf[..size].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::Id;

    #[test]
    fn save_test() {
        let id = Id::new().unwrap();
        id.save().unwrap();
        let id_new = Id::load().unwrap();
        assert_eq!(
            id.public_key_to_base64().unwrap(),
            id_new.public_key_to_base64().unwrap()
        );
        assert_eq!(
            id.private_key_to_base64().unwrap(),
            id_new.private_key_to_base64().unwrap()
        );
    }

    #[test]
    fn encrypt_test() {
        let message = String::from("This is a small message");
        let message_bytes = message.as_bytes();
        let id = Id::new().unwrap();
        let message_encrypted = id.encrypt(message_bytes).unwrap();
        let message_decrypted = id.decrypt(&message_encrypted).unwrap();
        let message_out = String::from_utf8(message_decrypted).unwrap();

        assert_eq!(message, message_out);
    }
}
