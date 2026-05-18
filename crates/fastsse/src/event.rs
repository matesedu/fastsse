use compact_str::CompactString;

/// A decoded SSE event borrowing from decoder-owned buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event<'a> {
  /// Event type. Defaults to `"message"`.
  pub event: &'a str,
  /// Event payload with protocol line folding removed.
  pub data: &'a str,
  /// Effective last-event-id after processing the current block.
  pub id: &'a str,
}

impl Event<'_> {
  /// Copies this borrowed event into an owned representation.
  #[must_use]
  pub fn to_owned(self) -> OwnedEvent {
    OwnedEvent {
      event: CompactString::new(self.event),
      data: self.data.to_owned(),
      id: CompactString::new(self.id),
    }
  }
}

/// An SSE event ready for encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeEvent<'a> {
  /// Optional event type. Omit to use the default `"message"` dispatch type.
  pub event: Option<&'a str>,
  /// Required event payload.
  ///
  /// The encoder writes one `data:` field per logical line. `\r\n`, bare `\r`,
  /// and bare `\n` are all normalized to LF-separated SSE data lines on the wire.
  pub data: &'a str,
  /// Optional event identifier. Use `Some("")` to reset the id.
  pub id: Option<&'a str>,
  /// Optional reconnection delay in milliseconds.
  pub retry: Option<u64>,
}

impl<'a> EncodeEvent<'a> {
  /// Creates a default `"message"` event.
  #[must_use]
  pub const fn message(data: &'a str) -> Self {
    Self {
      event: None,
      data,
      id: None,
      retry: None,
    }
  }
}

/// An owned decoded SSE event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedEvent {
  /// Event type. Defaults to `"message"`.
  pub event: CompactString,
  /// Event payload.
  pub data: String,
  /// Effective last-event-id after processing the current block.
  pub id: CompactString,
}

impl OwnedEvent {
  /// Returns a borrowed view of this event.
  #[must_use]
  pub fn as_event(&self) -> Event<'_> {
    Event {
      event: self.event.as_str(),
      data: self.data.as_str(),
      id: self.id.as_str(),
    }
  }
}

/// A decoded output item from the SSE stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item<'a> {
  /// A dispatched SSE event.
  Event(Event<'a>),
  /// A valid `retry:` control field.
  Retry(u64),
}

impl Item<'_> {
  /// Copies this output item into an owned representation.
  #[must_use]
  pub fn to_owned(self) -> OwnedItem {
    match self {
      Self::Event(event) => OwnedItem::Event(event.to_owned()),
      Self::Retry(retry) => OwnedItem::Retry(retry),
    }
  }
}

/// An owned output item from the SSE stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnedItem {
  /// A dispatched SSE event.
  Event(OwnedEvent),
  /// A valid `retry:` control field.
  Retry(u64),
}
