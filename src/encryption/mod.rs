//! Primitives for symmetric encryption for Aether.
//! Makes use of AES-256-GCM cipher. Implementation built on top of OpenSSL.

use std::fmt::{Debug, Formatter};

use openssl::{
    sha::sha256,
    symm::{decrypt_aead, encrypt_aead, Cipher},
};

use crate::{error::AetherError, util::gen_nonce};

const EMPTY_BYTES: [u8; 0] = [];
pub const IV_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const TAG_SIZE: usize = 16;

#[derive(Clone)]
pub struct AetherCipher {
    cipher: Cipher,
    key: [u8; KEY_SIZE],
}

pub struct Encrypted {
    pub cipher_text: Vec<u8>,
    pub tag: Vec<u8>,
    pub iv: Vec<u8>,
    pub aad: Vec<u8>,
}

impl AetherCipher {
    pub fn new(shared_secret: Vec<u8>) -> AetherCipher {
        let cipher = Cipher::aes_256_gcm();
        let key = sha256(&shared_secret);

        AetherCipher { cipher, key }
    }

    pub fn encrypt_bytes(&self, plain_text: Vec<u8>) -> Result<Encrypted, AetherError> {
        let mut tag = vec![0u8; TAG_SIZE];
        let iv = gen_nonce(IV_SIZE);
        let encrypted = encrypt_aead(
            self.cipher,
            &self.key,
            Some(&iv),
            &EMPTY_BYTES,
            &plain_text,
            &mut tag,
        )?;

        Ok(Encrypted {
            cipher_text: encrypted,
            tag,
            iv,
            aad: EMPTY_BYTES.to_vec(),
        })
    }

    pub fn decrypt_bytes(&self, cipher_text: Encrypted) -> Result<Vec<u8>, AetherError> {
        Ok(decrypt_aead(
            self.cipher,
            &self.key,
            Some(&cipher_text.iv),
            &cipher_text.aad,
            &cipher_text.cipher_text,
            &cipher_text.tag,
        )?)
    }
}

impl From<Encrypted> for Vec<u8> {
    fn from(mut encrypted: Encrypted) -> Self {
        let mut result: Vec<u8> = Vec::new();
        result.append(&mut encrypted.aad);
        result.append(&mut encrypted.tag);
        result.append(&mut encrypted.iv);
        result.append(&mut encrypted.cipher_text);
        result
    }
}

impl From<Vec<u8>> for Encrypted {
    fn from(mut bytes: Vec<u8>) -> Self {
        Encrypted {
            aad: EMPTY_BYTES.to_vec(),
            tag: bytes.drain(0..TAG_SIZE).collect(),
            iv: bytes.drain(0..IV_SIZE).collect(),
            cipher_text: bytes,
        }
    }
}

impl Debug for AetherCipher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AetherCipher")
            .field("cipher", &"AES-256-GCM")
            .field("key", &base64::encode(self.key))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        encryption::{Encrypted, KEY_SIZE},
        util::gen_nonce,
    };

    use super::AetherCipher;

    #[test]
    fn encryption_test() {
        let data = gen_nonce(512);

        let cipher = AetherCipher::new(gen_nonce(KEY_SIZE));

        let encrypted = cipher.encrypt_bytes(data.clone()).unwrap();

        let decrypted = cipher.decrypt_bytes(encrypted).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn encoding_test() {
        let data = gen_nonce(512);

        let cipher = AetherCipher::new(gen_nonce(KEY_SIZE));

        let encrypted = cipher.encrypt_bytes(data.clone()).unwrap();

        // Encrypted data is converted to sequence of bytes and sent
        let encrypted_raw: Vec<u8> = Vec::from(encrypted);

        // Other end receives sequence of bytes as encrypted text
        let received = Encrypted::from(encrypted_raw);

        let decrypted = cipher.decrypt_bytes(received).unwrap();

        assert_eq!(data, decrypted);
    }
}
