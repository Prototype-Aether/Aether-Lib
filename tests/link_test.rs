#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
    use std::thread;
    use std::time::Duration;

    use aether_lib::config::Config;
    use aether_lib::identity::{Id, PublicId};
    use aether_lib::link::Link;
    use aether_lib::peer::handshake::handshake;
    #[test]
    pub fn link_test() {
        let socket1 = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
        let socket2 = UdpSocket::bind(("0.0.0.0", 0)).unwrap();

        let mut peer_addr1 = socket1.local_addr().unwrap();
        let mut peer_addr2 = socket2.local_addr().unwrap();

        let id1 = Id::new().unwrap();
        let id2 = Id::new().unwrap();

        let id1_public = PublicId::from_base64(&id1.public_key_to_base64().unwrap()).unwrap();
        let id2_public = PublicId::from_base64(&id2.public_key_to_base64().unwrap()).unwrap();

        peer_addr1.set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        peer_addr2.set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        let mut link1 = Link::new(
            id1,
            socket1,
            peer_addr2,
            id2_public,
            0,
            1000,
            Config::default(),
        )
        .unwrap();
        let mut link2 = Link::new(
            id2,
            socket2,
            peer_addr1,
            id1_public,
            1000,
            0,
            Config::default(),
        )
        .unwrap();

        println!("{:?} {:?}", peer_addr1, peer_addr2);

        link1.start();
        link2.start();

        crossbeam::thread::scope(|s| {
            let handle1 = s.spawn(|_| {
                link1.enable_encryption().unwrap();
            });
            let handle2 = s.spawn(|_| {
                link2.enable_encryption().unwrap();
            });
            handle1.join().unwrap();
            handle2.join().unwrap();
        })
        .unwrap();

        let mut data: Vec<Vec<u8>> = Vec::new();

        for i in 1..100 {
            data.push(format!("Hello {}", i).as_bytes().to_vec());
        }

        for x in &data {
            link1.send(x.clone()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        let mut count = 0;
        let mut recv: Vec<Vec<u8>> = Vec::new();
        loop {
            if let Ok(recved_data) = link2.recv() {
                count += 1;
                recv.push(recved_data);
                if count >= data.len() {
                    break;
                }
            }
        }

        for i in 0..recv.len() {
            let a = String::from_utf8(recv[i].clone()).unwrap();
            let b = String::from_utf8(data[i].clone()).unwrap();
            println!("{} == {}", a, b);
            assert_eq!(recv[i], data[i]);
        }
    }

    #[test]
    pub fn handshake_test() {
        let socket1 = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
        let socket2 = UdpSocket::bind(("0.0.0.0", 0)).unwrap();

        let id1 = Id::new().unwrap();
        let id2 = Id::new().unwrap();

        let uid1 = id1.public_key_to_base64().unwrap();
        let uid2 = id2.public_key_to_base64().unwrap();

        let uid1_clone = uid1.clone();
        let uid2_clone = uid2.clone();

        let peer_addr1 = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            socket1.local_addr().unwrap().port(),
        );
        let peer_addr2 = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            socket2.local_addr().unwrap().port(),
        );

        println!("{:?} {:?}", peer_addr1, peer_addr2);

        let len = 100;

        let send_thread = thread::spawn(move || {
            let link = handshake(
                id1,
                socket1,
                peer_addr2,
                uid1,
                uid2_clone,
                Config::default(),
            )
            .expect("Handshake failed");

            let mut data: Vec<Vec<u8>> = Vec::new();

            for i in 0..len {
                data.push(format!("Hello {}", i).as_bytes().to_vec());
            }

            for x in &data {
                link.send(x.clone()).unwrap();
            }

            link.wait_empty().unwrap();
            println!("Stopping sender");

            data
        });

        let recv_thread = thread::spawn(move || {
            let link = handshake(
                id2,
                socket2,
                peer_addr1,
                uid2,
                uid1_clone,
                Config::default(),
            )
            .expect("Handshake failed");

            let mut count = 0;
            let mut recv: Vec<Vec<u8>> = Vec::new();
            loop {
                match link.recv() {
                    Ok(recved_data) => {
                        count += 1;
                        recv.push(recved_data);
                        if count >= len {
                            break;
                        }
                    }
                    Err(err) => {
                        panic!("Error {}", err);
                    }
                }
            }

            link.wait_empty().unwrap();
            println!("Stopping receiver");
            recv
        });

        let data = send_thread.join().expect("Send thread panicked");
        let recv = recv_thread.join().expect("Receive thread panicked");

        for i in 0..recv.len() {
            assert_eq!(recv[i], data[i]);
        }

        println!("Stopping");
    }
}
