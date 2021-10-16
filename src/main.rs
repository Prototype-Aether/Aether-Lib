use network_module::PacketQueue;

fn main() {
    let mut q = PacketQueue::new();

    q.send(String::from("Hello"));
    q.send(String::from("Hello again"));
    q.print();
}
