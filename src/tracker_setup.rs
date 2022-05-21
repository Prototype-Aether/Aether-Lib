use std::{process::Command, sync::Once, thread};

static INIT: Once = Once::new();

#[doc(hidden)]
fn run(cmd: &str, show_output: bool) {
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

#[doc(hidden)]
pub fn tracker_setup() {
    // need to only setup the tracker once
    INIT.call_once(|| {
        thread::spawn(|| {
            run("mkdir -p tmp", false);
            run("curl -L https://github.com/Prototype-Aether/Aether-Tracker/releases/latest/download/aether-tracker-server-x86_64-unknown-linux-gnu --output tmp/aether-tracker-server", false);
            run("chmod +x tmp/aether-tracker-server", false);
            run("TRACKER_PORT=8000 tmp/aether-tracker-server", false);
        });
    });
}
