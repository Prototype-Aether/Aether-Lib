use crate::error::AetherError;
use crate::identity::{Id, PublicId};
use crate::{acknowledgement::Acknowledgement, config::Config, packet::Packet};
use crate::{link::Link, packet::PType};
use std::io::ErrorKind;
use std::{
    net::{SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use rand::{thread_rng, Rng};

pub fn handshake(
    private_id: Id,
    socket: UdpSocket,
    address: SocketAddr,
    my_uid: String,
    peer_uid: String,
    config: Config,
) -> Result<Link, AetherError> {
    let seq = thread_rng().gen_range(0..(1 << 16_u32)) as u32;
    let recv_seq: u32;

    let ack: bool;

    if socket
        .set_read_timeout(Some(Duration::from_millis(config.handshake.peer_poll_time)))
        .is_err()
    {
        return Err(AetherError::SetReadTimeout);
    }

    let mut packet = Packet::new(PType::Initiation, seq);
    packet.append_payload(my_uid.into_bytes());

    let sequence_data = packet.compile();

    let now = SystemTime::now();
    // Repeat sending start sequence number and ID
    loop {
        let elapsed = now.elapsed()?;

        if elapsed.as_millis() > config.handshake.handshake_timeout.into() {
            return Err(AetherError::HandshakeError);
        }

        loop {
            match socket.send_to(&sequence_data, address) {
                Ok(_) => break,
                Err(err) => match err.kind() {
                    ErrorKind::PermissionDenied => continue,
                    _ => panic!("Error sending sequence: {}", err),
                },
            }
        }

        let mut buf: [u8; 1024] = [0; 1024];

        if let Ok(size) = socket.recv(&mut buf) {
            if size > 0 {
                let recved = Packet::from(buf[..size].to_vec());
                let uid_recved = match String::from_utf8(recved.payload.clone()) {
                    Ok(string) => string,
                    Err(_) => return Err(AetherError::HandshakeError),
                };

                // Verify the sender has the correct uid
                if uid_recved == peer_uid {
                    recv_seq = recved.sequence;

                    ack = recved.flags.ack && recved.ack.ack_begin == seq;

                    break;
                }
            }
        }
    }

    // If not acknowledged by other peer yet
    if !ack {
        packet.add_ack(Acknowledgement {
            ack_begin: recv_seq,
            ack_end: 0,
            miss_count: 0,
            miss: Vec::new(),
        });

        let ack_data = packet.compile();

        // Repeat sending start sequence number, acknowledgement and ID
        loop {
            let elapsed = now.elapsed()?;

            if elapsed.as_millis() > config.handshake.handshake_timeout.into() {
                return Err(AetherError::HandshakeError);
            }

            loop {
                match socket.send_to(&ack_data, address) {
                    Ok(_) => break,
                    Err(err) => match err.kind() {
                        ErrorKind::PermissionDenied => continue,
                        _ => panic!("Error sending sequence: {}", err),
                    },
                }
            }

            let mut buf: [u8; 1024] = [0; 1024];

            if let Ok(size) = socket.recv(&mut buf) {
                if size > 0 {
                    let recved = Packet::from(buf[..size].to_vec());
                    let uid_recved = match String::from_utf8(recved.payload.clone()) {
                        Ok(string) => string,
                        Err(_) => return Err(AetherError::HandshakeError),
                    };

                    // Verify the sender has the correct uid
                    if uid_recved == peer_uid
                        && recved.sequence == recv_seq
                        && recved.flags.ack
                        && recved.ack.ack_begin == seq
                    {
                        break;
                    }
                }
            }
        }
    }

    let peer_id = PublicId::from_base64(&peer_uid)?;

    // Start the link
    let mut link = Link::new(private_id, socket, address, peer_id, seq, recv_seq, config)?;
    link.start();
    Ok(link)
}
