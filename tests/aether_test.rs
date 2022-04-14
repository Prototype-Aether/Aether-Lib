#[cfg(test)]
mod tests {

    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        process::Command,
        thread,
    };

    use aether_lib::{identity::Id, peer::Aether};

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
