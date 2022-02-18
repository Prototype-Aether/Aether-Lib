use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct ConnectionRequest {
    pub identity_number: u32,
    pub username: String,
    pub port: u16,
    pub ip: [u8; 4],
}

impl Clone for ConnectionRequest {
    fn clone(&self) -> Self {
        ConnectionRequest {
            identity_number: self.identity_number,
            username: self.username.clone(),
            port: self.port,
            ip: self.ip,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
pub struct TrackerPacket {
    pub identity_number: u32,
    pub username: String,
    pub peer_username: String,
    pub req: bool,
    pub packet_type: u8,
    pub port: u16,
    pub ip: [u8; 4],
    pub connections: Vec<ConnectionRequest>,
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

#[cfg(test)]
mod tests {

    use crate::tracker::{ConnectionRequest, TrackerPacket};
    use std::convert::TryFrom;
    #[test]
    fn tracker_test() {
        let connection = ConnectionRequest {
            identity_number: 32,
            username: String::from("someone"),
            port: 4200,
            ip: [42, 32, 22, 12],
        };

        let packet = TrackerPacket {
            identity_number: 42,
            peer_username: "another".to_string(),
            connections: vec![connection],
            username: "test".to_string(),
            req: true,
            packet_type: 10 as u8,
            port: 1234,
            ip: [1, 2, 3, 4],
        };

        let original_packet = packet.clone();

        let parsed_packet: Vec<u8> = TryFrom::try_from(packet).unwrap();
        let unparsed_packet: TrackerPacket = TryFrom::try_from(parsed_packet).unwrap();

        assert_eq!(unparsed_packet, original_packet);
    }
}
