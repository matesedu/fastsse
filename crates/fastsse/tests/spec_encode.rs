#![allow(missing_docs)]

use fastsse::{
  EncodeEvent, OwnedEvent, OwnedItem, decode, encode_comment, encode_event, encoded_event_len,
};

#[test]
fn encode_exact_length_and_empty_data_match_output() {
  let event = EncodeEvent::message("");
  let encoded = encode_event(&event).expect("encoding succeeds");

  assert_eq!(
    encoded.len(),
    encoded_event_len(&event).expect("length succeeds")
  );
  assert_eq!(encoded, b"data:\n\n");
}

#[test]
fn encode_rejects_invalid_event_and_id_values() {
  assert!(
    encode_event(&EncodeEvent {
      event: Some("bad\nname"),
      data: "ok",
      id: None,
      retry: None,
    })
    .is_err()
  );
  assert!(
    encode_event(&EncodeEvent {
      event: None,
      data: "ok",
      id: Some("bad\0id"),
      retry: None,
    })
    .is_err()
  );
}

#[test]
fn encode_comment_normalizes_all_line_endings() {
  assert_eq!(encode_comment("a\r\nb\rc\n"), b":a\n:b\n:c\n:\n\n");
}

#[test]
fn encode_event_normalizes_data_line_endings_to_lf() {
  let encoded = encode_event(&EncodeEvent::message("a\r\nb\rc\n")).expect("encoding succeeds");

  assert_eq!(encoded, b"data:a\ndata:b\ndata:c\ndata:\n\n");
  assert_eq!(
    decode(&encoded).expect("decoding succeeds"),
    vec![OwnedItem::Event(OwnedEvent {
      event: "message".into(),
      data: "a\nb\nc\n".into(),
      id: "".into(),
    })]
  );
}

#[test]
fn encoded_lines_round_trip_with_preserved_leading_space() {
  let encoded = encode_event(&EncodeEvent {
    event: Some(" update"),
    data: "  payload",
    id: Some(" leading"),
    retry: None,
  })
  .expect("encoding succeeds");

  assert_eq!(
    decode(&encoded).expect("decoding succeeds"),
    vec![OwnedItem::Event(OwnedEvent {
      event: " update".into(),
      data: "  payload".into(),
      id: " leading".into(),
    })]
  );
}
