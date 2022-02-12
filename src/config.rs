use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, default::Default, fs, path::Path};

use crate::error::AetherError;

/// Structure to represent configuration options for `aether_lib`
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub aether: AetherConfig,
}

/// Structure to represent configuration for [`Aether`][crate::peer::Aether]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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
                Err(_) => Err(AetherError {
                    code: 1007,
                    description: String::from("Unable to parse config file"),
                    cause: None,
                }),
            },
            Err(err) => Err(AetherError {
                code: 1008,
                description: format!("Unable to read config file: {}", err),
                cause: None,
            }),
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

                println!(
                    "Reading configuration from {}",
                    path.to_str().unwrap_or("Cannot parse path")
                );

                match Config::from_file(path) {
                    Ok(config) => Ok(config),
                    Err(err) => match err.code {
                        1008 => {
                            println!("{:?}", err);
                            Ok(Config::default())
                        }
                        _ => Err(AetherError {
                            code: 1009,
                            description: String::from("Unable to read default config file"),
                            cause: Some(Box::new(err)),
                        }),
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
