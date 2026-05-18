//! `fastsse` is a low-allocation SSE codec focused on incremental decoding.
//!
//! The wire format follows the WHATWG HTML Living Standard:
//! <https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream>
//! and
//! <https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream>.

mod decoder;
mod encoder;
mod error;
mod event;

pub use crate::decoder::{Decoder, decode};
pub use crate::encoder::{
  encode_comment, encode_comment_to, encode_event, encode_event_to, encode_retry, encode_retry_to,
  encoded_comment_len, encoded_event_len, encoded_retry_len,
};
pub use crate::error::{DecodeError, EncodeError};
pub use crate::event::{EncodeEvent, Event, Item, OwnedEvent, OwnedItem};
