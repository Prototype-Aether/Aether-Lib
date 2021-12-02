#[cfg(test)]
mod tests {

    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        thread,
        time::Duration,
    };

    use aether_lib::peer::Aether;

    #[test]
    pub fn aether_test() {
        let tracker_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let aether1 = Aether::new(String::from("alice"), tracker_addr);

        let aether2 = Aether::new(String::from("bob"), tracker_addr);

        aether1.start();
        aether2.start();

        aether1.connect(String::from("bob"));

        aether2.connect(String::from("alice"));

        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
