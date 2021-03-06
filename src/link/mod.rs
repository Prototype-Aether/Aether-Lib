//! Structure for representing a reliable [`Link`] between 2 peers.

pub mod decryptionthread;
pub mod receivethread;
pub mod sendthread;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;

use crate::acknowledgement::{AcknowledgementCheck, AcknowledgementList};
use crate::config::Config;
use crate::encryption::AetherCipher;
use crate::encryption::KEY_SIZE;
use crate::error::AetherError;
use crate::identity::Id;
use crate::identity::PublicId;
use crate::link::receivethread::ReceiveThread;
use crate::link::sendthread::SendThread;
use crate::packet::PType;
use crate::packet::Packet;
use crate::util::gen_nonce;
use crate::util::xor;

use self::decryptionthread::DecryptionThread;

/// Check if a given packet needs to be acknowledged based on the [`PType`]
pub fn needs_ack(packet: &Packet) -> bool {
    match packet.flags.p_type {
        PType::Data => true,
        PType::KeyExchange => true,
        PType::AckOnly => false,
        _ => false,
    }
}

/// Represents a single reliable [`Link`] to another peer
#[derive(Debug)]
pub struct Link {
    /// Identity of the user that created this identity
    pub private_id: Id,
    /// Public Identity of the other peer
    pub peer_id: PublicId,
    /// The symmetric cipher to be used for E2EE
    cipher: Option<AetherCipher>,
    /// List of the acknowledgments that have to be sent to the other peer
    ack_list: Arc<Mutex<AcknowledgementList>>,
    /// List of the acknowledgments received from the other peer
    ack_check: Arc<Mutex<AcknowledgementCheck>>,
    /// UDP socket used to communicate with the other peer
    socket: Arc<UdpSocket>,
    /// The address of the other peer
    peer_addr: SocketAddr,
    /// Queue of packets to be sent to the other peer
    primary_queue: (Sender<Packet>, Receiver<Packet>),
    /// Queue of packets received from the other peer
    receive_queue: (Sender<Packet>, Receiver<Packet>),
    /// Queue of packets to be output
    output_queue: (Sender<Packet>, Receiver<Packet>),
    /// [`JoinHandle`] for threads created by [`Link`] module
    thread_handles: Vec<JoinHandle<()>>,
    /// Sequence number for the next packet to be sent
    send_seq: Arc<Mutex<u32>>,
    /// Keeps track of sequence number of received packets [ Not used yet ]
    recv_seq: Arc<Mutex<u32>>,
    /// Flag to indicate if the [`Link`] is currently active or not
    stop_flag: Arc<Mutex<bool>>,
    /// Flag to indicate if the batch queue is empty or not
    batch_empty: Arc<Mutex<bool>>,
    /// Timeout for receiving packets from the other peer
    read_timeout: Option<Duration>,
    /// Current configuration for Aether
    config: Config,
}

impl Link {
    /// Creates a new [`Link`] to another peer
    /// # Arguments
    /// * `id` - [`Id`] of the user that is creating this link
    /// * `socket` - UDP socket used to communicate with the other peer
    /// * `peer_addr` - Address of the other peer
    /// * `peer_id` - Public Id of the other peer
    /// * `send_seq` - Sending Sequence number that the Link needs to be initialised with
    /// * `recv_seq` - Receiving Sequence number that the Link needs to be initialised with
    /// * `config` - Configuration for Aether
    pub fn new(
        id: Id,
        socket: UdpSocket,
        peer_addr: SocketAddr,
        peer_id: PublicId,
        send_seq: u32,
        recv_seq: u32,
        config: Config,
    ) -> Result<Link, AetherError> {
        let socket = Arc::new(socket);

        // if - let for errors
        if socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .is_err()
        {
            return Err(AetherError::SetReadTimeout);
        }

        let primary_queue = unbounded();
        let receive_queue = unbounded();
        let output_queue = unbounded();

        let stop_flag = Arc::new(Mutex::new(false));
        let batch_empty = Arc::new(Mutex::new(false));
        Ok(Link {
            private_id: id,
            ack_list: Arc::new(Mutex::new(AcknowledgementList::new(recv_seq))),
            ack_check: Arc::new(Mutex::new(AcknowledgementCheck::new(send_seq))),
            peer_addr,
            peer_id,
            cipher: None,
            socket,
            primary_queue,
            receive_queue,
            output_queue,
            send_seq: Arc::new(Mutex::new(send_seq)),
            recv_seq: Arc::new(Mutex::new(recv_seq)),
            thread_handles: Vec::new(),
            stop_flag,
            batch_empty,
            read_timeout: None,
            config,
        })
    }

