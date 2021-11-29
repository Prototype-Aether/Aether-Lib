#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, UdpSocket};
    use std::str::FromStr;

    use aether_lib::link::Link;
    #[test]
    #[ignore]
    pub fn link_test() {
        let peer_addr1 = SocketAddr::from_str("127.0.0.1:8181").unwrap();
        let peer_addr2 = SocketAddr::from_str("127.0.0.1:8282").unwrap();

        let socket1 = UdpSocket::bind(("0.0.0.0", 8181)).unwrap();
        let socket2 = UdpSocket::bind(("0.0.0.0", 8282)).unwrap();

        let mut link1 = Link::new(socket1, peer_addr2, 10, 10);
        let mut link2 = Link::new(socket2, peer_addr1, 10, 10);

        link1.start();
        link2.start();

        let mut data: Vec<Vec<u8>> = Vec::new();

        for i in 1..1000000 {
            data.push(format!("Hello {}", i).as_bytes().to_vec());
        }

        for x in &data {
            link1.send(x.clone());
        }

        let mut count = 0;
        let mut recv: Vec<Vec<u8>> = Vec::new();
        loop {
            match link2.recv() {
                Ok(recved_data) => {
                    count += 1;
                    recv.push(recved_data);
                    if count >= data.len() {
                        break;
                    }
                }
                _ => (),
            }
        }

        for i in 0..recv.len() {
            let a = String::from_utf8(recv[i].clone()).unwrap();
            let b = String::from_utf8(data[i].clone()).unwrap();
            println!("{} == {}", a, b);
            assert_eq!(recv[i], data[i]);
        }

        println!("Stopping");
    }
}
