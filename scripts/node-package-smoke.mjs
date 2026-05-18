import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const packageDir = join(rootDir, "npm", "fastsse_node");
const nativeAddonPath = join(packageDir, "fastsse.node");

assert.ok(
  existsSync(nativeAddonPath),
  "fastsse.node is missing; run `pnpm --dir npm/fastsse_node build` before smoke testing",
);

function verifyApi(api, label) {
  assert.equal(typeof api.Decoder, "function", `${label}: Decoder export`);
  assert.equal(typeof api.encodeEvent, "function", `${label}: encodeEvent export`);
  assert.equal(typeof api.encodeRetry, "function", `${label}: encodeRetry export`);

  const encoded = api.encodeEvent({
    event: "chat",
    data: "hello\nworld",
    id: "evt-7",
    retry: 1500,
  });
  assert.ok(Buffer.isBuffer(encoded), `${label}: encodeEvent returns Buffer`);

  const decoder = new api.Decoder();
  assert.deepEqual(decoder.push(encoded), [
    { kind: "retry", retry: 1500 },
    { kind: "event", event: "chat", data: "hello\nworld", id: "evt-7" },
  ]);
  assert.equal(decoder.retry, 1500);
  assert.equal(decoder.lastEventId, "evt-7");

  const stringDecoder = new api.Decoder();
  assert.deepEqual(stringDecoder.pushString("retry: 2500\n\n"), [{ kind: "retry", retry: 2500 }]);
  assert.equal(stringDecoder.retry, 2500);
  assert.deepEqual(stringDecoder.pushString("id: str-1\ndata: from string\n\n"), [
    { kind: "event", event: "message", data: "from string", id: "str-1" },
  ]);
  assert.equal(stringDecoder.lastEventId, "str-1");

  assert.throws(
    () => api.encodeRetry(-1),
    /retry must be a finite, non-negative integer/,
    `${label}: encodeRetry rejects negative values`,
  );
  assert.throws(
    () => api.encodeEvent({ data: "bad", retry: 1.5 }),
    /retry must be a finite, non-negative integer/,
    `${label}: encodeEvent rejects fractional retry values`,
  );
}

const cjsApi = require(join(packageDir, "index.js"));
verifyApi(cjsApi, "CommonJS entrypoint");

const esmApi = await import(pathToFileURL(join(packageDir, "index.mjs")).href);
verifyApi(esmApi, "ESM entrypoint");

const tempDir = mkdtempSync(join(tmpdir(), "fastsse-node-smoke-"));

try {
  const packOutput = execFileSync("npm", ["pack", "--pack-destination", tempDir, "--json"], {
    cwd: packageDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const [packument] = JSON.parse(packOutput);
  const tarball = join(tempDir, packument.filename);
  const projectDir = tempDir;

  execFileSync("npm", ["init", "-y"], {
    cwd: tempDir,
    stdio: "ignore",
  });
  execFileSync("npm", ["install", "--silent", tarball], {
    cwd: tempDir,
    stdio: "inherit",
  });

  writeFileSync(
    join(tempDir, "smoke.cjs"),
    [
      "const assert = require('node:assert/strict');",
      "const { Decoder, encodeEvent } = require('@matesinc/fastsse-node');",
      "const decoder = new Decoder();",
      "const items = decoder.push(encodeEvent({ data: 'installed cjs' }));",
      "assert.deepEqual(items, [{ kind: 'event', event: 'message', data: 'installed cjs', id: '' }]);",
    ].join("\n"),
  );
  writeFileSync(
    join(tempDir, "smoke.mjs"),
    [
      "import assert from 'node:assert/strict';",
      "import { Decoder, encodeEvent } from '@matesinc/fastsse-node';",
      "const decoder = new Decoder();",
      "const items = decoder.push(encodeEvent({ data: 'installed esm' }));",
      "assert.deepEqual(items, [{ kind: 'event', event: 'message', data: 'installed esm', id: '' }]);",
    ].join("\n"),
  );

  execFileSync("node", [join(tempDir, "smoke.cjs")], {
    cwd: projectDir,
    stdio: "inherit",
  });
  execFileSync("node", [join(tempDir, "smoke.mjs")], {
    cwd: projectDir,
    stdio: "inherit",
  });
} finally {
  rmSync(tempDir, { force: true, recursive: true });
}

console.log("Node package smoke ok: CJS, ESM, encode/decode, pushString, retry validation");
