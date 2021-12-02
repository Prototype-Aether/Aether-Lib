#[cfg(test)]
mod tests {

    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        thread,
        time::Duration,
    };

    use aether_lib::peer::Aether;

    #[test]
    #[ignore]
    pub fn aether_test() {
        let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let aether1 = Aether::new(String::from("alice"), tracker_addr);

        let aether2 = Aether::new(String::from("bob"), tracker_addr);

        aether1.start();
        aether2.start();

        aether1.connect(String::from("bob"));

        aether2.connect(String::from("alice"));

        aether1
            .wait_connection(&aether2.username)
            .expect("couldn't connect");
        aether2
            .wait_connection(&aether1.username)
            .expect("couldn't connect");

        aether1
            .send_to(
                &aether2.username,
                String::from(format!("Hello {}", aether2.username)).into_bytes(),
            )
            .expect("unable to send to peer");

        let result = aether2
            .recv_from(&aether1.username)
            .expect("Unable to recv");
        println!("Received message: {}", String::from_utf8(result).unwrap());

        aether2
            .send_to(
                &aether1.username,
                String::from(format!("Hello {}", aether1.username)).into_bytes(),
            )
            .expect("unable to send to peer");

        let result = aether1
            .recv_from(&aether2.username)
            .expect("Unable to recv");
        println!("Received message: {}", String::from_utf8(result).unwrap());

        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
