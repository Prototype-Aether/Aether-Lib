pub mod authentication;
pub mod handshake;

use std::collections::VecDeque;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex, MutexGuard};

use std::thread;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, net::SocketAddr};

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use rand::{thread_rng, Rng};

use crate::config::Config;
use crate::peer::authentication::authenticate;
use crate::tracker::TrackerPacket;
use crate::{error::AetherError, link::Link, tracker::ConnectionRequest};

use self::handshake::handshake;

/// Enumeration representing different states of a connection
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

pub struct Peer {
    pub username: String,
    pub ip: [u8; 4],
    pub port: u16,
    pub identity_number: u32,
    link: Link,
}

#[derive(Debug)]
pub struct Initialized {
    username: String,
    socket: UdpSocket,
    identity_number: u32,
}

impl Initialized {
    pub fn new(username: String) -> Initialized {
        Initialized {
            username,
            socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
            identity_number: 1,
        }
    }
}

#[derive(Debug)]
pub struct Failure {
    time: SystemTime,
    socket: UdpSocket,
    username: String,
}

/// [`Aether`] is an interface used to connect to other peers as well as communicate
/// with them
pub struct Aether {
    /// Username assigned to the Aether instance
    pub username: String,
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
    pub fn new(username: String, tracker_addr: SocketAddr) -> Aether {
        let config = Config::get_config().expect("Error getting config");
        let socket = Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap());
        socket
            .set_read_timeout(Some(Duration::from_millis(
                config.aether.server_retry_delay,
            )))
            .expect("Unable to set read timeout");
        Aether {
            username,
            requests: Arc::new(Mutex::new(VecDeque::new())),
            tracker_addr,
            socket,
            connections: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn start(&self) {
        println!("Starting aether service...");
        self.connection_poll();
        self.handle_sockets();
        self.handle_requests();
    }

    pub fn connect(&self, username: String) {
        let mut connections_lock = self.connections.lock().expect("Unable to lock peers");

        let is_present = (*connections_lock).get(&username).is_some();

        if !is_present {
            let initialized = Initialized::new(username.clone());

            (*connections_lock).insert(username, Connection::Init(initialized));
        }
    }

    pub fn send_to(&self, username: &str, buf: Vec<u8>) -> Result<u8, u8> {
        let mut connections_lock = self.connections.lock().expect("unable to lock peers list");
        match (*connections_lock).get_mut(username) {
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

    pub fn recv_from(&self, username: &str) -> Result<Vec<u8>, AetherError> {
        match self.connections.lock() {
            Ok(mut connections_lock) => match (*connections_lock).get_mut(username) {
                Some(Connection::Connected(peer)) => match peer.link.recv() {
                    Ok(recv_vec) => {
                        log::info!("Link Receive Module succesfully initialized.");
                        Ok(recv_vec)
                    }
                    Err(aether_error) => Err(aether_error),
                },
                _ => Err(AetherError::NotConnected(username.to_string())),
            },
            Err(_) => Err(AetherError::MutexLock("connections")),
        }
    }

    pub fn wait_connection(&self, username: &str) -> Result<u8, u8> {
        if !self.is_initialized(username) {
            if self.is_connecting(username) {
                while self.is_connecting(username) {
                    thread::sleep(Duration::from_millis(
                        self.config.aether.connection_check_delay,
                    ));
                }
                Ok(0)
            } else if self.is_connected(username) {
                Ok(0)
            } else {
                Err(0)
            }
        } else {
            while !self.is_connected(username) {
                thread::sleep(Duration::from_millis(
                    self.config.aether.connection_check_delay,
                ));
            }
            Ok(0)
        }
    }

    pub fn is_connected(&self, username: &str) -> bool {
        let connections_lock = self.connections.lock().expect("unable to lock peers list");
        matches!(
            (*connections_lock).get(username),
            Some(Connection::Connected(_))
        )
    }

    pub fn is_connecting(&self, username: &str) -> bool {
        let connections_lock = self
            .connections
            .lock()
            .expect("unable to lock connecting list");
        match (*connections_lock).get(username) {
            Some(connection) => {
                !matches!(connection, Connection::Failed(_) | Connection::Connected(_))
            }
            None => false,
        }
    }

    pub fn is_initialized(&self, username: &str) -> bool {
        let connections_lock = self
            .connections
            .lock()
            .expect("unable to lock connecting list");
        matches!((*connections_lock).get(username), Some(Connection::Init(_)))
    }

    fn handle_sockets(&self) {
        let my_username = self.username.clone();
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
                                my_username.clone(),
                                init.username.clone(),
                                &init.socket,
                                tracker_addr,
                            );
                        }
                        Connection::Failed(failed) => Self::send_connection_request(
                            my_username.clone(),
                            failed.username.clone(),
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
        username: String,
        peer_username: String,
        socket: &UdpSocket,
        tracker_addr: SocketAddr,
    ) {
        let packet = TrackerPacket {
            username,
            peer_username,
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
            username: self.username.clone(),
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
        let my_username = self.username.clone();
        let tracker_addr = self.tracker_addr;
        let config = self.config;

        thread::spawn(move || loop {
            let mut req_lock = requests.lock().expect("Unable to lock requests queue");

            // For each request received
            if let Some(request) = (*req_lock).pop_front() {
                Self::handle_request(
                    request,
                    my_username.clone(),
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
        request: ConnectionRequest,
        my_username: String,
        connections: &mut Arc<Mutex<HashMap<String, Connection>>>,
        tracker_addr: SocketAddr,
        req_lock: &mut MutexGuard<VecDeque<ConnectionRequest>>,
        config: Config,
    ) {
        let mut connections_lock = connections.lock().expect("unable to lock failed list");
        // Clone important data to pass to handshake thread
        let connections_clone = connections.clone();
        let my_username_clone = my_username.clone();

        let config_clone = config;

        let handshake_thread = move |init: Initialized, request: ConnectionRequest| {
            // Initailize data values for handshake
            let peer_ip = IpAddr::V4(Ipv4Addr::from(request.ip));
            let peer_octets = match peer_ip {
                IpAddr::V4(ip4) => ip4.octets(),
                IpAddr::V6(_) => unreachable!(),
            };
            let peer_addr = SocketAddr::new(peer_ip, request.port);
            let peer_username = request.username;

            let mut success = false; // This bool DOES in fact get read and modified. Not sure why compiler doesn't recognize its usage.

            // Start handshake
            let link_result = handshake(
                init.socket,
                peer_addr,
                my_username_clone.clone(),
                peer_username.clone(),
                config_clone,
            );

            match link_result {
                Ok(link) => {
                    println!("Handshake success");

                    match authenticate(
                        link,
                        my_username_clone,
                        peer_username.clone(),
                        peer_octets,
                        request.port,
                        request.identity_number,
                        config,
                    ) {
                        Ok(peer) => {
                            let mut connections_lock =
                                connections_clone.lock().expect("unable to lock peer list");

                            // Add connected peer to connections list
                            // with connected state
                            (*connections_lock).insert(
                                peer_username.clone(),
                                Connection::Connected(Box::new(peer)),
                            );
                            success = true;
                        }
                        Err(AetherError::AuthenticationFailed(_)) => {
                            println!("Cannot reach");
                        }
                        Err(AetherError::AuthenticationInvalid(_)) => {
                            println!("Identity could not be authenticated")
                        }
                        Err(other) => {
                            panic!("Unexpected error {}", other);
                        }
                    }
                }
                Err(e) => {
                    println!("Handshake failed {}", e);
                }
            }

            // If unsuccessful store time of failure
            if !success {
                let mut connections_lock =
                    connections_clone.lock().expect("unable to lock peer list");

                // Add failure entry to connection list
                (*connections_lock).insert(
                    peer_username.clone(),
                    Connection::Failed(Failure {
                        time: SystemTime::now(),
                        socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
                        username: peer_username,
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
                (*connections_lock).insert(init.username.clone(), Connection::Handshake);

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
                        failed.username.clone(),
                        Connection::Init(Initialized {
                            username: failed.username,
                            socket: failed.socket,
                            identity_number: 1,
                        }),
                    );
                } else {
                    // If elapsed time is not long enough
                    // insert back into the list
                    (*connections_lock).insert(failed.username.clone(), Connection::Failed(failed));
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
                    username: request.username.clone(),
                };

                let packet = TrackerPacket {
                    username: my_username,
                    peer_username: connection.username.clone(),
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
