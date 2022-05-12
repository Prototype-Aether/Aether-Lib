[![Build](https://github.com/Prototype-Aether/Aether-Lib/actions/workflows/build.yml/badge.svg)](https://github.com/Prototype-Aether/Aether-Lib/actions/workflows/build.yml)
[![Tests](https://github.com/Prototype-Aether/Aether-Lib/actions/workflows/tests.yml/badge.svg)](https://github.com/Prototype-Aether/Aether-Lib/actions/workflows/tests.yml)
[![License](https://img.shields.io/badge/License-GPL--3.0-blue)](https://github.com/Prototype-Aether/Aether-Lib/blob/main/LICENSE)
# Aether Lib

Prototype Aether is a General Purpose Peer to Peer communication protocol. It allows
developers to develop P2P applications using an easy to use library.

The Rust library `aether_lib` contains the actual implementation of the protocol.
It can be used directly as a Rust library to develop applications. However, the
[Aether Service](https://github.com/Prototype-Aether/Aether-Service) which is currently
under development is recommended way to interact with Aether.

The documentation for Aether Lib can be found [here](https://prototype-aether.github.io/Aether-Lib/aether_lib/)

# Installation

Add `aether_lib` to your project in `Cargo.toml` as

```toml
[dependencies]
aether_lib = { git = "https://github.com/Prototype-Aether/Aether-Lib.git" }
```

# Basic Usage

The following examples show how to use `aether_lib` for P2P communications.

[A tracker server](https://github.com/Prototype-Aether/Aether-Tracker) is required
for the network to function. The tracker server helps in peer discovery and provides
public identities of the peers for UDP Holepunching (peer introduction).

## Starting a client

In order to use P2P communication, you need to initialize an `Aether` instance. This
can be done as follows -

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use aether_lib::peer::Aether;

// The socker address of a tracker server to be used
let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 8982);

// Initalize the link with a given tracker address
let aether = Aether::new(tracker_addr);

// Start the link
aether.start();
```

## Connecting to a peer

To start a P2P link to another peer, use the function `connect()` in `Aether`.

```rust
let peer_uid = String::from("<peer-uid-here>");
aether.connect(&peer_uid);
```

## Sending bytes to another peer

All communications happen in form of bytes, so you can send `Vec<u8>`. For example,
strings can be converted to bytes using `string.into_bytes()` (this converts to UTF-8
encoding) and from UTF-8 encoded bytes using `let string = String::from_utf8(bytes)`.

In order to send bytes to a user `peer_uid` you can use `send_to()`

```rust
let message = String::from("Hello");
let bytes = message.into_bytes();
aether.send_to(&peer_uid, bytes).unwrap();
```

## Receiving bytes from another peer

In order to receive bytes from a user `peer_uid` you can use `recv_from()`

```rust
let bytes = aether.recv_from(&peer_uid).unwrap();
let message = String::from_utf8(bytes).unwrap();
```
