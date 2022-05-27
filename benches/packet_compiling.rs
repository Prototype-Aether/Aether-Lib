use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use aether_lib::{
    acknowledgement::AcknowledgementList,
    packet::{PType, Packet},
    util::gen_nonce,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let sizes = [0usize, 32, 128, 256, 300, 384, 480, 512, 1024, 2048, 4096];

    let mut group = c.benchmark_group("packet_compiling");
    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let payload = gen_nonce(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &payload, |b, payload| {
            b.iter(|| {
                let mut packet = black_box(Packet::new(PType::Data, 32));
                let mut ack = black_box(AcknowledgementList::new(1000));
                ack.insert(1002);
                ack.insert(1003);
                ack.insert(1005);
                ack.insert(2000);
                packet.add_ack(ack.get());

                packet.set_enc(true);

                packet.append_payload(payload.clone());

                packet.compile()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
