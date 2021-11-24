use crossbeam;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::packet::Packet;

pub struct Link {
    ack_list: Arc<Mutex<AcknowledgmentList>>,
    ack_check: Arc<Mutex<AcknowledgmentCheck>>,
    socket: Arc<UdpSocket>,
    send_thread: Option<thread::JoinHandle<()>>,
    receive_thread: Option<thread::JoinHandle<()>>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
}

struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    primary_queue: Arc<Mutex<VecDeque<Packet>>>,
}

struct ReceiveThread {
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
}

impl Link {
    pub fn new(
        socket: UdpSocket,
        peer_addr: SocketAddr,
        primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    ) -> Link {
        let socket = Arc::new(socket);
        Link {
            ack_list: Arc::new(Mutex::new(AcknowledgmentList::new(10))),
            ack_check: Arc::new(Mutex::new(AcknowledgmentCheck::new(10))),
            peer_addr,
            send_thread: None,
            receive_thread: None,
            socket,
            primary_queue,
        }
    }

    pub fn start(&self) {
        let send_thread_data = SendThread::new(
            self.socket.clone(),
            self.peer_addr.clone(),
            self.primary_queue.clone(),
        );

        let send_thread = thread::spawn(move || {
            send_thread_data.start();
        });

        let recv_thread_data = ReceiveThread::new(self.socket.clone(), self.peer_addr.clone());

        let recv_thread = thread::spawn(move || {
            recv_thread_data.start();
        });

        send_thread.join().expect("Send thread didn't join");
        recv_thread.join().expect("Receive thread didn't join");
    }
}

impl SendThread {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        primary_queue: Arc<Mutex<VecDeque<Packet>>>,
    ) -> SendThread {
        SendThread {
            batch_queue: VecDeque::new(),
            socket,
            peer_addr,
            primary_queue,
        }
    }

    pub fn start(&self) {
        println!("Starting send thread...");
        loop {
            let mut queue = self.primary_queue.lock().expect("Error locking queue");
            match (*queue).pop_front() {
                Some(packet) => self.send(packet),
                None => (),
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    pub fn send(&self, packet: Packet) {
        let data = packet.compile();
        self.socket
            .send_to(&data, self.peer_addr)
            .expect("Unable to send data");
    }
}

impl ReceiveThread {
    pub fn new(socket: Arc<UdpSocket>, peer_addr: SocketAddr) -> ReceiveThread {
        ReceiveThread { socket, peer_addr }
    }

    pub fn start(&self) {
        let mut buf = [0; 512];
        println!("Starting receive thread...");
        loop {
            let (size, addr) = self.socket.recv_from(&mut buf).expect("Receive error");
            let message = String::from_utf8(buf[..size].to_vec()).expect("Cannot decode string");
            println!("{}", message);
        }
    }
}
