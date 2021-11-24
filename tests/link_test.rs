#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::net::{SocketAddr, UdpSocket};
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use aether_lib::link::Link;
    use aether_lib::packet::Packet;
    #[test]
    pub fn link_test() {
        let socket = UdpSocket::bind(("0.0.0.0", 8282)).unwrap();
        let primary_queue = Arc::new(Mutex::new(VecDeque::new()));
        let peer_addr = SocketAddr::from_str("127.0.0.1:8181").unwrap();
        let link = Link::new(socket, peer_addr, primary_queue.clone());

        let mut queue = primary_queue.lock().expect("Unable to lock queue");
        for i in 1..10 {
            let mut packet = Packet::new(32, 32);
            packet.append_payload("Hello".into());
            (*queue).push_back(packet);
        }

        drop(queue);

        link.start();
    }
}
