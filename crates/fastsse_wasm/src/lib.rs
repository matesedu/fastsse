#![allow(missing_docs)]

use fastsse::{
  DecodeError, Decoder, EncodeError, EncodeEvent, Item, encode_comment, encode_event, encode_retry,
};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[wasm_bindgen(js_name = Decoder)]
pub struct JsDecoder {
  inner: Decoder,
  scratch: Vec<u8>,
}

impl Default for JsDecoder {
  fn default() -> Self {
    Self::new()
  }
}

#[wasm_bindgen]
impl JsDecoder {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Decoder::new(),
      scratch: Vec::new(),
    }
  }

  #[wasm_bindgen]
  pub fn push(&mut self, chunk: Uint8Array) -> Result<Array, JsValue> {
    let len = chunk.length() as usize;
    self.scratch.resize(len, 0);
    chunk.copy_to(self.scratch.as_mut_slice());
    let items = Array::new();
    self
      .inner
      .feed(self.scratch.as_slice(), |item| {
        items.push(&item_to_js(item));
      })
      .map_err(decode_error)?;
    Ok(items)
  }

  #[wasm_bindgen(js_name = "pushString")]
  pub fn push_string(&mut self, chunk: &str) -> Result<Array, JsValue> {
    let items = Array::new();
    self
      .inner
      .feed_str(chunk, |item| {
        items.push(&item_to_js(item));
      })
      .map_err(decode_error)?;
    Ok(items)
  }

  #[wasm_bindgen]
  pub fn finish(&mut self) {
    self.inner.finish();
  }

  #[wasm_bindgen]
  pub fn reset(&mut self) {
    self.inner.reset();
  }

  #[wasm_bindgen(getter, js_name = "lastEventId")]
  pub fn last_event_id(&self) -> String {
    self.inner.last_event_id().to_owned()
  }

  #[wasm_bindgen(getter)]
  pub fn retry(&self) -> Option<f64> {
    self.inner.retry().map(|retry| retry as f64)
  }
}

#[wasm_bindgen(js_name = "encodeEvent")]
pub fn encode_event_js(event: JsValue) -> Result<Uint8Array, JsValue> {
  let data = get_required_string(&event, "data")?;
  let event_type = get_optional_string(&event, "event")?;
  let id = get_optional_string(&event, "id")?;
  let retry = get_optional_retry(&event, "retry")?;
  let event = EncodeEvent {
    event: event_type.as_deref(),
    data: data.as_str(),
    id: id.as_deref(),
    retry,
  };
  let encoded = encode_event(&event).map_err(encode_error)?;
  Ok(Uint8Array::from(encoded.as_slice()))
}

#[wasm_bindgen(js_name = "encodeRetry")]
pub fn encode_retry_js(retry: f64) -> Result<Uint8Array, JsValue> {
  let encoded = encode_retry(retry_from_js(retry)?);
  Ok(Uint8Array::from(encoded.as_slice()))
}

#[wasm_bindgen(js_name = "encodeComment")]
pub fn encode_comment_js(comment: &str) -> Uint8Array {
  let encoded = encode_comment(comment);
  Uint8Array::from(encoded.as_slice())
}

fn item_to_js(item: Item<'_>) -> JsValue {
  let object = Object::new();

  match item {
    Item::Event(event) => {
      set(&object, "kind", JsValue::from_str("event"));
      set(&object, "event", JsValue::from_str(event.event));
      set(&object, "data", JsValue::from_str(event.data));
      set(&object, "id", JsValue::from_str(event.id));
    }
    Item::Retry(retry) => {
      set(&object, "kind", JsValue::from_str("retry"));
      set(&object, "retry", JsValue::from_f64(retry as f64));
    }
  }

  object.into()
}

fn set(object: &Object, key: &str, value: JsValue) {
  Reflect::set(object, &JsValue::from_str(key), &value)
    .expect("setting JS object property should not fail");
}

fn get_required_string(value: &JsValue, key: &str) -> Result<String, JsValue> {
  get_optional_string(value, key)?.ok_or_else(|| JsValue::from_str(&format!("{key} is required")))
}

fn get_optional_string(value: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
  let property = Reflect::get(value, &JsValue::from_str(key))?;
  if property.is_undefined() || property.is_null() {
    return Ok(None);
  }

  property
    .as_string()
    .map(Some)
    .ok_or_else(|| JsValue::from_str(&format!("{key} must be a string")))
}

fn get_optional_retry(value: &JsValue, key: &str) -> Result<Option<u64>, JsValue> {
  let property = Reflect::get(value, &JsValue::from_str(key))?;
  if property.is_undefined() || property.is_null() {
    return Ok(None);
  }

  let retry = property
    .as_f64()
    .ok_or_else(|| JsValue::from_str(&format!("{key} must be a number")))?;
  Ok(Some(retry_from_js(retry)?))
}

fn retry_from_js(retry: f64) -> Result<u64, JsValue> {
  if !retry.is_finite() || retry.is_sign_negative() || retry.fract() != 0.0 {
    return Err(JsValue::from_str(
      "retry must be a finite, non-negative integer",
    ));
  }
  if retry > MAX_SAFE_INTEGER {
    return Err(JsValue::from_str(
      "retry must be <= Number.MAX_SAFE_INTEGER",
    ));
  }

  Ok(retry as u64)
}

fn decode_error(error: DecodeError) -> JsValue {
  JsValue::from_str(&error.to_string())
}

fn encode_error(error: EncodeError) -> JsValue {
  JsValue::from_str(&error.to_string())
}
