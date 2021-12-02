pub mod handshake;

use std::collections::VecDeque;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};

use std::thread;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, net::SocketAddr};

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use rand::{thread_rng, Rng};

use crate::link::POLL_TIME_US;
use crate::tracker::TrackerPacket;
use crate::{link::Link, tracker::ConnectionRequest};

use self::handshake::handshake;

pub const SERVER_RETRY_DELAY: u64 = 200;
pub const SERVER_POLL_TIME: u64 = 200;
pub const HANDSHAKE_RETRY_DELAY: u64 = 1000;
pub const CONNECTION_CHECK_DELAY: u64 = 200;
pub const DELTA_TIME: u64 = 200;

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

pub struct Aether {
    pub username: String,
    socket: Arc<UdpSocket>,
    peers: Arc<Mutex<HashMap<String, Peer>>>,
    is_connecting: Arc<Mutex<HashMap<String, bool>>>,
    initialized: Arc<Mutex<HashMap<String, Initialized>>>,
    requests: Arc<Mutex<VecDeque<ConnectionRequest>>>,
    failed: Arc<Mutex<HashMap<(u32, String), SystemTime>>>,
    tracker_addr: SocketAddr,
    id_number: Arc<Mutex<u32>>,
}

impl Aether {
    pub fn new(username: String, tracker_addr: SocketAddr) -> Aether {
        let socket = Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap());
        socket
            .set_read_timeout(Some(Duration::from_millis(SERVER_RETRY_DELAY)))
            .expect("Unable to set read timeout");
        Aether {
            username,
            peers: Arc::new(Mutex::new(HashMap::new())),
            initialized: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(VecDeque::new())),
            tracker_addr,
            is_connecting: Arc::new(Mutex::new(HashMap::new())),
            failed: Arc::new(Mutex::new(HashMap::new())),
            id_number: Arc::new(Mutex::new(1)),
            socket,
        }
    }

    pub fn start(&self) {
        println!("Starting aether service...");
        self.connection_poll();
        self.handle_initialized();
        self.handle_requests();
    }

    pub fn connect(&self, username: String) {
        let peers_lock = self.peers.lock().expect("Unable to lock peers");

        let is_connected = match (*peers_lock).get(&username) {
            Some(_) => true,
            None => false,
        };

        drop(peers_lock);

        if !is_connected {
            let mut id_lock = self.id_number.lock().expect("unable to lock id number");
            (*id_lock) = 1;
            let id_number = *id_lock;

            let mut initialized_lock = self
                .initialized
                .lock()
                .expect("unable to lock initailized list");

            let connection = Initialized {
                identity_number: id_number,
                socket: UdpSocket::bind(("0.0.0.0", 0)).expect("unable to create socket"),
                username: username.clone(),
            };

            (*initialized_lock).insert(username, connection);
        }
    }

    pub fn send_to(&self, username: &String, buf: Vec<u8>) -> Result<u8, u8> {
        let mut peers_lock = self.peers.lock().expect("unable to lock peers list");
        match (*peers_lock).get_mut(username) {
            Some(peer) => {
                peer.link.send(buf);
                Ok(0)
            }

            None => Err(1),
        }
    }

    pub fn recv_from(&self, username: &String) -> Result<Vec<u8>, u8> {
        let mut peers_lock = self.peers.lock().expect("unable to lock peers list");

        match (*peers_lock).get_mut(username) {
            Some(peer) => peer.link.recv(),
            None => Err(1),
        }
    }

    pub fn wait_connection(&self, username: &String) -> Result<u8, u8> {
        if !self.is_initialized(username) {
            if self.is_connecting(username) {
                while self.is_connecting(username) {
                    thread::sleep(Duration::from_millis(CONNECTION_CHECK_DELAY));
                }
                Ok(0)
            } else {
                if self.is_connected(username) {
                    Ok(0)
                } else {
                    Err(0)
                }
            }
        } else {
            while !self.is_connected(username) {
                thread::sleep(Duration::from_millis(CONNECTION_CHECK_DELAY));
            }
            Ok(0)
        }
    }

    pub fn is_connected(&self, username: &String) -> bool {
        let peers_lock = self.peers.lock().expect("unable to lock peers list");

        match (*peers_lock).get(username) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn is_connecting(&self, username: &String) -> bool {
        let connecting_lock = self
            .is_connecting
            .lock()
            .expect("unable to lock connecting list");
        match (*connecting_lock).get(username) {
            Some(v) => *v,
            None => false,
        }
    }

    pub fn is_initialized(&self, username: &String) -> bool {
        let init_lock = self
            .initialized
            .lock()
            .expect("unable to lock initialized list");

        match (*init_lock).get(username) {
            Some(_) => true,
            None => false,
        }
    }

    fn handle_initialized(&self) {
        let my_username = self.username.clone();
        let initialized = self.initialized.clone();
        let tracker_addr = self.tracker_addr.clone();
        thread::spawn(move || {
            loop {
                // Lock initialized list
                let init_lock = initialized.lock().expect("unable to lock initialized list");

                // For each initailized connection, send a connection request
                for (_, v) in (*init_lock).iter() {
                    let packet = TrackerPacket {
                        username: my_username.clone(),
                        peer_username: v.username.clone(),
                        identity_number: v.identity_number,
                        packet_type: 2,
                        req: true,
                        ..Default::default()
                    };

                    let packet_data: Vec<u8> =
                        Vec::try_from(packet).expect("Unable to encode packet");

                    v.socket
                        .send_to(&packet_data, tracker_addr)
                        .expect("unable to send packet to server");
                }

                // Unlock initailized list
                drop(init_lock);
                thread::sleep(Duration::from_millis(SERVER_POLL_TIME));
            }
        });
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
        let tracker_addr = self.tracker_addr.clone();

        let requests = self.requests.clone();

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

                //println!("{:?}", response_packet.connections);

                for v in response_packet.connections {
                    let mut req_lock = requests.lock().expect("unable to lock request queue");
                    (*req_lock).push_back(v);
                }

                thread::sleep(Duration::from_millis(SERVER_POLL_TIME));
            }
        });
    }

    fn handle_requests(&self) {
        let requests = self.requests.clone();
        let initialized = self.initialized.clone();
        let peers = self.peers.clone();
        let is_connecting = self.is_connecting.clone();
        let my_username = self.username.clone();
        let tracker_addr = self.tracker_addr.clone();

        let failed_list = self.failed.clone();

        let id_number = self.id_number.clone();

        thread::spawn(move || loop {
            let mut req_lock = requests.lock().expect("Unable to lock requests queue");

            // For each request received
            match (*req_lock).pop_front() {
                Some(request) => {
                    let failed_lock = failed_list.lock().expect("unable to lock failed list");
                    let elapsed = match (*failed_lock)
                        .get(&(request.identity_number, request.username.clone()))
                    {
                        Some(time) => time
                            .elapsed()
                            .expect("unable to get system time")
                            .as_millis(),
                        None => u128::MAX,
                    };
                    drop(failed_lock);

                    let mut init_lock =
                        initialized.lock().expect("unable to lock initialized list");
                    let init_option = (*init_lock).remove(&request.username);

                    // Check if already been initialized
                    match init_option {
                        // If initialized, start handshake
                        Some(init) => {
                            // if elapsed time since last fail is greater than threshold
                            // Only then try again
                            let delay = thread_rng().gen_range(0..DELTA_TIME);
                            if elapsed > (HANDSHAKE_RETRY_DELAY + delay).into() {
                                let mut connect_lock = is_connecting
                                    .lock()
                                    .expect("unable to lock is connecting list");
                                (*connect_lock).insert(init.username.clone(), true);

                                drop(connect_lock);

                                let is_connecting_clone = is_connecting.clone();

                                let username = my_username.clone();
                                let peers_list = peers.clone();

                                let failed_list_clone = failed_list.clone();

                                thread::spawn(move || {
                                    let peer_ip = IpAddr::V4(Ipv4Addr::from(request.ip));
                                    let peer_octets = match peer_ip {
                                        IpAddr::V4(ip4) => ip4.octets(),
                                        IpAddr::V6(_) => unreachable!(),
                                    };
                                    let peer_addr = SocketAddr::new(peer_ip, request.port);
                                    let peer_username = request.username;

                                    let mut success = false;

                                    let link_result = handshake(
                                        init.socket,
                                        peer_addr,
                                        username.clone(),
                                        peer_username.clone(),
                                    );

                                    match link_result {
                                        Ok(mut link) => {
                                            println!("Handshake success");
                                            link.send(username.clone().into_bytes());
                                            let delay = thread_rng().gen_range(0..DELTA_TIME);
                                            match link.recv_timeout(Duration::from_millis(
                                                HANDSHAKE_RETRY_DELAY / 2 + delay,
                                            )) {
                                                Ok(recved) => {
                                                    println!("Received nonce");
                                                    let recved_username =
                                                        match String::from_utf8(recved) {
                                                            Ok(name) => name,
                                                            Err(_) => String::from(""),
                                                        };

                                                    if recved_username == peer_username {
                                                        println!("Authenticated");
                                                        let peer = Peer {
                                                            username: peer_username.clone(),
                                                            ip: peer_octets,
                                                            port: request.port,
                                                            identity_number: request
                                                                .identity_number,
                                                            link,
                                                        };

                                                        let mut peers_lock = peers_list
                                                            .lock()
                                                            .expect("unable to lock peer list");

                                                        (*peers_lock)
                                                            .insert(peer_username.clone(), peer);
                                                        success = true;
                                                    } else {
                                                        println!("Authentication failed");
                                                    }
                                                }
                                                Err(255) => {
                                                    println!("Authentication failed")
                                                }
                                                _ => panic!("Unexpected error"),
                                            }
                                        }
                                        Err(e) => {
                                            println!("Handshake failed {}", e);
                                        }
                                    }

                                    let mut connect_lock = is_connecting_clone
                                        .lock()
                                        .expect("unable to lock is connecting list");
                                    (*connect_lock).insert(peer_username.clone(), false);

                                    // If unsuccessful store time of failure
                                    if !success {
                                        let mut failed_lock = failed_list_clone
                                            .lock()
                                            .expect("unable to lock failed list");
                                        (*failed_lock).insert(
                                            (request.identity_number, peer_username),
                                            SystemTime::now(),
                                        );
                                    } else {
                                        // if successful remove any time for failure
                                        let mut failed_lock = failed_list_clone
                                            .lock()
                                            .expect("unable to lock failed list");
                                        (*failed_lock)
                                            .remove(&(request.identity_number, peer_username));
                                    }

                                    drop(connect_lock);
                                });
                            } else {
                                (*init_lock).insert(init.username.clone(), init);
                            }
                        }
                        // If not initailized (other peer is initiator)
                        // Initailize the request
                        None => {
                            let connect_lock = is_connecting
                                .lock()
                                .expect("unable to lock is connecting list");

                            let flag = match (*connect_lock).get(&request.username) {
                                Some(v) => *v,
                                None => false,
                            };

                            drop(connect_lock);

                            if !flag {
                                let peers_lock = peers.lock().expect("unable to lock peers list");
                                let is_connected = match (*peers_lock).get(&request.username) {
                                    Some(_) => true,
                                    None => false,
                                };

                                drop(peers_lock);

                                // if already connected do nothing
                                if !is_connected {
                                    let mut id_lock =
                                        id_number.lock().expect("unable to lock id number");
                                    (*id_lock) = 1;
                                    let id_number = *id_lock;

                                    drop(id_lock);

                                    // Create new identity
                                    let connection = Initialized {
                                        identity_number: id_number,
                                        socket: UdpSocket::bind(("0.0.0.0", 0))
                                            .expect("unable to create socket"),
                                        username: request.username.clone(),
                                    };
                                    let packet = TrackerPacket {
                                        username: my_username.clone(),
                                        peer_username: connection.username.clone(),
                                        identity_number: connection.identity_number,
                                        packet_type: 2,
                                        req: true,
                                        ..Default::default()
                                    };

                                    let packet_data: Vec<u8> =
                                        Vec::try_from(packet).expect("Unable to encode packet");

                                    connection
                                        .socket
                                        .send_to(&packet_data, tracker_addr)
                                        .expect("unable to send packet to server");

                                    (*init_lock).insert(request.username.clone(), connection);

                                    (*req_lock).push_back(request);
                                }
                            }
                            drop(init_lock);
                        }
                    }
                }
                None => (),
            }

            drop(req_lock);
            thread::sleep(Duration::from_micros(POLL_TIME_US));
        });
    }
}
