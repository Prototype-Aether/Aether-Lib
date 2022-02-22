//! Primitives for representing PKC based user identities. Used to identify and authenticate users
//! as well as for key exchange.
//!
//! Current implementation uses RSA as the asymmetric encryption algorithm. But can be replaced in
//! the future in favor of more efficient algorithms.
//!
//! # Identity Storage
//!
//! The [`Id`] is stored in `$HOME/.config/aether/` by default. If `$HOME` cannot be resolved, the
//! current working directory is used instead.
//!
//! # OpenSSL Errors
//!
//! This library uses the [OpenSSL wrapper](https://crates.io/crates/openssl) for encryption
//! purposes. So, some of the functions can return [`AetherError::OpenSSLError`].
//! Check [`openssl::error::ErrorStack`] for detailed description of OpenSSL errors.
//!
//! Refer: [https://www.openssl.org/](https://www.openssl.org/)
//!
//! # Examples
//!
//! To load a new identity from the filesystem or create a new identity if not found use
//! `load_or_generate()`
//!
//! ```
//! use aether_lib::identity::Id;
//!
//! let id = Id::load_or_generate().unwrap();
//! let plain_text = "A message to be encrypted";
//! // Returns a Vec<u8> of cipher text bytes
//! let cipher_text_bytes = id.public_encrypt(&plain_text.as_bytes()).unwrap();
//! // Returns a Vec<u8> of decrypted bytes
//! let decrypted_text_bytes = id.private_decrypt(&cipher_text_bytes).unwrap();
//!
//! let plain_text_decrypted = String::from_utf8(decrypted_text_bytes).unwrap();
//!
//! assert_eq!(plain_text, plain_text_decrypted);
//! ```
//!
//! To generate a new identity use `new()`
//!
//! ```
//! use aether_lib::identity::Id;
//!
//! let id = Id::new().unwrap();
//! ```
use std::{fs, path::PathBuf};

use openssl::{
    pkey::{Private, Public},
    rsa::{Padding, Rsa},
};

use crate::error::AetherError;
use home::home_dir;

/// Size of RSA keys to be used
pub const RSA_SIZE: u32 = 1024;

/// Primitive to represent and store the identity of a user. Used by a user to store their own
/// identity.
/// Uses asymmetric encryption as the basis for authentication.
#[derive(Debug, Clone)]
pub struct Id {
    /// RSA Private key defining the user
    rsa: Rsa<Private>,
}

/// Primitive to represent public identity of a user. Used by a user to store other users'
/// identities
/// Different from `Id` as it is meant to be used to store only public key. So, only used to
/// represent identity of other users
pub struct PublicId {
    /// RSA public key defining the user
    rsa: Rsa<Public>,
}

impl Id {
    /// Generate a new identity
    /// # Errors
    /// * [`AetherError::OpenSSLError`]   -   If the RSA key pair could not be generated
    pub fn new() -> Result<Id, AetherError> {
        Ok(Id {
            rsa: Rsa::generate(RSA_SIZE)?,
        })
    }

    /// Returns [`PathBuf`] to the private key on the filesystem
    pub fn get_private_key_path() -> PathBuf {
        let mut config = Self::get_config_dir();
        config.push("private_key.pem");
        config
    }

    /// Returns [`PathBuf`] to the public key on the filesystem
    pub fn get_public_key_path() -> PathBuf {
        let mut config = Self::get_config_dir();
        config.push("public_key.pem");
        config
    }

    /// Returns [`PathBuf`] to the config directory on the filesystem
    fn get_config_dir() -> PathBuf {
        match home_dir() {
            Some(mut home) => {
                home.push(".config/aether/");
                match fs::create_dir_all(home.clone()) {
                    Ok(()) => home,
                    Err(_) => PathBuf::from("./"),
                }
            }
            None => PathBuf::from("./"),
        }
    }

    /// Save the current identity on the filesystem
    /// Saves the public key and the private key in PEM format
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

    /// Load an identity from the default location on the filesystem
    /// Reads the private key from the default location
    pub fn load() -> Result<Id, AetherError> {
        let private_pem = match fs::read(Self::get_private_key_path()) {
            Ok(data) => data,
            Err(err) => return Err(AetherError::FileRead(err)),
        };

        let rsa = Rsa::private_key_from_pem(&private_pem)?;

        Ok(Id { rsa })
    }

