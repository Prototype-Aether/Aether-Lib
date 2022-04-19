pub mod authentication;
pub mod handshake;

use log::{error, trace};

use std::collections::VecDeque;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex, MutexGuard};

use std::thread;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, net::SocketAddr};

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use rand::{thread_rng, Rng};

use crate::config::Config;
use crate::identity::Id;
use crate::peer::authentication::authenticate;
use crate::tracker::TrackerPacket;
use crate::{error::AetherError, link::Link, tracker::ConnectionRequest};

use self::handshake::handshake;

/// Enumeration representing different states of a connection
#[derive(Debug)]
pub enum Connection {
    /// Initialized state - connection has been initialized and is waiting to receive
    /// other peer's public identity
    Init(Initialized),
    /// Handshake state - handshake with the other peer is in progress
    Handshake,
    /// Connected state - a connection to the other peer has been established
    Connected(Box<Peer>),
    /// Failed state - a connection to the other peer had failed and would be retried
    Failed(Failure),
}

#[derive(Debug)]
pub struct Peer {
    pub uid: String,
    pub identity_number: u32,
    link: Link,
}

#[derive(Debug)]
pub struct Initialized {
    uid: String,
    socket: UdpSocket,
    identity_number: u32,
}

impl Initialized {
    pub fn new(uid: String) -> Initialized {
        Initialized {
            uid,
            socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
            identity_number: 1,
        }
    }
}

#[derive(Debug)]
pub struct Failure {
    time: SystemTime,
    socket: UdpSocket,
    uid: String,
}

/// [`Aether`] is an interface used to connect to other peers as well as communicate
/// with them
pub struct Aether {
    /// Username assigned to the Aether instance
    uid: String,
    /// Identity of user
    private_id: Id,
    /// The [`UdpSocket`] to be used for communication
    socket: Arc<UdpSocket>,
    /// Queue of connection requests received
    requests: Arc<Mutex<VecDeque<ConnectionRequest>>>,
    /// Address of the tracker server
    tracker_addr: SocketAddr,
    /// List of peers related to this peer
    connections: Arc<Mutex<HashMap<String, Connection>>>,
    /// Configuration
    config: Config,
}

impl Aether {
    pub fn new(tracker_addr: SocketAddr) -> Self {
        let private_id = Id::load_or_generate().expect("Error loading identity");

        Self::new_with_id(private_id, tracker_addr)
    }

