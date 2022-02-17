#[cfg(test)]
mod tests {

    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        process::Command,
        thread,
    };

    use aether_lib::peer::Aether;

    pub fn run(cmd: &str) {
        let child = Command::new("sh").arg("-c").arg(cmd).spawn().unwrap();
        let output = child.wait_with_output().unwrap();
        println!(
            "{}\n{}",
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap()
        );
    }

    #[test]
    pub fn aether_test() {
        // Run the tracker server
        thread::spawn(|| {
            run("rm -rf tmp && mkdir -p tmp && cd tmp && git clone https://github.com/Prototype-Aether/Aether-Tracker.git");
            run("cd tmp/Aether-Tracker && cargo run --bin server 8000")
        });

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

        let send_str1 = format!("Hello {}", aether2.username);
        aether1
            .send_to(
                &aether2.username,
                String::from(send_str1.clone()).into_bytes(),
            )
            .expect("unable to send to peer");

        let result = aether2
            .recv_from(&aether1.username)
            .expect("Unable to recv");

        let result_str1 = String::from_utf8(result).unwrap();
        println!("Received message: {}", result_str1);

        let send_str2 = format!("Hello {}", aether1.username);
        aether2
            .send_to(
                &aether1.username,
                String::from(send_str2.clone()).into_bytes(),
            )
            .expect("unable to send to peer");

        let result = aether1
            .recv_from(&aether2.username)
            .expect("Unable to recv");

        let result_str2 = String::from_utf8(result).unwrap();
        println!("Received message: {}", result_str2);

        assert_eq!(result_str1, send_str1);
        assert_eq!(result_str2, send_str2);
    }
}
