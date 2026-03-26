#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fastsse::{Decoder, EncodeEvent, encode_event};

fn bench_encode(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("encode");
  let event = EncodeEvent {
    event: Some("update"),
    data: "alpha\nbeta\ngamma\ndelta",
    id: Some("evt-42"),
    retry: Some(5_000),
  };

  group.throughput(Throughput::Bytes(event.data.len() as u64));
  group.bench_function("multiline", |bencher| {
    bencher.iter(|| encode_event(&event).expect("encoding succeeds"));
  });
  group.finish();
}

fn bench_decode(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("decode");
  let payload: &[u8] =
    b"id: evt-42\nevent: update\nretry: 5000\ndata: alpha\ndata: beta\ndata: gamma\ndata: delta\n\n";

  group.throughput(Throughput::Bytes(payload.len() as u64));
  group.bench_with_input(BenchmarkId::from_parameter("single-chunk"), &payload, |bencher, input| {
    bencher.iter(|| {
      let mut decoder = Decoder::new();
      decoder
        .feed(input, |_| {})
        .expect("decoding succeeds");
    });
  });
  group.bench_with_input(BenchmarkId::from_parameter("16-byte-chunks"), &payload, |bencher, input| {
    bencher.iter(|| {
      let mut decoder = Decoder::new();
      for chunk in input.chunks(16) {
        decoder.feed(chunk, |_| {}).expect("decoding succeeds");
      }
    });
  });
  group.finish();
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
