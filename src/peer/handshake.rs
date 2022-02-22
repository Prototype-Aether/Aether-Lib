use crate::identity::Id;
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
) -> Result<Link, u8> {
    let seq = thread_rng().gen_range(0..(1 << 16_u32)) as u32;
    let recv_seq: u32;

    let ack: bool;

    socket
        .set_read_timeout(Some(Duration::from_millis(config.handshake.peer_poll_time)))
        .expect("Unable to set read timeout");

    let mut packet = Packet::new(PType::Initiation, seq);
    packet.append_payload(my_uid.into_bytes());

    let sequence_data = packet.compile();

    let now = SystemTime::now();
    // Repeat sending start sequence number and ID
    loop {
        let elapsed = now.elapsed().expect("Unable to get system time");

        if elapsed.as_millis() > config.handshake.handshake_timeout.into() {
            return Err(255);
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
                let uid_recved =
                    String::from_utf8(recved.payload.clone()).expect("Unable to get uid");

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
            let elapsed = now.elapsed().expect("Unable to get system time");

            if elapsed.as_millis() > config.handshake.handshake_timeout.into() {
                return Err(254);
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
                    let uid_recved =
                        String::from_utf8(recved.payload.clone()).expect("Unable to get uid");

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

    // Start the link
    let mut link = Link::new(private_id, socket, address, seq, recv_seq, config).unwrap();
    link.start();
    Ok(link)
}
