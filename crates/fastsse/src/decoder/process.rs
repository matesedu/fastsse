use std::borrow::Cow;

use memchr::{memchr, memchr2};

use crate::decoder::{Decoder, contains_nul};
use crate::error::DecodeError;
use crate::event::{Event, Item};

impl Decoder {
  pub(super) fn process_input<F>(&mut self, mut chunk: &[u8], emit: &mut F) -> Result<(), DecodeError>
  where
    F: for<'event> FnMut(Item<'event>),
  {
    if self.skip_next_lf {
      if chunk.is_empty() {
        return Ok(());
      }
      self.skip_next_lf = false;
      if chunk.first() == Some(&b'\n') {
        chunk = &chunk[1..];
      }
    }

    while !chunk.is_empty() {
      let Some(line_end) = memchr2(b'\n', b'\r', chunk) else {
        self.line.extend_from_slice(chunk);
        return Ok(());
      };

      let terminator = chunk[line_end];
      let line = &chunk[..line_end];

      if self.line.is_empty() {
        process_line(
          line,
          &mut self.data,
          &mut self.event,
          &mut self.last_event_id,
          &mut self.retry,
          emit,
        )?;
      } else {
        self.line.extend_from_slice(line);
        process_line(
          self.line.as_slice(),
          &mut self.data,
          &mut self.event,
          &mut self.last_event_id,
          &mut self.retry,
          emit,
        )?;
        self.line.clear();
      }

      let mut advance = 1;
      if terminator == b'\r' {
        if chunk.get(line_end + 1) == Some(&b'\n') {
          advance = 2;
        } else if line_end + 1 == chunk.len() {
          self.skip_next_lf = true;
        }
      }

      chunk = &chunk[line_end + advance..];
    }

    Ok(())
  }
}

fn process_line<F>(
  line: &[u8],
  data: &mut Vec<u8>,
  event: &mut String,
  last_event_id: &mut String,
  retry: &mut Option<u64>,
  emit: &mut F,
) -> Result<(), DecodeError>
where
  F: for<'event> FnMut(Item<'event>),
{
  if line.is_empty() {
    return dispatch_event(data, event, last_event_id.as_str(), emit);
  }
  if line[0] == b':' {
    return Ok(());
  }

  let (field, value) = split_field(line);

  if field == b"data" {
    data.extend_from_slice(value);
    data.push(b'\n');
    return Ok(());
  }
  if field == b"event" {
    event.clear();
    event.push_str(decode_utf8(value).as_ref());
    return Ok(());
  }
  if field == b"id" {
    if !contains_nul(value) {
      last_event_id.clear();
      last_event_id.push_str(decode_utf8(value).as_ref());
    }
    return Ok(());
  }
  if field == b"retry" && let Some(parsed) = parse_retry(value) {
    *retry = Some(parsed);
    emit(Item::Retry(parsed));
  }

  Ok(())
}

fn dispatch_event<F>(
  data_buffer: &mut Vec<u8>,
  event_buffer: &mut String,
  last_event_id: &str,
  emit: &mut F,
) -> Result<(), DecodeError>
where
  F: for<'event> FnMut(Item<'event>),
{
  if data_buffer.is_empty() {
    event_buffer.clear();
    return Ok(());
  }

  let data_len = data_buffer.len() - 1;
  let data_storage;
  let data = match decode_utf8(&data_buffer[..data_len]) {
    Cow::Borrowed(data) => data,
    Cow::Owned(owned) => {
      data_storage = owned;
      data_storage.as_str()
    }
  };
  let event_name = if event_buffer.is_empty() {
    "message"
  } else {
    event_buffer.as_str()
  };

  emit(Item::Event(Event {
    event: event_name,
    data,
    id: last_event_id,
  }));

  data_buffer.clear();
  event_buffer.clear();
  Ok(())
}

#[inline]
fn split_field(line: &[u8]) -> (&[u8], &[u8]) {
  match memchr(b':', line) {
    Some(index) => {
      let mut value = &line[index + 1..];
      if value.first() == Some(&b' ') {
        value = &value[1..];
      }
      (&line[..index], value)
    }
    None => (line, &[][..]),
  }
}

#[inline]
fn parse_retry(value: &[u8]) -> Option<u64> {
  if value.is_empty() {
    return None;
  }

  let mut parsed = 0_u64;
  for byte in value {
    if !byte.is_ascii_digit() {
      return None;
    }
    parsed = parsed.checked_mul(10)?;
    parsed = parsed.checked_add(u64::from(byte - b'0'))?;
  }

  Some(parsed)
}

#[inline]
fn decode_utf8(bytes: &[u8]) -> Cow<'_, str> {
  String::from_utf8_lossy(bytes)
}
