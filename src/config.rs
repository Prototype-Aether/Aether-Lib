//! Structures to represent configuration used by `aether_lib`
//!
//! - All time values are in milliseconds unless specified otherwise
//! - `_US` is used as suffix for time values in microseconds
//!
//! ## Configuration file
//! The default configuration file is to be stored in `$HOME/.config/aether/config.yaml` and must
//! be in [YAML](https://yaml.org/) format
//!
//! Note that any missing values will be replaced with default values. It is not recommended to
//! leave any missing values in the configuration file as the values need to follow certain
//! constaints. For example, `handshake_timeout` cannot be smaller than `peer_poll_time` because in
//! such a case, the handshake would timeout before even a single poll is complete.
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, default::Default, fs, path::Path};

use crate::error::AetherError;

/// Structure to represent configuration options for `aether_lib`
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Config {
    /// Configuration for [`peer`][crate::peer] module
    pub aether: AetherConfig,
    /// Configuration for [`handshake`][crate::peer::handshake] module
    pub handshake: HandshakeConfig,
    /// Configuration for [`link`][crate::link] module
    pub link: LinkConfig,
}

/// Structure to represent configuration for [`peer`][crate::peer] module
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
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

/// Structure to represent configuration for [`handshake`][crate::peer::handshake] module
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct HandshakeConfig {
    /// Poll time to send sequence or sequence+acknowledgement to the other peer
    /// Also, the timeout for receiving sequence or sequence+acknowledgment from the other peer (in
    /// ms)
    pub peer_poll_time: u64,
    /// Timeout after which handshake can be declared failed if not complete (in ms)
    pub handshake_timeout: u64,
}

/// Structure to represent configuration for [`link`][crate::link] module
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct LinkConfig {
    /// Window size for the link. Determines how many packets are sent in a single burst
    pub window_size: u8,
    /// Time to wait for acknowledgement to be received
    pub ack_wait_time: u64,
    /// Poll time for shared memory structures
    pub poll_time_us: u64,
    /// Timeout or time of inactivity after which link is declared as broken
    pub timeout: u64,
    /// Time to wait for acknowledgment before sending packets again
    pub retry_delay: u64,
    /// Time to wait before sending another acknowledgment only packet when primary queue is empty
    /// i.e. no more packets to be sent
    pub ack_only_time: u64,
    /// Number of times a packet can be retried before link is declared as broken
    pub max_retries: i16,
}

impl Config {
    /// Returns configuration read from `file_path`
    /// Configuration file must be in [YAML](https://yaml.org/) format
    /// This may return an [`AetherError`] if the file is not present or if the file
    /// is not correctly formated as yaml
    ///
    /// # Examples
    ///
    /// ```
    /// use aether_lib::config::Config;
    /// use std::path::Path;
    ///
    /// // For a file located inside /home/user/aether_config.yaml we can construct
    /// // a path
    /// let path = Path::new("/home/user/aether_config.yaml");
    ///
    /// let config = Config::from_file(&path);
    /// ```
    pub fn from_file(file_path: &Path) -> Result<Config, AetherError> {
        match fs::read_to_string(file_path) {
            Ok(data) => match Config::try_from(data) {
                Ok(config) => Ok(config),
                Err(err) => Err(AetherError::YamlParse(err)),
            },
            Err(err) => Err(AetherError::FileRead(err)),
        }
    }

    /// Returns configuration read from the default configuration file
    /// If default configuration file is not found, the default internal configuration
    /// is returned
    ///
    /// # Examples
    ///
    /// ```
    /// use aether_lib::config::Config;
    /// let config = Config::get_config();
    /// ```
    pub fn get_config() -> Result<Config, AetherError> {
        match home::home_dir() {
            Some(mut path_buf) => {
                path_buf.push(".config");
                path_buf.push("aether");
                path_buf.push("config.yaml");

                let path = path_buf.as_path();

                info!(
                    "Reading configuration from {}",
                    path.to_str().unwrap_or("Cannot parse path")
                );

                match Config::from_file(path) {
                    Ok(config) => Ok(config),
                    Err(err) => match err {
                        AetherError::FileRead(file_err) => {
                            warn!("{:?}", file_err);
                            Ok(Config::default())
                        }
                        _ => Err(err),
                    },
                }
            }
            None => Ok(Config::default()),
        }
    }
}

impl TryFrom<String> for Config {
    type Error = serde_yaml::Error;
    fn try_from(string: String) -> Result<Self, Self::Error> {
        serde_yaml::from_str(&string)
    }
}

impl TryFrom<Config> for String {
    type Error = serde_yaml::Error;
    fn try_from(value: Config) -> Result<Self, Self::Error> {
        serde_yaml::to_string(&value)
    }
}

/// Default values for [`AetherConfig`]
impl Default for AetherConfig {
    fn default() -> Self {
        Self {
            server_retry_delay: 1_000,
            server_poll_time: 1_000,
            handshake_retry_delay: 1_500,
            connection_check_delay: 1_000,
            delta_time: 1000,
            poll_time_us: 100,
        }
    }
}

/// Default values for [`HandshakeConfig`]
impl Default for HandshakeConfig {
    fn default() -> Self {
        Self {
            peer_poll_time: 100,
            handshake_timeout: 2_500,
        }
    }
}

/// Default values for ['LinkConfig`]
impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            window_size: 20,
            ack_wait_time: 1_000,
            poll_time_us: 100,
            timeout: 10_000,
            retry_delay: 100,
            ack_only_time: 100,
            max_retries: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use std::{convert::TryFrom, fs, path::Path};

    #[test]
    fn read_test() {
        let default = Config::default();

        let path = "./tmp/config.yaml";

        fs::create_dir_all("./tmp").unwrap();

        fs::write(path, String::try_from(default).unwrap()).unwrap();

        let config = Config::from_file(Path::new(path)).unwrap();

        assert_eq!(config, default);
    }
}
