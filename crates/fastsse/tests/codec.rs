#![allow(missing_docs)]

use fastsse::{
  DecodeError,
  Decoder,
  EncodeEvent,
  Event,
  Item,
  OwnedEvent,
  OwnedItem,
  decode,
  encode_comment,
  encode_event,
  encode_retry,
};
use insta::assert_snapshot;

#[test]
fn encodes_event_snapshot() {
  let payload = encode_event(&EncodeEvent {
    event: Some("chat"),
    data: "hello\nworld",
    id: Some("evt-1"),
    retry: Some(1_500),
  })
  .expect("event encodes");

  assert_snapshot!("encode_event", String::from_utf8(payload).expect("utf8"));
}

#[test]
fn encodes_control_blocks_snapshot() {
  let mut combined = encode_comment("keepalive");
  combined.extend_from_slice(&encode_retry(2_000));

  assert_snapshot!("encode_controls", String::from_utf8(combined).expect("utf8"));
}

#[test]
fn decodes_chunked_stream_and_preserves_last_event_id() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(b"\xEF\xBB", &mut items)?;
  decoder.feed_collect(
    b"\xBFid: one\r\nevent: update\r\ndata: hel",
    &mut items,
  )?;
  decoder.feed_collect(b"lo\r\ndata: world\r\n\r\n", &mut items)?;
  decoder.feed_collect(b"data: next\n\n", &mut items)?;

  let expected = vec![
    OwnedItem::Event(OwnedEvent {
      event: "update".into(),
      data: "hello\nworld".into(),
      id: "one".into(),
    }),
    OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "next".into(),
      id: "one".into(),
    }),
  ];

  assert_eq!(items, expected);
  assert_eq!(decoder.last_event_id(), "one");

  Ok(())
}

#[test]
fn decodes_retry_before_event() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed(b"retry: 2500\ndata: ok\n\n", |item| items.push(item.to_owned()))?;

  assert_eq!(
    items,
    vec![
      OwnedItem::Retry(2_500),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "ok".into(),
        id: "".into(),
      }),
    ]
  );
  assert_eq!(decoder.retry(), Some(2_500));

  Ok(())
}

#[test]
fn ignores_retry_with_non_digits() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed(b"retry: 10ms\ndata: ok\n\n", |item| items.push(item.to_owned()))?;

  assert_eq!(
    items,
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "ok".into(),
      id: "".into(),
    })]
  );
  assert_eq!(decoder.retry(), None);

  Ok(())
}

#[test]
fn replaces_invalid_utf8_in_data() -> Result<(), DecodeError> {
  let items = decode(b"data: \xFF\n\n")?;

  assert_eq!(
    items,
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "\u{FFFD}".into(),
      id: "".into(),
    })]
  );

  Ok(())
}

#[test]
fn borrowed_api_avoids_owned_conversion_until_requested() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut seen = false;

  decoder.feed(b"id: abc\ndata: payload\n\n", |item| match item {
    Item::Event(Event { event, data, id }) => {
      assert_eq!(event, "message");
      assert_eq!(data, "payload");
      assert_eq!(id, "abc");
      seen = true;
    }
    Item::Retry(_) => panic!("unexpected retry"),
  })?;

  assert!(seen);
  Ok(())
}
