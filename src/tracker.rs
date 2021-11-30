use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize)]
pub struct TrackerPacket {
    pub username: String,
    pub req: bool,
    pub packet_type: u8,
    pub port: u16,
    pub ip: [u8; 4],
}

impl TryFrom<TrackerPacket> for Vec<u8> {
    type Error = &'static str;

    fn try_from(packet: TrackerPacket) -> Result<Self, Self::Error> {
        match serde_json::to_string(&packet) {
            Ok(json) => Ok(json.into_bytes()),
            Err(_) => Err("Error converting to json"),
        }
    }
}

impl TryFrom<Vec<u8>> for TrackerPacket {
    type Error = &'static str;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        match String::from_utf8(bytes) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(data) => Ok(data),
                Err(_) => Err("Unable to parse json"),
            },
            Err(_) => Err("Unable to parse utf8"),
        }
    }
}
