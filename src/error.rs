//! Structures to represent errors in `aether_lib`
use std::fmt::Debug;
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
    FileRead(#[from] std::io::Error),
}
