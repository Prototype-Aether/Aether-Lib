use serde::{Deserialize, Serialize};
use std::default::Default;

/// Structure to represent configuration options for `aether_lib`
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Config {
    pub aether: AetherConfig,
}

/// Structure to represent configuration for [`Aether`][crate::peer::Aether]
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct AetherConfig {
    /// Duration to wait for Tracker server to respond (in ms)
    pub server_retry_delay: u64,
    /// How often to poll server for new connections
    pub server_poll_time: u64,
    /// How long to wait to retry handshake after a failed attempt
    /// Also used as duration to wait to receive nonce from other peer during
    /// authentication
    pub handshake_retry_delay: u64,
    /// Poll time to check if connection has been established
    pub connection_check_delay: u64,
    /// Magnitude by which to randomize retry delay
    pub delta_time: u64,
    /// General poll time to be used to check for updates to lists shared by threads
    /// (in us)
    pub poll_time_us: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aether: Default::default(),
        }
    }
}

impl Default for AetherConfig {
    fn default() -> Self {
        Self {
            server_retry_delay: 1000,
            server_poll_time: 1000,
            handshake_retry_delay: 5000,
            connection_check_delay: 1000,
            delta_time: 100,
            poll_time_us: 100,
        }
    }
}