    pub fn new_with_id(id: Id, tracker_addr: SocketAddr) -> Self {
        let config = Config::get_config().expect("Error getting config");

        let uid = id.public_key_to_base64().expect("Error getting public key");

        let socket = Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap());
        socket
            .set_read_timeout(Some(Duration::from_millis(
                config.aether.server_retry_delay,
            )))
            .expect("Unable to set read timeout");
        Aether {
            uid,
            private_id: id,
            requests: Arc::new(Mutex::new(VecDeque::new())),
            tracker_addr,
            socket,
            connections: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn get_uid(&self) -> &str {
        &self.uid
    }

    pub fn start(&self) {
        trace!("Starting aether service...");
        self.connection_poll();
        self.handle_sockets();
        self.handle_requests();
    }

    pub fn connect(&self, uid: &str) {
        let mut connections_lock = self.connections.lock().expect("Unable to lock peers");

        let is_present = (*connections_lock).get(uid).is_some();

        if !is_present {
            let initialized = Initialized::new(uid.to_string());

            (*connections_lock).insert(uid.to_string(), Connection::Init(initialized));
        }
    }

    pub fn send_to(&self, uid: &str, buf: Vec<u8>) -> Result<u8, u8> {
        let mut connections_lock = self.connections.lock().expect("unable to lock peers list");
        match (*connections_lock).get_mut(uid) {
            Some(connection) => match connection {
                Connection::Connected(peer) => {
                    peer.link.send(buf).unwrap();
                    Ok(0)
                }
                _ => Err(3),
            },

            None => Err(1),
        }
    }

    pub fn recv_from(&self, uid: &str) -> Result<Vec<u8>, AetherError> {
        let connections_lock = match self.connections.lock() {
            Ok(lock) => lock,
            Err(_) => return Err(AetherError::MutexLock("connections")),
        };

        let peer = match (*connections_lock).get(uid) {
            Some(Connection::Connected(peer)) => peer,
            _ => return Err(AetherError::NotConnected(uid.to_string())),
        };

        let receiver = peer.link.get_receiver()?;

        drop(connections_lock);

        let packet = receiver.recv()?;

        Ok(packet.payload)
    }

    pub fn wait_connection(&self, uid: &str) -> Result<u8, u8> {
        if !self.is_initialized(uid) {
            if self.is_connecting(uid) {
                while self.is_connecting(uid) {
                    thread::sleep(Duration::from_millis(
                        self.config.aether.connection_check_delay,
                    ));
                }
                Ok(0)
            } else if self.is_connected(uid) {
                Ok(0)
            } else {
                Err(0)
            }
        } else {
            while !self.is_connected(uid) {
                thread::sleep(Duration::from_millis(
                    self.config.aether.connection_check_delay,
                ));
            }
            Ok(0)
        }
    }

    pub fn is_connected(&self, uid: &str) -> bool {
        let connections_lock = self.connections.lock().expect("unable to lock peers list");
        matches!((*connections_lock).get(uid), Some(Connection::Connected(_)))
    }

    pub fn is_connecting(&self, uid: &str) -> bool {
        let connections_lock = self
            .connections
            .lock()
            .expect("unable to lock connecting list");
        match (*connections_lock).get(uid) {
            Some(connection) => {
                !matches!(connection, Connection::Failed(_) | Connection::Connected(_))
            }
            None => false,
        }
    }

    pub fn is_initialized(&self, uid: &str) -> bool {
        let connections_lock = self
            .connections
            .lock()
            .expect("unable to lock connecting list");
        matches!((*connections_lock).get(uid), Some(Connection::Init(_)))
    }

    fn handle_sockets(&self) {
        let my_uid = self.uid.clone();
        let connections = self.connections.clone();
        let tracker_addr = self.tracker_addr;
        let config = self.config;
        thread::spawn(move || {
            loop {
                // Lock connections list
                let connections_lock = connections.lock().expect("unable to lock initialized list");

                // For each connection
                for (_, connection) in (*connections_lock).iter() {
                    // If connection is in initialized or failed state, send connection
                    // request
                    match connection {
                        Connection::Init(init) => {
                            Self::send_connection_request(
                                my_uid.clone(),
                                init.uid.clone(),
                                &init.socket,
                                tracker_addr,
                            );
                        }
                        Connection::Failed(failed) => Self::send_connection_request(
                            my_uid.clone(),
                            failed.uid.clone(),
                            &failed.socket,
                            tracker_addr,
                        ),
                        _ => {}
                    };
                }

                // Unlock initailized list
                drop(connections_lock);
                thread::sleep(Duration::from_millis(config.aether.server_poll_time));
            }
        });
    }

    fn send_connection_request(
        uid: String,
        peer_uid: String,
        socket: &UdpSocket,
        tracker_addr: SocketAddr,
    ) {
        let packet = TrackerPacket {
            username: uid,
            peer_username: peer_uid,
            identity_number: 1,
            packet_type: 2,
            req: true,
            ..Default::default()
        };

        let packet_data: Vec<u8> = Vec::try_from(packet).expect("Unable to encode packet");

        socket
            .send_to(&packet_data, tracker_addr)
            .expect("unable to send packet to server");
    }

    fn connection_poll(&self) {
        let poll_request = TrackerPacket {
            username: self.uid.clone(),
            packet_type: 3,
            req: true,
            ..Default::default()
        };

        let data_bytes: Vec<u8> = Vec::try_from(poll_request).expect("Unable to encode packet");
        let mut buf: [u8; 1024] = [0; 1024];

        let socket = self.socket.clone();
        let tracker_addr = self.tracker_addr;

        let requests = self.requests.clone();

        let config = self.config;

        thread::spawn(move || loop {
            socket
                .send_to(&data_bytes, tracker_addr)
                .expect("Unable to send to server");

            let response_data = match socket.recv(&mut buf) {
                Ok(size) => buf[..size].to_vec(),
                Err(_) => Vec::new(),
            };

            if !response_data.is_empty() {
                let response_packet =
                    TrackerPacket::try_from(response_data).expect("Unable to decode packet");

                for v in response_packet.connections {
                    let mut req_lock = requests.lock().expect("unable to lock request queue");
                    (*req_lock).push_back(v);
                }

                thread::sleep(Duration::from_millis(config.aether.server_poll_time));
            }
        });
    }

    fn handle_requests(&self) {
        let requests = self.requests.clone();
        let connections = self.connections.clone();
        let my_uid = self.uid.clone();
        let tracker_addr = self.tracker_addr;
        let config = self.config;
        let private_id = self.private_id.clone();

        thread::spawn(move || loop {
            let mut req_lock = requests.lock().expect("Unable to lock requests queue");

            // For each request received
            if let Some(request) = (*req_lock).pop_front() {
                Self::handle_request(
                    private_id.clone(),
                    request,
                    my_uid.clone(),
                    &mut connections.clone(),
                    tracker_addr,
                    &mut req_lock,
                    config,
                )
            }

            drop(req_lock);
            thread::sleep(Duration::from_micros(config.aether.poll_time_us));
        });
    }

    fn handle_request(
        private_id: Id,
        request: ConnectionRequest,
        my_uid: String,
        connections: &mut Arc<Mutex<HashMap<String, Connection>>>,
        tracker_addr: SocketAddr,
        req_lock: &mut MutexGuard<VecDeque<ConnectionRequest>>,
        config: Config,
    ) {
        let mut connections_lock = connections.lock().expect("unable to lock failed list");
        // Clone important data to pass to handshake thread
        let connections_clone = connections.clone();
        let my_uid_clone = my_uid.clone();

        let config_clone = config;

        let handshake_thread = move |init: Initialized, request: ConnectionRequest| {
            // Initailize data values for handshake
            let peer_ip = IpAddr::V4(Ipv4Addr::from(request.ip));
            let peer_addr = SocketAddr::new(peer_ip, request.port);
            let peer_uid = request.username;

            let mut success = false; // This bool DOES in fact get read and modified. Not sure why compiler doesn't recognize its usage.

            // Start handshake
            let link_result = handshake(
                private_id,
                init.socket,
                peer_addr,
                my_uid_clone.clone(),
                peer_uid.clone(),
                config_clone,
            );

            match link_result {
                Ok(link) => {
                    trace!("Handshake success");

                    match authenticate(link, peer_uid.clone(), request.identity_number, config) {
                        Ok(mut peer) => {
                            if let Err(err) = peer.link.enable_encryption() {
                                error!("Cannot enable encryption: {}", err);
                            } else {
                                let mut connections_lock =
                                    connections_clone.lock().expect("unable to lock peer list");

                                // Add connected peer to connections list
                                // with connected state
                                (*connections_lock).insert(
                                    peer_uid.clone(),
                                    Connection::Connected(Box::new(peer)),
                                );
                                success = true;
                            }
                        }
                        Err(AetherError::AuthenticationFailed(_)) => {
                            trace!("Cannot reach");
                        }
                        Err(AetherError::AuthenticationInvalid(_)) => {
                            error!("Identity could not be authenticated")
                        }
                        Err(other) => {
                            panic!("Unexpected error {}", other);
                        }
                    }
                }
                Err(e) => {
                    trace!("Handshake failed {}", e);
                }
            }

            // If unsuccessful store time of failure
            if !success {
                let mut connections_lock =
                    connections_clone.lock().expect("unable to lock peer list");

                // Add failure entry to connection list
                (*connections_lock).insert(
                    peer_uid.clone(),
                    Connection::Failed(Failure {
                        time: SystemTime::now(),
                        socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
                        uid: peer_uid,
                    }),
                );
            }
        };

        // Check if connection exists in connection list
        match (*connections_lock).remove(&request.username) {
            // If initialized, start handshake
            // Initailized either since connection request was made by us first
            // Or initailized after receiving connection request from other peer
            Some(Connection::Init(init)) => {
                // Put current user in handshake state
                (*connections_lock).insert(init.uid.clone(), Connection::Handshake);

                // Create a thread to start handshake and establish connection
                thread::spawn(move || handshake_thread(init, request));
            }
            Some(Connection::Failed(failed)) => {
                let delta = thread_rng().gen_range(0..config.aether.delta_time);
                let elapsed = failed
                    .time
                    .elapsed()
                    .expect("unable to get system time")
                    .as_millis();

                // if elapsed time since the fail is greater than threshold
                // then put back in initialized state
                if elapsed > (config.aether.handshake_retry_delay + delta).into() {
                    (*connections_lock).insert(
                        failed.uid.clone(),
                        Connection::Init(Initialized {
                            uid: failed.uid,
                            socket: failed.socket,
                            identity_number: 1,
                        }),
                    );
                } else {
                    // If elapsed time is not long enough
                    // insert back into the list
                    (*connections_lock).insert(failed.uid.clone(), Connection::Failed(failed));
                }
            }
            Some(other) => {
                // If in other state, insert back the value
                (*connections_lock).insert(request.username.clone(), other);
            }
            // If not in connections (other peer is initiator)
            // Initailize the request
            None => {
                // Create new identity
                let connection = Initialized {
                    identity_number: 1,
                    socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
                    uid: request.username.clone(),
                };

                let packet = TrackerPacket {
                    username: my_uid,
                    peer_username: connection.uid.clone(),
                    identity_number: connection.identity_number,
                    packet_type: 2,
                    req: true,
                    ..Default::default()
                };

                let packet_data: Vec<u8> = Vec::try_from(packet).expect("Unable to encode packet");

                connection
                    .socket
                    .send_to(&packet_data, tracker_addr)
                    .expect("unable to send packet to server");

                // Insert new initialized connection
                (*connections_lock).insert(request.username.clone(), Connection::Init(connection));

                (*req_lock).push_back(request);
            }
        }
    }
}
