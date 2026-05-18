# @matesinc/fastsse-node

Node.js bindings for the `fastsse` Server-Sent Events codec.

This package is intentionally publishable as a public scoped npm package. It
ships a prebuilt N-API addon in `fastsse.node`; it does not compile from source
during consumer installs and does not require a repository checkout. Release
automation must build the addon for the target platform, run the smoke test, and
run the dry-run pack assertions before publishing that tarball.

## ESM

```js
import { Decoder, encodeEvent } from "@matesinc/fastsse-node";

const decoder = new Decoder();
const bytes = encodeEvent({ event: "chat", data: "hello", id: "evt-7" });

console.log(decoder.push(bytes));
```

## CommonJS

```js
const { Decoder, encodeEvent } = require("@matesinc/fastsse-node");

const decoder = new Decoder();
const bytes = encodeEvent({ event: "chat", data: "hello", id: "evt-7" });

console.log(decoder.push(bytes));
```
