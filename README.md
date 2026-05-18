# fastsse

`fastsse` is a Server-Sent Events codec with a Rust core and thin Node.js / browser bindings.

## Goals

- Exact line-oriented SSE parsing, including BOM stripping, `CRLF`/`CR`/`LF` handling, `id` persistence, and `retry` control frames.
- Low steady-state allocation pressure in the Rust core.
- Thin target adapters for `napi-rs` and `wasm-bindgen`.

## Specification

- WHATWG HTML, parsing an event stream: <https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream>
- WHATWG HTML, interpreting an event stream: <https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream>

## Workspace layout

- `crates/fastsse`: core Rust encoder/decoder.
- `crates/fastsse_node`: N-API Rust crate for Node.js bindings.
- `crates/fastsse_wasm`: browser-facing `wasm-bindgen` wrapper.
- `npm/fastsse_node`: npm package for the Node.js addon.
- `npm/fastsse_wasm`: generated browser package output from `wasm-pack build`.

## Development

The repository follows the latest `start` stack direction: Nix for the dev shell, `Node.js 24`, and `vp` as the shared task entrypoint.

```bash
nix develop
vp install
vp run fmt
vp run check
vp run test
```

## Release

See [RELEASE.md](RELEASE.md) for the full release checklist and versioning
contract.

```bash
vp run release:patch
vp run release:minor
vp run release:alpha
vp run release:beta
```

Use `-- --dry-run --no-tag` to preview a version bump without writing files or creating a tag.

## Rust usage

```rust
use fastsse::{Decoder, EncodeEvent, encode_event};

let bytes = encode_event(&EncodeEvent {
  event: Some("update"),
  data: "alpha\nbeta",
  id: Some("evt-1"),
  retry: Some(1_500),
})?;

let mut decoder = Decoder::new();
decoder.feed(&bytes, |item| println!("{item:?}"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Node.js usage

```ts
import { Decoder, encodeEvent } from "@matesinc/fastsse-node";

const decoder = new Decoder();
const bytes = encodeEvent({ event: "chat", data: "hello", id: "evt-7" });
console.log(decoder.push(bytes));
```

## Browser usage

```ts
import init, { Decoder, encodeEvent } from "./pkg/fastsse_wasm.js";

await init();

const decoder = new Decoder();
const bytes = encodeEvent({ data: "hello from wasm" });
console.log(decoder.push(bytes));
```
