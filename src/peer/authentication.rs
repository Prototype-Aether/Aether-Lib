use std::{net::IpAddr, time::Duration};

use crate::error::AetherError;
use crate::peer::Peer;
use rand::{thread_rng, Rng};

use crate::{config::Config, link::Link};

pub fn authenticate(
    link: Link,
    my_username: String,
    peer_username: String,
    identity_number: u32,
    config: Config,
) -> Result<Peer, AetherError> {
    // Authentication
    // Send own username
    link.send(my_username.clone().into_bytes()).unwrap();
    let delay = thread_rng().gen_range(0..config.aether.delta_time);

    let peer_octets = match link.get_addr().ip() {
        IpAddr::V4(v4) => v4.octets(),
        _ => unreachable!("Invalied IP address"),
    };

    let peer_port = link.get_addr().port();

    // Receive other peer's username
    match link.recv_timeout(Duration::from_millis(
        config.aether.handshake_retry_delay / 2 + delay,
    )) {
        Ok(recved) => {
            println!("Received nonce");
            let recved_username = match String::from_utf8(recved) {
                Ok(name) => name,
                Err(_) => String::from(""),
            };

            // If correct authentication
            if recved_username == peer_username {
                println!("Authenticated");

                // Create new Peer instance
                let peer = Peer {
                    username: peer_username.clone(),
                    ip: peer_octets,
                    port: peer_port,
                    identity_number,
                    link,
                };

                Ok(peer)
            } else {
                Err(AetherError::AuthenticationInvalid(peer_username))
            }
        }
        Err(err) => match err {
            AetherError::RecvTimeout => Err(AetherError::AuthenticationFailed(peer_username)),

            other => Err(other),
        },
    }
}
