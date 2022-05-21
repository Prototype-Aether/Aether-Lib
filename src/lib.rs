//! A library that provides P2P communication for Prototype Aether. Contains the
//! implementations of the Aether Protocol. This library can be used to develop
//! P2P applications. However, [Aether Service](https://github.com/Prototype-Aether/Aether-Service)
//! is the recommended way to interact with Aether.
//!
//! # Basic Usage
//!
//! Following examples demonstrate the basic usage including connecting to a peer and
//! sending and receiving bytes from the peer.
//!
//! ## Initializing a client
//!
//! A client can be initialized using [`Aether`][Aether]. In order to initialize
//! a client, you also need to have a tracker server. The tracker server implementation
//! can be found [here](https://github.com/Prototype-Aether/Aether-Tracker).
//!
//! ```rust,no_run
//! use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//! use aether_lib::peer::Aether;
//!
//! // address of the tracker server to be used
//! // one is hosted on 149.129.129.226:8982
//! let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(149, 129, 129, 226)), 8982);
//! // initialize the client
//! let aether = Aether::new(tracker_addr);
//!
//! // start the client
//! aether.start();
//! ```
//!
//! ## Connecting to a peer
//!
//! In order to connect to a peer, you need the other peer's UID. This UID is unique
//! to each client and is generated on the first run and saved on the file system (see [identity]).
//!
//! You can use the peer's UID to connect as follows
//!
//! ```rust,no_run
//! use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//! use aether_lib::peer::Aether;
//!
//! // address of the tracker server to be used
//! // one is hosted on 149.129.129.226:8982
//! let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(149, 129, 129, 226)), 8982);
//! // initialize the client
//! let aether = Aether::new(tracker_addr);
//!
//! // start the client
//! aether.start();
//!
//! // the UID of the other peer
//! let peer_uid = String::from("<peer-uid-here>");
//!
//! // connect to the other peer
//! aether.connect(&peer_uid);
//! ```
//!
//! ## Sending bytes to another peer
//!
//! All communications happen in form of bytes, so you can send [`Vec<u8>`]. For example,
//! strings can be converted to bytes using `string.into_bytes()` (this converts to UTF-8
//! encoding) and from UTF-8 encoded bytes using `let string = String::from_utf8(bytes)`.
//!
//! In order to send bytes to a user `peer_uid` you can use `send_to()`
//!
//! ```rust,no_run
//! use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//! use aether_lib::peer::Aether;
//!
//! // address of the tracker server to be used
//! // one is hosted on 149.129.129.226:8982
//! let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(149, 129, 129, 226)), 8982);
//! // initialize the client
//! let aether = Aether::new(tracker_addr);
//!
//! // start the client
//! aether.start();
//!
//! // the UID of the other peer
//! let peer_uid = String::from("<peer-uid-here>");
//!
//! // connect to the other peer
//! aether.connect(&peer_uid);
//!
//! // message to be sent
//! let message = String::from("Hello");
//!
//! // convert to bytes
//! let bytes = message.into_bytes();
//!
//! // send to peer with peer_uid
//! aether.send_to(&peer_uid, bytes).unwrap();
//! ```
//!
//! ## Receiving bytes from another peer
//!
//! In order to receive bytes from a user `peer_uid` you can use `recv_from()`
//!
//! ```rust,no_run
//! use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//! use aether_lib::peer::Aether;
//!
//! // address of the tracker server to be used
//! // one is hosted on 149.129.129.226:8982
//! let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(149, 129, 129, 226)), 8982);
//! // initialize the client
//! let aether = Aether::new(tracker_addr);
//!
//! // start the client
//! aether.start();
//!
//! // the UID of the other peer
//! let peer_uid = String::from("<peer-uid-here>");
//!
//! // connect to the other peer
//! aether.connect(&peer_uid);
//!
//! // receive bytes from peer with peer_uid
//! let bytes = aether.recv_from(&peer_uid).unwrap();
//!
//! // decode UTF-8 encoded bytes
//! let message = String::from_utf8(bytes).unwrap();
//! ```
//!
//! [Aether]: crate::peer::Aether
//! [identity]: crate::identity

pub mod acknowledgement;
pub mod config;
pub mod encryption;
pub mod error;
pub mod identity;
pub mod link;
pub mod packet;
pub mod peer;
pub mod tracker;
pub mod util;
