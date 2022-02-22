use std::time::Duration;

use crate::identity::PublicId;
use crate::peer::Peer;
use crate::{error::AetherError, util::gen_nonce};
use rand::{thread_rng, Rng};

use crate::{config::Config, link::Link};

/// Size of the nonce to be used in authentication in bytes
pub const NONCE_SIZE: u8 = 32;

pub fn authenticate(
    link: Link,
    peer_uid: String,
    identity_number: u32,
    config: Config,
) -> Result<Peer, AetherError> {
    // Authentication
    // Send own uid
    let delta = thread_rng().gen_range(0..config.aether.delta_time);
    let recv_timeout = Duration::from_millis(config.aether.handshake_retry_delay + delta);

    let other_id = PublicId::from_base64(&peer_uid)?;

    // generate nonce
    let nonce = gen_nonce(NONCE_SIZE);

    // encrypt nonce with public key and send to other peer
    link.send(other_id.public_encrypt(&nonce)?).unwrap();

    // receive encrypted nonce
    let nonce_enc = match link.recv_timeout(recv_timeout) {
        Ok(data) => data,
        Err(err) => match err {
            AetherError::RecvTimeout => return Err(AetherError::AuthenticationFailed(peer_uid)),
            other => return Err(other),
        },
    };

    // TODO: Decrypt nonce received
    let nonce_dec = link.private_id.private_decrypt(&nonce_enc)?;

    // send decrypted nonce
    link.send(nonce_dec).unwrap();

    // receive decrypted nonce
    let nonce_recv = match link.recv_timeout(recv_timeout) {
        Ok(data) => data,
        Err(err) => match err {
            AetherError::RecvTimeout => return Err(AetherError::AuthenticationFailed(peer_uid)),
            other => return Err(other),
        },
    };

    // if nonce received is same as nonce sent, the other peer is authenticated
    if nonce == nonce_recv {
        println!("Authenticated");

        // Create new Peer instance
        let peer = Peer {
            uid: peer_uid.clone(),
            identity_number,
            link,
        };

        Ok(peer)
    } else {
        Err(AetherError::AuthenticationInvalid(peer_uid))
    }
}
