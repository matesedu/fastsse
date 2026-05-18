//! Incremental SSE decoding following the WHATWG HTML event stream parsing rules:
//! <https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream>.

use memchr::memchr;

use crate::error::DecodeError;
use crate::event::{Item, OwnedItem};

mod process;

const UTF8_BOM: &[u8; 3] = b"\xEF\xBB\xBF";

/// Incremental SSE decoder.
#[derive(Clone, Debug)]
pub struct Decoder {
  line: Vec<u8>,
  data: Vec<u8>,
  event: String,
  last_event_id: String,
  bom_prefix: [u8; 3],
  bom_len: u8,
  skip_next_lf: bool,
  bom_resolved: bool,
  retry: Option<u64>,
}

impl Default for Decoder {
  fn default() -> Self {
    Self {
      line: Vec::with_capacity(256),
      data: Vec::with_capacity(1024),
      event: String::with_capacity(32),
      last_event_id: String::with_capacity(64),
      bom_prefix: [0; 3],
      bom_len: 0,
      skip_next_lf: false,
      bom_resolved: false,
      retry: None,
    }
  }
}

impl Decoder {
  /// Creates a fresh decoder.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns the last accepted `retry:` value.
  #[must_use]
  pub const fn retry(&self) -> Option<u64> {
    self.retry
  }

  /// Returns the effective last-event-id buffer.
  #[must_use]
  pub fn last_event_id(&self) -> &str {
    self.last_event_id.as_str()
  }

  /// Drops any in-flight partial block while preserving `retry` and `last_event_id`.
  ///
  /// This matches the HTML spec's end-of-file rule: an incomplete trailing event is discarded,
  /// and the decoder is prepared for a subsequent stream, including stripping a fresh leading BOM.
  pub fn finish(&mut self) {
    self.line.clear();
    self.data.clear();
    self.event.clear();
    self.bom_len = 0;
    self.skip_next_lf = false;
    self.bom_resolved = false;
  }

  /// Resets the entire decoder state.
  pub fn reset(&mut self) {
    self.finish();
    self.last_event_id.clear();
    self.retry = None;
  }

  /// Feeds a UTF-8 string chunk into the decoder.
  pub fn feed_str<F>(&mut self, chunk: &str, emit: F) -> Result<(), DecodeError>
  where
    F: for<'event> FnMut(Item<'event>),
  {
    self.feed(chunk.as_bytes(), emit)
  }

  /// Feeds a byte chunk into the decoder.
  pub fn feed<'chunk, F>(&mut self, mut chunk: &'chunk [u8], mut emit: F) -> Result<(), DecodeError>
  where
    F: for<'event> FnMut(Item<'event>),
  {
    if !self.bom_resolved {
      chunk = self.resolve_bom(chunk, &mut emit)?;
      if chunk.is_empty() {
        return Ok(());
      }
    }

    self.process_input(chunk, &mut emit)
  }

  /// Feeds a byte chunk and collects owned items into `out`.
  pub fn feed_collect(
    &mut self,
    chunk: &[u8],
    out: &mut Vec<OwnedItem>,
  ) -> Result<(), DecodeError> {
    self.feed(chunk, |item| out.push(item.to_owned()))
  }

  fn resolve_bom<'chunk, F>(
    &mut self,
    mut chunk: &'chunk [u8],
    emit: &mut F,
  ) -> Result<&'chunk [u8], DecodeError>
  where
    F: for<'event> FnMut(Item<'event>),
  {
    while !self.bom_resolved {
      let Some((&first, rest)) = chunk.split_first() else {
        return Ok(chunk);
      };

      self.bom_prefix[self.bom_len as usize] = first;
      self.bom_len += 1;
      chunk = rest;

      match self.bom_len {
        1 if self.bom_prefix[0] != UTF8_BOM[0] => {
          self.bom_resolved = true;
          let prefix = self.bom_prefix;
          self.process_input(&prefix[..1], emit)?;
          return Ok(chunk);
        }
        2 if self.bom_prefix[1] != UTF8_BOM[1] => {
          self.bom_resolved = true;
          let prefix = self.bom_prefix;
          self.process_input(&prefix[..2], emit)?;
          return Ok(chunk);
        }
        3 => {
          self.bom_resolved = true;
          if &self.bom_prefix != UTF8_BOM {
            let prefix = self.bom_prefix;
            self.process_input(&prefix, emit)?;
          }
          self.bom_len = 0;
          return Ok(chunk);
        }
        _ => {}
      }
    }

    Ok(chunk)
  }
}

/// Decodes a complete byte slice into owned items.
pub fn decode(input: &[u8]) -> Result<Vec<OwnedItem>, DecodeError> {
  let mut decoder = Decoder::new();
  let mut out = Vec::new();
  decoder.feed_collect(input, &mut out)?;
  decoder.finish();
  Ok(out)
}

#[inline]
fn contains_nul(bytes: &[u8]) -> bool {
  memchr(b'\0', bytes).is_some()
}