    /// Try to load the identity from the default location on the filesystem or create a new
    /// identity. If a new identity is created, it is stored in the default location
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

    /// Convert public key to a base64 encoded string
    /// Encodes public key as DER and then encodes DER into base64
    pub fn public_key_to_base64(&self) -> Result<String, AetherError> {
        let public_key_der = self.rsa.public_key_to_der()?;
        Ok(base64::encode(public_key_der))
    }

    /// Convert private key to a base64 encoded string
    /// Encodes private key as DER and then encodes DER into base64
    pub fn private_key_to_base64(&self) -> Result<String, AetherError> {
        let private_key_der = self.rsa.private_key_to_der()?;
        Ok(base64::encode(private_key_der))
    }

    /// Encrypt given bytes using the public key
    pub fn public_encrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        self.rsa.public_encrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf.to_vec())
    }

    /// Encrypt given bytes using the private key
    pub fn private_encrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        self.rsa.private_encrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf.to_vec())
    }

    /// Decrypt given bytes using the public key
    pub fn public_decrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        let size = self.rsa.public_decrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf[..size].to_vec())
    }

    /// Decrypt given bytes using the private key
    pub fn private_decrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        let size = self.rsa.private_decrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf[..size].to_vec())
    }
}

impl PublicId {
    /// Decode the given base64 string into a [`PublicId`]
    /// # Errors
    /// * [`AetherError::Base64DecodeError`]    -   If the given string is not valid base64
    pub fn from_base64(key: &str) -> Result<PublicId, AetherError> {
        let bytes = base64::decode(key)?;
        let rsa = Rsa::public_key_from_der(&bytes)?;
        Ok(Self { rsa })
    }

    /// Convert public key to a base64 encoded string
    /// Encodes public key as DER and then encodes DER into base64
    pub fn public_key_to_base64(&self) -> Result<String, AetherError> {
        let public_key_der = self.rsa.public_key_to_der()?;
        Ok(base64::encode(public_key_der))
    }

    /// Encrypt given bytes using the public key
    pub fn public_encrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        self.rsa.public_encrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf.to_vec())
    }

    /// Decrypt given bytes using the public key
    pub fn public_decrypt(&self, from: &[u8]) -> Result<Vec<u8>, AetherError> {
        let mut buf: Vec<u8> = vec![0; self.rsa.size() as usize];
        let size = self.rsa.public_decrypt(from, &mut buf, Padding::PKCS1)?;
        Ok(buf[..size].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::util::gen_nonce;

    use super::{Id, PublicId};

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
        let message_encrypted = id.public_encrypt(message_bytes).unwrap();
        let message_decrypted = id.private_decrypt(&message_encrypted).unwrap();
        let message_out = String::from_utf8(message_decrypted).unwrap();

        assert_eq!(message, message_out);
    }

    #[test]
    fn signature_test() {
        let alice_id = Id::new().unwrap();
        let alice_public =
            PublicId::from_base64(&alice_id.public_key_to_base64().unwrap()).unwrap();

        let alice_message = "A message to be signed";
        let alice_message_signed = alice_id.private_encrypt(alice_message.as_bytes()).unwrap();

        let bob_decrypted_bytes = alice_public.public_decrypt(&alice_message_signed).unwrap();

        let bob_message = String::from_utf8(bob_decrypted_bytes).unwrap();

        assert_eq!(alice_message, bob_message);
    }

    #[test]
    fn authentication_test() {
        let alice_id = Id::new().unwrap();
        // Alice publishes her public key
        let alice_public =
            PublicId::from_base64(&alice_id.public_key_to_base64().unwrap()).unwrap();

        // bob generates a random 256 bit number
        let bob_nonce = gen_nonce(32);

        // bob encrypts nonce with alice's public key and sends to alice
        let bob_challenge = alice_public.public_encrypt(&bob_nonce).unwrap();

        // alice decrypts the nonce with her private key and sends to bob
        let alice_response = alice_id.private_decrypt(&bob_challenge).unwrap();

        println!(
            "{} == {}",
            base64::encode(bob_nonce.clone()),
            base64::encode(alice_response.clone())
        );
        // if bob receives the same random nonce, alice owns the private key corresponding to the
        // public key
        assert_eq!(bob_nonce, alice_response);
    }
}
