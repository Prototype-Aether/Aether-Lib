use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use crate::{acknowledgment::Acknowledgment, packet::Packet};
use crate::{link::Link, packet::PType};

use rand::{thread_rng, Rng};

const INITIATE_DELAY: u64 = 500;

pub fn handshake(
    socket: UdpSocket,
    address: SocketAddr,
    my_username: String,
    peer_username: String,
) -> Link {
    let seq = thread_rng().gen_range(0..(1 << 16 as u32)) as u32;
    let recv_seq: u32;

    println!("Seq: {}", seq);

    let ack: bool;

    socket
        .set_read_timeout(Some(Duration::from_millis(INITIATE_DELAY)))
        .expect("Unable to set read timeout");

    let mut packet = Packet::new(PType::Initiation, seq);
    packet.append_payload(my_username.clone().into_bytes());

    let sequence_data = packet.compile();

    // Repeat sending start sequence number and ID
    loop {
        socket
            .send_to(&sequence_data, address)
            .expect("Couldn't send sequence");

        let mut buf: [u8; 1024] = [0; 1024];

        match socket.recv(&mut buf) {
            Ok(size) => {
                if thread_rng().gen_range(0..100) > 20 && size > 0 {
                    let recved = Packet::from(buf[..size].to_vec());
                    let username_recved =
                        String::from_utf8(recved.payload.clone()).expect("Unable to get username");

                    // Verify the sender has the correct username
                    if username_recved == peer_username {
                        recv_seq = recved.sequence;

                        ack = recved.flags.ack && recved.ack.ack_begin == seq;

                        break;
                    }
                }
            }
            _ => (),
        }
    }

    println!("{}: stage 1 complete", my_username);

    // If not acknowledged by other peer yet
    if !ack {
        packet.add_ack(Acknowledgment {
            ack_begin: recv_seq,
            ack_end: 0,
            miss_count: 0,
            miss: Vec::new(),
        });

        let ack_data = packet.compile();

        // Repeat sending start sequence number, acknowledgement and ID
        loop {
            socket
                .send_to(&ack_data, address)
                .expect("Couldn't send sequence");

            let mut buf: [u8; 1024] = [0; 1024];
            if thread_rng().gen_range(0..100) < 99 {
                continue;
            }
            match socket.recv(&mut buf) {
                Ok(size) => {
                    if thread_rng().gen_range(0..100) > 20 && size > 0 {
                        let recved = Packet::from(buf[..size].to_vec());
                        let username_recved = String::from_utf8(recved.payload.clone())
                            .expect("Unable to get username");

                        // Verify the sender has the correct username
                        if username_recved == peer_username && recved.sequence == recv_seq {
                            if recved.flags.ack && recved.ack.ack_begin == seq {
                                break;
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    println!("{}: stage 2 complete", my_username);
    println!("Handshake done {}", seq);

    // Start the link
    let mut link = Link::new(socket, address.clone(), seq, recv_seq);

    link.start();

    link
}