    /// Starts the [`Link`] to the other peer
    pub fn start(&mut self) {
        // Create data structure for the send thread
        let mut send_thread_data = SendThread::new(
            self.socket.clone(),
            self.peer_addr,
            self.primary_queue.1.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
            self.send_seq.clone(),
            self.batch_empty.clone(),
            self.config,
        );

        // Start the send thread
        // Check for arc self if stable : https://stackoverflow.com/questions/25462935/what-types-are-valid-for-the-self-parameter-of-a-method
        let send_thread = thread::spawn(move || {
            send_thread_data.start();
        });

        // Create data strcuture for the receive thread
        let mut recv_thread_data = ReceiveThread::new(
            self.socket.clone(),
            self.peer_addr,
            self.receive_queue.0.clone(),
            self.stop_flag.clone(),
            self.ack_check.clone(),
            self.ack_list.clone(),
            self.recv_seq.clone(),
            self.config,
        );

        // Start the receive thread
        let recv_thread = thread::spawn(move || {
            recv_thread_data.start();
        });

        // Push the threads' join handles to join when stopping the link
        self.thread_handles.push(send_thread);
        self.thread_handles.push(recv_thread);
    }

    pub fn enable_encryption(&mut self) -> Result<(), AetherError> {
        // Generate a secret
        let own_secret = gen_nonce(KEY_SIZE);

        // Encrypt secret with other's public key
        let encrypted_secret = self.peer_id.public_encrypt(&own_secret)?;
        // Send encrypted secret
        let mut packet = Packet::new(PType::KeyExchange, 0);
        packet.append_payload(encrypted_secret);
        self.send_packet(packet)?;

        // Receive encrypted secret
        let other_encrypted = self.recv()?;
        // Decrypt received secret using own private key
        let other_secret = self.private_id.private_decrypt(&other_encrypted)?;

        // XOR received secret with own secret
        let shared_secret = xor(own_secret, other_secret);

        // Instantiate a new cipher with the shared secret
        let cipher = AetherCipher::new(shared_secret);
        let decryption_thread_data = DecryptionThread::new(
            cipher.clone(),
            self.receive_queue.1.clone(),
            self.output_queue.0.clone(),
            self.stop_flag.clone(),
            self.config,
        );

        let decryption_thread = thread::spawn(move || {
            decryption_thread_data.start().unwrap();
        });

        self.thread_handles.push(decryption_thread);

        self.cipher = Some(cipher);

        Ok(())
    }

    pub fn is_encrypted(&self) -> bool {
        self.cipher.is_some()
    }

    /// Stops the [`Link`] to the other peer
    pub fn stop(&mut self) -> Result<(), AetherError> {
        // Set the stop flag
        match self.stop_flag.lock() {
            Ok(mut flag_lock) => {
                *flag_lock = true;

                // Unlock stop flag
                drop(flag_lock);

                // Join each thread
                while match self.thread_handles.pop() {
                    Some(handle) => {
                        handle.join().expect("Thread failed to join");
                        true
                    }
                    None => false,
                } {}
                Ok(())
            }
            Err(_) => Err(AetherError::MutexLock("stop flag")),
        }
    }

    /// Get the [`SocketAddr`] of the peer
    pub fn get_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Sends bytes to the other peer
    /// # Arguments
    /// * `buf` - Buffer containing the bytes to be sent
    pub fn send(&self, buf: Vec<u8>) -> Result<(), AetherError> {
        // Create a new packet to be sent
        let mut packet = Packet::new(PType::Data, 0);
        // if a cipher is present, encrypt the payload
        let data: Vec<u8> = match self.cipher {
            Some(ref cipher) => {
                packet.set_enc(true);
                cipher.encrypt_bytes(buf)?.into()
            }
            None => buf,
        };
        packet.append_payload(data);
        self.send_packet(packet)
    }

