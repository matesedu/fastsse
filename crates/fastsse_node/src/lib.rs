#![allow(missing_docs)]

use fastsse::{
  DecodeError, Decoder, EncodeError, EncodeEvent, Item, encode_comment, encode_event, encode_retry,
};
use napi::bindgen_prelude::{Buffer, Result};
use napi::{Error, Status};
use napi_derive::napi;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[napi(object)]
pub struct JsEncodeEvent {
  pub event: Option<String>,
  pub data: String,
  pub id: Option<String>,
  pub retry: Option<f64>,
}

#[napi(object)]
pub struct JsItem {
  pub kind: String,
  pub event: Option<String>,
  pub data: Option<String>,
  pub id: Option<String>,
  pub retry: Option<f64>,
}

#[napi(js_name = "Decoder")]
pub struct JsDecoder {
  inner: Decoder,
}

impl Default for JsDecoder {
  fn default() -> Self {
    Self::new()
  }
}

#[napi]
impl JsDecoder {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Decoder::new(),
    }
  }

  #[napi]
  pub fn push(&mut self, chunk: Buffer) -> Result<Vec<JsItem>> {
    let mut items = Vec::new();
    self
      .inner
      .feed(chunk.as_ref(), |item| items.push(item_to_js(item)))
      .map_err(decode_error)?;
    Ok(items)
  }

  #[napi(js_name = "pushString")]
  pub fn push_string(&mut self, chunk: String) -> Result<Vec<JsItem>> {
    let mut items = Vec::new();
    self
      .inner
      .feed_str(chunk.as_str(), |item| items.push(item_to_js(item)))
      .map_err(decode_error)?;
    Ok(items)
  }

  #[napi]
  pub fn finish(&mut self) {
    self.inner.finish();
  }

  #[napi]
  pub fn reset(&mut self) {
    self.inner.reset();
  }

  #[napi(getter, js_name = "lastEventId")]
  pub fn last_event_id(&self) -> String {
    self.inner.last_event_id().to_owned()
  }

  #[napi(getter)]
  pub fn retry(&self) -> Option<f64> {
    self.inner.retry().map(|retry| retry as f64)
  }
}

#[napi(js_name = "encodeEvent")]
pub fn encode_event_js(event: JsEncodeEvent) -> Result<Buffer> {
  let event = js_encode_event_to_rust(&event)?;
  encode_event(&event).map(Buffer::from).map_err(encode_error)
}

#[napi(js_name = "encodeRetry")]
pub fn encode_retry_js(retry: f64) -> Result<Buffer> {
  let retry = retry_from_js(retry)?;
  Ok(Buffer::from(encode_retry(retry)))
}

#[napi(js_name = "encodeComment")]
pub fn encode_comment_js(comment: String) -> Buffer {
  Buffer::from(encode_comment(comment.as_str()))
}

fn js_encode_event_to_rust(event: &JsEncodeEvent) -> Result<EncodeEvent<'_>> {
  Ok(EncodeEvent {
    event: event.event.as_deref(),
    data: event.data.as_str(),
    id: event.id.as_deref(),
    retry: transpose_retry(event.retry)?,
  })
}

fn item_to_js(item: Item<'_>) -> JsItem {
  match item {
    Item::Event(event) => JsItem {
      kind: String::from("event"),
      event: Some(event.event.to_owned()),
      data: Some(event.data.to_owned()),
      id: Some(event.id.to_owned()),
      retry: None,
    },
    Item::Retry(retry) => JsItem {
      kind: String::from("retry"),
      event: None,
      data: None,
      id: None,
      retry: Some(retry as f64),
    },
  }
}

fn transpose_retry(retry: Option<f64>) -> Result<Option<u64>> {
  retry.map(retry_from_js).transpose()
}

fn retry_from_js(retry: f64) -> Result<u64> {
  if !retry.is_finite() || retry.is_sign_negative() || retry.fract() != 0.0 {
    return Err(Error::new(
      Status::InvalidArg,
      String::from("retry must be a finite, non-negative integer"),
    ));
  }
  if retry > MAX_SAFE_INTEGER {
    return Err(Error::new(
      Status::InvalidArg,
      String::from("retry must be <= Number.MAX_SAFE_INTEGER"),
    ));
  }

  Ok(retry as u64)
}

fn decode_error(error: DecodeError) -> Error {
  Error::new(Status::InvalidArg, error.to_string())
}

fn encode_error(error: EncodeError) -> Error {
  Error::new(Status::InvalidArg, error.to_string())
}
