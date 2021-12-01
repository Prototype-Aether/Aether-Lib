use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize)]
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
            ip: self.ip.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
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

    use crate::tracker::TrackerPacket;
    use serde::{Deserialize, Serialize};
    use std::convert::TryFrom;
    #[test]
    fn tracker_test() {
        let packet = TrackerPacket {
            identity_number: 42,
            peer_username: "another".to_string(),
            connections: Vec::new(),
            username: "test".to_string(),
            req: true,
            packet_type: 10 as u8,
            port: 1234,
            ip: [1, 2, 3, 4],
        };
        let parsed_packet: Vec<u8> = TryFrom::try_from(packet).unwrap();
        let unparsed_packet: TrackerPacket = TryFrom::try_from(parsed_packet).unwrap();
        assert_eq!("test".to_string(), unparsed_packet.username);
        assert_eq!("another".to_string(), unparsed_packet.peer_username);
        assert!(unparsed_packet.connections.is_empty());
        assert_eq!(42, unparsed_packet.identity_number);
        assert_eq!(true, unparsed_packet.req);
        assert_eq!(10, unparsed_packet.packet_type);
        assert_eq!(1234, unparsed_packet.port);
        assert_eq!([1, 2, 3, 4], unparsed_packet.ip);
    }
}
