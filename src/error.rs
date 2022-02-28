//! Structures to represent errors in `aether_lib`
use openssl::error::ErrorStack;
use std::fmt::Debug;
use std::string::FromUtf8Error;
use std::time::SystemTimeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AetherError {
    #[error("Current time is from future so cannot calculate elapsed time")]
    ElapsedTime(#[from] SystemTimeError),
    #[error("Failed to lock a mutex")]
    MutexLock(&'static str),
    #[error("Link module stopped")]
    LinkStopped(&'static str),
    #[error("Receive timed out")]
    RecvTimeout,
    #[error("Link timed out")]
    LinkTimeout,
    #[error("Failed to set read timeout on socket")]
    SetReadTimeout,
    #[error("User not connected")]
    NotConnected(String),
    #[error("Error parsing yaml string")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("Error reading file")]
    FileRead(std::io::Error),
    #[error("Error writing file")]
    FileWrite(std::io::Error),
    #[error("Other peer cannot be authenticated")]
    AuthenticationInvalid(String),
    #[error("Other peer cannot be reached when authenticating")]
    AuthenticationFailed(String),
    #[error("OpenSSL Error")]
    OpenSSLError(#[from] ErrorStack),
    #[error("Error parsing utf8 string")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("Error decoding base64 string")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Handshake couldn't complete")]
    HandshakeError,
}
