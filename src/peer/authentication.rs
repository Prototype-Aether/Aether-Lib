use std::{net::IpAddr, time::Duration};

use crate::peer::Peer;
use crate::{error::AetherError, util::gen_nonce};
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
    let delta = thread_rng().gen_range(0..config.aether.delta_time);
    let recv_timeout = Duration::from_millis(config.aether.handshake_retry_delay + delta);

    let peer_octets = match link.get_addr().ip() {
        IpAddr::V4(v4) => v4.octets(),
        _ => unreachable!("Invalied IP address"),
    };

    let peer_port = link.get_addr().port();

    let nonce = gen_nonce(32);

    // generate nonce
    link.send(nonce.clone()).unwrap();

    // TODO: encrypt nonce with public key

    // receive encrypted nonce
    let nonce_enc = match link.recv_timeout(recv_timeout) {
        Ok(data) => data,
        Err(err) => match err {
            AetherError::RecvTimeout => {
                return Err(AetherError::AuthenticationFailed(peer_username))
            }
            other => return Err(other),
        },
    };

    // TODO: Decrypt nonce received

    // send decrypted nonce
    link.send(nonce_enc).unwrap();

    // receive decrypted nonce
    let nonce_recv = match link.recv_timeout(recv_timeout) {
        Ok(data) => data,
        Err(err) => match err {
            AetherError::RecvTimeout => {
                return Err(AetherError::AuthenticationFailed(peer_username))
            }
            other => return Err(other),
        },
    };

    // if nonce received is same as nonce sent, the other peer is authenticated
    if nonce == nonce_recv {
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