    /// Send a `packet` to the other peer
    /// > This alter's the `packet.sequence` number of the `packet` argument. Rest
    /// of the packet is sent as it is
    ///
    /// # Arguments
    ///
    /// * `packet` - The [`Packet`] to be sent
    pub fn send_packet(&self, mut packet: Packet) -> Result<(), AetherError> {
        // Lock seq number
        match self.send_seq.lock() {
            Ok(mut seq_lock) => {
                // Increase sequence number
                (*seq_lock) += 1;

                let seq: u32 = *seq_lock;

                // Unlock seq
                drop(seq_lock);

                // set sequence number on packet
                packet.sequence = seq;

                // Push the new packet onto the primary queue
                self.primary_queue.0.send(packet)?;

                Ok(())
            }
            Err(_) => Err(AetherError::MutexLock("send queue")),
        }
    }

    /// Sets the read timeout for the [`Link`]
    /// # Arguments
    /// * `timeout` - Timeout for receiving packets from the other peer
    pub fn set_read_timout(&mut self, timeout: Duration) {
        self.read_timeout = Some(timeout);
    }

    /// Receive bytes from the other peer or return an error if the timeout is reached
    /// # Arguments
    /// * `timeout` - Timeout to wait for receiving packets
    /// # Returns
    /// * [`Vec<u8>`] - Buffer containing the received bytes
    /// # Errors
    /// * [`AetherError::RecvTimeout`] - Timeout reached before receiving any bytes
    /// * [`AetherError::LinkStopped`] - [`Link`] stopped before receiving any bytes
    ///
    /// Other general errors might occur (refer to [`AetherError`])
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Vec<u8>, AetherError> {
        let receiver = self.get_receiver()?;
        let packet = receiver.recv_timeout(timeout)?;
        Ok(packet.payload)
    }

    /// Receive bytes from the other peer
    /// # Returns
    /// * `Vec<u8>` - Buffer containing the received bytes
    /// # Errors
    /// * [`AetherError::LinkStopped`] - [`Link`] stopped before receiving any bytes
    /// * [`AetherError::LinkTimeout`] - [`Link`] timed out before receiving any bytes
    ///
    /// Other general errors might occur (refer to [`AetherError`])
    pub fn recv(&self) -> Result<Vec<u8>, AetherError> {
        let receiver = self.get_receiver()?;
        let packet = if let Some(time) = self.read_timeout {
            receiver.recv_timeout(time)?
        } else {
            receiver.recv()?
        };

        Ok(packet.payload)
    }

    /// Returns a [`Receiver`] to receive packets from the output queue
    pub fn get_receiver(&self) -> Result<Receiver<Packet>, AetherError> {
        match self.stop_flag.lock() {
            Ok(flag_lock) => {
                let stop = *flag_lock;
                drop(flag_lock);

                if stop {
                    Err(AetherError::LinkStopped("get receiver"))
                } else {
                    // if encrypted receive from output queue
                    if self.is_encrypted() {
                        Ok(self.output_queue.1.clone())
                    } else {
                        // if not encrypted receive directly from receive queue
                        Ok(self.receive_queue.1.clone())
                    }
                }
            }
            Err(_) => Err(AetherError::MutexLock("stop flag")),
        }
    }

    /// Returns true if no more packets needs to be sent
    /// Checks if both primary queue and batch queue are empty
    pub fn is_empty(&self) -> Result<bool, AetherError> {
        if self.primary_queue.0.is_empty() {
            match self.batch_empty.lock() {
                Ok(batch_lock) => Ok(*batch_lock),
                Err(_) => Err(AetherError::MutexLock("batch empty flag")),
            }
        } else {
            Ok(false)
        }
    }

    /// Waits and blocks the current thread until the [`Link`] is empty
    pub fn wait_empty(&self) -> Result<(), AetherError> {
        loop {
            match self.is_empty() {
                Ok(empty) => {
                    if empty {
                        thread::sleep(Duration::from_millis(self.config.link.ack_wait_time));
                        break Ok(());
                    } else {
                        thread::sleep(Duration::from_micros(self.config.link.poll_time_us));
                    }
                }
                Err(aether_error) => {
                    break Err(aether_error);
                }
            }
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        match self.stop() {
            Ok(_) => {}
            Err(aether_error) => {
                log::error!("{}", aether_error)
            }
        }
    }
}
