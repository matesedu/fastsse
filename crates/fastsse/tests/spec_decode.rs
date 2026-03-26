#![allow(missing_docs)]

use fastsse::{DecodeError, Decoder, OwnedEvent, OwnedItem, decode};

fn decode_ok(input: &[u8]) -> Vec<OwnedItem> {
  decode(input).expect("decoding succeeds")
}

fn decode_with_split(input: &[u8], split: usize) -> Vec<OwnedItem> {
  let mut decoder = Decoder::new();
  let mut out = Vec::new();
  decoder
    .feed_collect(&input[..split], &mut out)
    .expect("first chunk decodes");
  decoder
    .feed_collect(&input[split..], &mut out)
    .expect("second chunk decodes");
  decoder.finish();
  out
}

fn decode_bytewise(input: &[u8]) -> Vec<OwnedItem> {
  let mut decoder = Decoder::new();
  let mut out = Vec::new();
  for byte in input {
    decoder
      .feed_collect(core::slice::from_ref(byte), &mut out)
      .expect("byte chunk decodes");
  }
  decoder.finish();
  out
}

#[test]
fn ignores_comment_only_blocks() {
  assert!(decode_ok(b": keepalive\n\n").is_empty());
}

#[test]
fn matches_spec_four_block_example() {
  let items = decode_ok(
    b": test stream\n\n\
data: first event\n\
id: 1\n\n\
data:second event\n\
id\n\n\
data:  third event\n\n",
  );

  assert_eq!(
    items,
    vec![
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "first event".into(),
        id: "1".into(),
      }),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "second event".into(),
        id: "".into(),
      }),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: " third event".into(),
        id: "".into(),
      }),
    ]
  );
}

#[test]
fn matches_spec_empty_and_newline_data_examples() {
  let items = decode_ok(b"data\n\ndata\ndata\n\ndata:");

  assert_eq!(
    items,
    vec![
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "".into(),
        id: "".into(),
      }),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "\n".into(),
        id: "".into(),
      }),
    ]
  );
}

#[test]
fn trims_only_one_leading_space_after_colon() {
  assert_eq!(decode_ok(b"data:test\n\n"), decode_ok(b"data: test\n\n"));
  assert_eq!(
    decode_ok(b"data:  test\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: " test".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn ignores_unknown_and_case_mismatched_fields() {
  assert_eq!(
    decode_ok(b"Data: no\naaa: still ignored\ndata: ok\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "ok".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn ignores_id_lines_with_nul_and_preserves_previous_id() {
  let items = decode_ok(b"id: one\n\ndata: a\n\nid: tw\0o\ndata: b\n\n");

  assert_eq!(
    items,
    vec![
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "a".into(),
        id: "one".into(),
      }),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "b".into(),
        id: "one".into(),
      }),
    ]
  );
}

#[test]
fn accepts_only_digit_retry_and_ignores_overflow() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(
    b"retry:\nretry: 18446744073709551616\nretry: 0042\n\n",
    &mut items,
  )?;

  assert_eq!(items, vec![OwnedItem::Retry(42)]);
  assert_eq!(decoder.retry(), Some(42));
  Ok(())
}

#[test]
fn event_type_without_data_does_not_leak_into_next_block() {
  assert_eq!(
    decode_ok(b"event: update\n\ndata: ok\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "ok".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn empty_event_field_resolves_to_default_message_type() {
  assert_eq!(
    decode_ok(b"event:\ndata: ok\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "ok".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn data_field_can_dispatch_empty_string() {
  assert_eq!(
    decode_ok(b"data:\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn finish_discards_incomplete_event_and_prepares_for_next_stream() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(b"data: partial", &mut items)?;
  decoder.finish();
  decoder.feed_collect(b"\xEF\xBB\xBFdata: next\n\n", &mut items)?;

  assert_eq!(
    items,
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "next".into(),
      id: "".into(),
    })]
  );
  Ok(())
}

#[test]
fn strips_only_the_first_leading_bom() {
  assert_eq!(
    decode_ok(b"\xEF\xBB\xBFdata: \xEF\xBB\xBFx\n\n"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "\u{FEFF}x".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn decoding_is_invariant_for_all_single_split_points() {
  let input = b"\xEF\xBB\xBFid: x\r\nevent: update\r\ndata: hello\r\ndata: world\r\n\r\n";
  let expected = decode_ok(input);

  for split in 0..=input.len() {
    assert_eq!(decode_with_split(input, split), expected, "split at {split}");
  }
}

#[test]
fn decoding_is_invariant_for_bytewise_streaming() {
  let input = b"id: x\nretry: 15\ndata: hello\ndata: world\n\n";
  assert_eq!(decode_bytewise(input), decode_ok(input));
}
