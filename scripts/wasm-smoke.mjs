import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const packageDir = new URL("../npm/fastsse_wasm/", import.meta.url);
const dtsUrl = new URL("fastsse.d.ts", packageDir);
const jsUrl = new URL("fastsse.js", packageDir);
const wasmUrl = new URL("fastsse_bg.wasm", packageDir);

async function readBuiltFile(url) {
  try {
    return await readFile(url);
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw new Error(
        `WASM package is missing ${url.pathname}. Run \`pnpm run build:wasm\` before this smoke test.`,
      );
    }
    throw error;
  }
}

function expectDeclaration(dts, expected) {
  assert.ok(dts.includes(expected), `fastsse.d.ts is missing expected declaration: ${expected}`);
}

function rejectDeclaration(dts, unexpected) {
  assert.ok(
    !dts.includes(unexpected),
    `fastsse.d.ts still exposes an untyped public declaration: ${unexpected}`,
  );
}

const dts = await readBuiltFile(dtsUrl).then((buffer) => buffer.toString("utf8"));

expectDeclaration(dts, "export interface EncodeEventInput");
expectDeclaration(dts, "export interface DecodedEvent");
expectDeclaration(dts, "export interface DecodedRetry");
expectDeclaration(dts, "export type DecodedItem = DecodedEvent | DecodedRetry;");
expectDeclaration(dts, "push(chunk: Uint8Array): DecodedItem[];");
expectDeclaration(dts, "pushString(chunk: string): DecodedItem[];");
expectDeclaration(dts, "export function encodeEvent(event: EncodeEventInput): Uint8Array;");
rejectDeclaration(dts, "push(chunk: Uint8Array): Array<any>;");
rejectDeclaration(dts, "pushString(chunk: string): Array<any>;");
rejectDeclaration(dts, "export function encodeEvent(event: any): Uint8Array;");

const [{ default: init, Decoder, encodeComment, encodeEvent, encodeRetry }, wasmBytes] =
  await Promise.all([import(jsUrl.href), readBuiltFile(wasmUrl)]);

await init({ module_or_path: wasmBytes });

const textDecoder = new TextDecoder();
const textEncoder = new TextEncoder();

const encodedEvent = encodeEvent({
  event: "notice",
  data: "hello\nworld",
  id: "evt-1",
  retry: 1500,
});
assert.equal(
  textDecoder.decode(encodedEvent),
  "retry:1500\nid:evt-1\nevent:notice\ndata:hello\ndata:world\n\n",
);

const decoder = new Decoder();
const firstChunkLength = 11;
assert.deepEqual(decoder.push(encodedEvent.slice(0, firstChunkLength)), [
  { kind: "retry", retry: 1500 },
]);
assert.deepEqual(decoder.push(encodedEvent.slice(firstChunkLength)), [
  {
    kind: "event",
    event: "notice",
    data: "hello\nworld",
    id: "evt-1",
  },
]);
assert.equal(decoder.retry, 1500);
assert.equal(decoder.lastEventId, "evt-1");

assert.equal(textDecoder.decode(encodeComment("keep-alive")), ":keep-alive\n\n");
assert.deepEqual(decoder.pushString(":keep-alive\n\n"), []);

const encodedRetry = textDecoder.decode(encodeRetry(2500));
assert.equal(encodedRetry, "retry:2500\n\n");
assert.deepEqual(decoder.push(textEncoder.encode(encodedRetry)), [{ kind: "retry", retry: 2500 }]);
assert.equal(decoder.retry, 2500);

decoder.reset();
assert.equal(decoder.retry, undefined);
assert.equal(decoder.lastEventId, "");
