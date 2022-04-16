#[cfg(test)]
mod tests {

    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
        process::Command,
        thread,
    };

    use aether_lib::{
        config::Config,
        identity::Id,
        peer::{handshake::handshake, Aether},
    };

    pub fn run(cmd: &str, show_output: bool) {
        let output = if show_output {
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap()
        } else {
            Command::new("sh").arg("-c").arg(cmd).output().unwrap()
        };
        println!(
            "{}\n{}",
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap()
        );
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

    #[test]
    pub fn aether_test() {
        // Run the tracker server
        thread::spawn(|| {
            run("rm -rf tmp && mkdir -p tmp && cd tmp && git clone https://github.com/Prototype-Aether/Aether-Tracker.git", false);
            run(
                "cd tmp/Aether-Tracker && TRACKER_PORT=8000 cargo run --bin server",
                false,
            )
        });

        let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let aether1 = Aether::new_with_id(Id::new().unwrap(), tracker_addr);

        let aether2 = Aether::new_with_id(Id::new().unwrap(), tracker_addr);

        println!("{}\n{}", aether1.get_uid(), aether2.get_uid());

        aether1.start();
        aether2.start();

        aether1.connect(aether2.get_uid());

        aether2.connect(aether1.get_uid());

        aether1
            .wait_connection(aether2.get_uid())
            .expect("couldn't connect");
        aether2
            .wait_connection(aether1.get_uid())
            .expect("couldn't connect");

        let send_str1 = format!("Hello {}", aether2.get_uid());
        aether1
            .send_to(aether2.get_uid(), send_str1.clone().into_bytes())
            .expect("unable to send to peer");

        let result = aether2
            .recv_from(aether1.get_uid())
            .expect("Unable to recv");

        let result_str1 = String::from_utf8(result).unwrap();
        println!("Received message: {}", result_str1);

        let send_str2 = format!("Hello {}", aether1.get_uid());
        aether2
            .send_to(aether1.get_uid(), send_str2.clone().into_bytes())
            .expect("unable to send to peer");

        let result = aether1
            .recv_from(aether2.get_uid())
            .expect("Unable to recv");

        let result_str2 = String::from_utf8(result).unwrap();
        println!("Received message: {}", result_str2);

        assert_eq!(result_str1, send_str1);
        assert_eq!(result_str2, send_str2);
    }
}
