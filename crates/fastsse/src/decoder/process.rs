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
        process_line(line, &mut self.process_state(), emit)?;
      } else {
        self.line.extend_from_slice(line);
        let mut state = ProcessState {
          data: &mut self.data,
          event: &mut self.event,
          last_event_id: &mut self.last_event_id,
          pending_last_event_id: &mut self.pending_last_event_id,
          has_pending_last_event_id: &mut self.has_pending_last_event_id,
          retry: &mut self.retry,
        };
        process_line(self.line.as_slice(), &mut state, emit)?;
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

  fn process_state(&mut self) -> ProcessState<'_> {
    ProcessState {
      data: &mut self.data,
      event: &mut self.event,
      last_event_id: &mut self.last_event_id,
      pending_last_event_id: &mut self.pending_last_event_id,
      has_pending_last_event_id: &mut self.has_pending_last_event_id,
      retry: &mut self.retry,
    }
  }
}

struct ProcessState<'a> {
  data: &'a mut Vec<u8>,
  event: &'a mut String,
  last_event_id: &'a mut String,
  pending_last_event_id: &'a mut String,
  has_pending_last_event_id: &'a mut bool,
  retry: &'a mut Option<u64>,
}

fn process_line<F>(line: &[u8], state: &mut ProcessState<'_>, emit: &mut F) -> Result<(), DecodeError>
where
  F: for<'event> FnMut(Item<'event>),
{
  if line.is_empty() {
    return dispatch_event(state, emit);
  }
  if line[0] == b':' {
    return Ok(());
  }

  let (field, value) = split_field(line);

  if field == b"data" {
    state.data.extend_from_slice(value);
    state.data.push(b'\n');
    return Ok(());
  }
  if field == b"event" {
    state.event.clear();
    state.event.push_str(decode_utf8(value).as_ref());
    return Ok(());
  }
  if field == b"id" {
    if !contains_nul(value) {
      state.pending_last_event_id.clear();
      state.pending_last_event_id.push_str(decode_utf8(value).as_ref());
      *state.has_pending_last_event_id = true;
    }
    return Ok(());
  }
  if field == b"retry" && let Some(parsed) = parse_retry(value) {
    *state.retry = Some(parsed);
    emit(Item::Retry(parsed));
  }

  Ok(())
}

fn dispatch_event<F>(
  state: &mut ProcessState<'_>,
  emit: &mut F,
) -> Result<(), DecodeError>
where
  F: for<'event> FnMut(Item<'event>),
{
  if *state.has_pending_last_event_id {
    state.last_event_id.clear();
    state
      .last_event_id
      .push_str(state.pending_last_event_id.as_str());
    state.pending_last_event_id.clear();
    *state.has_pending_last_event_id = false;
  }

  if state.data.is_empty() {
    state.event.clear();
    return Ok(());
  }

  let data_len = state.data.len() - 1;
  let data_storage;
  let data = match decode_utf8(&state.data[..data_len]) {
    Cow::Borrowed(data) => data,
    Cow::Owned(owned) => {
      data_storage = owned;
      data_storage.as_str()
    }
  };
  let event_name = if state.event.is_empty() {
    "message"
  } else {
    state.event.as_str()
  };

  emit(Item::Event(Event {
    event: event_name,
    data,
    id: state.last_event_id.as_str(),
  }));

  state.data.clear();
  state.event.clear();
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
