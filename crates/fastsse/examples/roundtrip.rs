//! Minimal roundtrip example for the Rust API.

use fastsse::{Decoder, EncodeEvent, encode_event};

fn main() {
  let bytes = encode_event(&EncodeEvent {
    event: Some("chat"),
    data: "hello\nworld",
    id: Some("evt-7"),
    retry: Some(1_000),
  })
  .expect("encoding succeeds");

  let mut decoder = Decoder::new();
  decoder
    .feed(&bytes, |item| println!("{item:?}"))
    .expect("decoding succeeds");
}
