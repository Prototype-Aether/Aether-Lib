use std::collections::VecDeque;
use std::net::UdpSocket;
use std::sync::Arc;

use crate::acknowledgment::{AcknowledgmentCheck, AcknowledgmentList};
use crate::packet::Packet;

pub struct Link {
    ack_list: Arc<AcknowledgmentList>,
    ack_check: Arc<AcknowledgmentCheck>,
    socket: Arc<UdpSocket>,
    send_thread: SendThread,
    receive_thread: ReceiveThread,
}

struct SendThread {
    batch_queue: VecDeque<Packet>,
    socket: Arc<UdpSocket>,
}

struct ReceiveThread {
    socket: Arc<UdpSocket>,
}

impl Link {
    pub fn new() -> Link {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:7878").unwrap());
        Link {
            ack_list: Arc::new(AcknowledgmentList::new(10)),
            ack_check: Arc::new(AcknowledgmentCheck::new(10)),
            send_thread: SendThread {
                batch_queue: VecDeque::new(),
                socket: socket.clone(),
            },
            receive_thread: ReceiveThread {
                socket: socket.clone(),
            },
            socket,
        }
    }
}
