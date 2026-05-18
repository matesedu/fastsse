//! SSE wire-format encoding following the WHATWG HTML event stream format:
//! <https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream>.

use memchr::memchr2;

use crate::error::EncodeError;
use crate::event::EncodeEvent;

const RETRY_PREFIX: &[u8] = b"retry:";
const ID_PREFIX: &[u8] = b"id:";
const EVENT_PREFIX: &[u8] = b"event:";
const DATA_PREFIX: &[u8] = b"data:";
const COMMENT_PREFIX: &[u8] = b":";

/// Returns the exact encoded byte length for a retry-only block.
#[must_use]
pub fn encoded_retry_len(retry: u64) -> usize {
  RETRY_PREFIX.len() + decimal_len(retry) + 2
}

/// Returns the exact encoded byte length for a comment block.
#[must_use]
pub fn encoded_comment_len(comment: &str) -> usize {
  let mut len = 1;
  for_each_line(comment, |line| {
    len += COMMENT_PREFIX.len() + line.len() + 1;
  });
  len
}

/// Returns the exact encoded byte length for an event block.
///
/// Event payload line endings are normalized: `\r\n`, bare `\r`, and bare `\n`
/// all become separate `data:` lines, which decode back with LF separators.
pub fn encoded_event_len(event: &EncodeEvent<'_>) -> Result<usize, EncodeError> {
  validate_field("event", event.event)?;
  validate_id(event.id)?;

  let mut len = 1;
  if let Some(retry) = event.retry {
    len += RETRY_PREFIX.len() + decimal_len(retry) + 1;
  }
  if let Some(id) = event.id {
    len += ID_PREFIX.len() + id.len() + 1 + usize::from(needs_preserved_leading_space(id));
  }
  if let Some(event_type) = event.event {
    len += EVENT_PREFIX.len()
      + event_type.len()
      + 1
      + usize::from(needs_preserved_leading_space(event_type));
  }
  for_each_line(event.data, |line| {
    len += DATA_PREFIX.len() + line.len() + 1 + usize::from(needs_preserved_leading_space(line));
  });

  Ok(len)
}

/// Encodes a retry-only block.
#[must_use]
pub fn encode_retry(retry: u64) -> Vec<u8> {
  let len = encoded_retry_len(retry);
  let mut out = Vec::with_capacity(len);
  write_retry(retry, &mut out);
  out
}

/// Appends a retry-only block to `out`.
pub fn encode_retry_to(retry: u64, out: &mut Vec<u8>) {
  out.reserve(encoded_retry_len(retry));
  write_retry(retry, out);
}

/// Encodes a comment block.
#[must_use]
pub fn encode_comment(comment: &str) -> Vec<u8> {
  let len = encoded_comment_len(comment);
  let mut out = Vec::with_capacity(len);
  write_comment(comment, &mut out);
  out
}

/// Appends a comment block to `out`.
pub fn encode_comment_to(comment: &str, out: &mut Vec<u8>) {
  out.reserve(encoded_comment_len(comment));
  write_comment(comment, out);
}

/// Encodes an SSE event block into a fresh byte vector.
///
/// Event payload line endings are normalized: `\r\n`, bare `\r`, and bare `\n`
/// all become separate `data:` lines, which decode back with LF separators.
pub fn encode_event(event: &EncodeEvent<'_>) -> Result<Vec<u8>, EncodeError> {
  let len = encoded_event_len(event)?;
  let mut out = Vec::with_capacity(len);
  write_event(event, &mut out);
  Ok(out)
}

/// Appends an SSE event block to `out`.
///
/// Event payload line endings are normalized: `\r\n`, bare `\r`, and bare `\n`
/// all become separate `data:` lines, which decode back with LF separators.
pub fn encode_event_to(event: &EncodeEvent<'_>, out: &mut Vec<u8>) -> Result<(), EncodeError> {
  out.reserve(encoded_event_len(event)?);
  write_event(event, out);

  Ok(())
}

#[inline]
fn write_retry(retry: u64, out: &mut Vec<u8>) {
  out.extend_from_slice(RETRY_PREFIX);
  push_u64(retry, out);
  out.extend_from_slice(b"\n\n");
}

#[inline]
fn write_comment(comment: &str, out: &mut Vec<u8>) {
  for_each_line(comment, |line| {
    out.extend_from_slice(COMMENT_PREFIX);
    out.extend_from_slice(line.as_bytes());
    out.push(b'\n');
  });
  out.push(b'\n');
}

#[inline]
fn write_event(event: &EncodeEvent<'_>, out: &mut Vec<u8>) {
  if let Some(retry) = event.retry {
    write_retry_line(retry, out);
  }
  if let Some(id) = event.id {
    write_field_value(out, ID_PREFIX, id);
  }
  if let Some(event_type) = event.event {
    write_field_value(out, EVENT_PREFIX, event_type);
  }
  for_each_line(event.data, |line| {
    write_field_value(out, DATA_PREFIX, line);
  });
  out.push(b'\n');
}

#[inline]
fn write_retry_line(retry: u64, out: &mut Vec<u8>) {
  out.extend_from_slice(RETRY_PREFIX);
  push_u64(retry, out);
  out.push(b'\n');
}

#[inline]
fn validate_field(name: &'static str, value: Option<&str>) -> Result<(), EncodeError> {
  if let Some(value) = value {
    if value.as_bytes().contains(&b'\0') {
      return Err(EncodeError::new(name, "NUL bytes are not allowed"));
    }
    if contains_line_break(value.as_bytes()) {
      return Err(EncodeError::new(name, "line breaks are not allowed"));
    }
  }

  Ok(())
}

#[inline]
fn validate_id(value: Option<&str>) -> Result<(), EncodeError> {
  validate_field("id", value)
}

#[inline]
fn contains_line_break(bytes: &[u8]) -> bool {
  memchr2(b'\n', b'\r', bytes).is_some()
}

#[inline]
fn decimal_len(mut value: u64) -> usize {
  let mut len = 1;
  while value >= 10 {
    value /= 10;
    len += 1;
  }
  len
}

#[inline]
fn push_u64(mut value: u64, out: &mut Vec<u8>) {
  let mut digits = [0_u8; 20];
  let mut cursor = digits.len();

  loop {
    cursor -= 1;
    digits[cursor] = b'0' + (value % 10) as u8;
    value /= 10;
    if value == 0 {
      break;
    }
  }

  out.extend_from_slice(&digits[cursor..]);
}

#[inline]
fn write_field_value(out: &mut Vec<u8>, prefix: &[u8], value: &str) {
  out.extend_from_slice(prefix);
  if needs_preserved_leading_space(value) {
    out.push(b' ');
  }
  out.extend_from_slice(value.as_bytes());
  out.push(b'\n');
}

#[inline]
fn needs_preserved_leading_space(value: &str) -> bool {
  value.as_bytes().first() == Some(&b' ')
}

fn for_each_line(mut value: &str, mut on_line: impl FnMut(&str)) {
  loop {
    let bytes = value.as_bytes();
    let Some(index) = memchr2(b'\n', b'\r', bytes) else {
      on_line(value);
      break;
    };

    on_line(&value[..index]);
    let mut next = index + 1;
    if bytes[index] == b'\r' && bytes.get(next) == Some(&b'\n') {
      next += 1;
    }
    value = &value[next..];
  }
}
