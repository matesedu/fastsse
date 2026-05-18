#![allow(missing_docs)]

use fastsse::{DecodeError, Decoder, DecoderLimits, OwnedEvent, OwnedItem, decode};

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

fn decode_bytewise_with_empty_chunks(input: &[u8]) -> Vec<OwnedItem> {
  let mut decoder = Decoder::new();
  let mut out = Vec::new();
  for byte in input {
    decoder
      .feed_collect(b"", &mut out)
      .expect("empty chunk decodes");
    decoder
      .feed_collect(core::slice::from_ref(byte), &mut out)
      .expect("byte chunk decodes");
  }
  decoder
    .feed_collect(b"", &mut out)
    .expect("empty chunk decodes");
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
fn finish_discards_uncommitted_id_from_incomplete_event() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(b"id: stable\ndata: first\n\n", &mut items)?;
  decoder.feed_collect(b"id: stale\ndata: lost\n", &mut items)?;
  decoder.finish();
  decoder.feed_collect(b"data: next\n\n", &mut items)?;

  assert_eq!(
    items,
    vec![
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "first".into(),
        id: "stable".into(),
      }),
      OwnedItem::Event(OwnedEvent {
        event: "message".into(),
        data: "next".into(),
        id: "stable".into(),
      }),
    ]
  );
  assert_eq!(decoder.last_event_id(), "stable");

  Ok(())
}

#[test]
fn id_only_block_commits_last_event_id() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(b"id: committed\n\n", &mut items)?;
  assert!(items.is_empty());
  assert_eq!(decoder.last_event_id(), "committed");

  decoder.feed_collect(b"data: next\n\n", &mut items)?;
  assert_eq!(
    items,
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "next".into(),
      id: "committed".into(),
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
    assert_eq!(
      decode_with_split(input, split),
      expected,
      "split at {split}"
    );
  }
}

#[test]
fn decoding_is_invariant_for_bytewise_streaming() {
  let input = b"id: x\nretry: 15\ndata: hello\ndata: world\n\n";
  assert_eq!(decode_bytewise(input), decode_ok(input));
}

#[test]
fn empty_chunks_do_not_consume_pending_crlf_lookahead() -> Result<(), DecodeError> {
  let mut decoder = Decoder::new();
  let mut items = Vec::new();

  decoder.feed_collect(b"data: x\r", &mut items)?;
  decoder.feed_collect(b"", &mut items)?;
  decoder.feed_collect(b"\n", &mut items)?;

  assert!(items.is_empty());

  decoder.feed_collect(b"\n", &mut items)?;
  assert_eq!(
    items,
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "x".into(),
      id: "".into(),
    })]
  );

  Ok(())
}

#[test]
fn decoding_is_invariant_for_bytewise_streaming_with_empty_chunks() {
  let input = b"\xEF\xBB\xBFid: x\r\nretry: 15\r\ndata: hello\r\ndata: world\r\n\r\n";
  assert_eq!(decode_bytewise_with_empty_chunks(input), decode_ok(input));
}

#[test]
fn configured_line_limit_rejects_complete_oversized_line() {
  let mut decoder = Decoder::with_limits(DecoderLimits::unbounded().max_line_bytes(6));
  let mut items = Vec::new();

  let err = decoder
    .feed_collect(b"data: x\n\n", &mut items)
    .expect_err("line exceeds configured limit");

  assert_eq!(err.field(), "line");
  assert_eq!(err.problem(), "configured byte limit exceeded");
  assert!(items.is_empty());
}

#[test]
fn configured_line_limit_rejects_partial_oversized_line() {
  let mut decoder = Decoder::with_limits(DecoderLimits::unbounded().max_line_bytes(6));
  let mut items = Vec::new();

  decoder
    .feed_collect(b"data:", &mut items)
    .expect("partial line still under limit");
  let err = decoder
    .feed_collect(b" xy", &mut items)
    .expect_err("partial line exceeds configured limit");

  assert_eq!(err.field(), "line");
  assert!(items.is_empty());
}

#[test]
fn configured_event_limit_rejects_oversized_data_buffer() {
  let mut decoder = Decoder::with_limits(DecoderLimits::unbounded().max_event_bytes(4));
  let mut items = Vec::new();

  decoder
    .feed_collect(b"data: abc\n", &mut items)
    .expect("data plus generated newline fits");
  let err = decoder
    .feed_collect(b"data:\n", &mut items)
    .expect_err("second data line exceeds configured limit");

  assert_eq!(err.field(), "data");
  assert!(items.is_empty());
}
