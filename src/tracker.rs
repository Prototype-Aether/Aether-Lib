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

#[cfg(test)]
mod tests {

    use crate::tracker::TrackerPacket;
    use serde::{Deserialize, Serialize};
    use std::convert::TryFrom;
    #[test]
    fn tracker_test() {
        let mut packet = TrackerPacket {
            username: "test".to_string(),
            req: true,
            packet_type: 10 as u8,
            port: 1234,
            ip: [1, 2, 3, 4],
        };
        let parsed_packet: Vec<u8> = TryFrom::try_from(packet).unwrap();
        let unparsed_packet: TrackerPacket = TryFrom::try_from(parsed_packet).unwrap();
        assert_eq!("test".to_string(), unparsed_packet.username);
        assert_eq!(true, unparsed_packet.req);
        assert_eq!(10, unparsed_packet.packet_type);
        assert_eq!(1234, unparsed_packet.port);
        assert_eq!([1, 2, 3, 4], unparsed_packet.ip);
    }
}
