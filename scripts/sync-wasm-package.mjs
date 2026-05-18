import { existsSync, readFileSync, writeFileSync } from "node:fs";

const ROOT_CARGO_TOML = new URL("../Cargo.toml", import.meta.url);
const WASM_PACKAGE_JSON = new URL("../npm/fastsse_wasm/package.json", import.meta.url);

if (!existsSync(WASM_PACKAGE_JSON)) {
  throw new Error("npm/fastsse_wasm/package.json does not exist; run build:wasm first");
}

const version = readWorkspaceVersion(readFileSync(ROOT_CARGO_TOML, "utf8"));
const packageJson = JSON.parse(readFileSync(WASM_PACKAGE_JSON, "utf8"));

const nextPackageJson = {
  ...packageJson,
  name: "@matesinc/fastsse-wasm",
  version,
  description: "Browser bindings for the fastsse Server-Sent Events codec.",
  keywords: ["rust", "server-sent-events", "sse", "wasm"],
  license: "GPL-3.0-or-later",
  author: "Mates Inc.",
  repository: {
    type: "git",
    url: "https://github.com/matesedu/fastsse",
    directory: "npm/fastsse_wasm",
  },
  bugs: {
    url: "https://github.com/matesedu/fastsse/issues",
  },
  homepage: "https://github.com/matesedu/fastsse#readme",
  publishConfig: {
    access: "public",
    provenance: true,
  },
};

writeFileSync(WASM_PACKAGE_JSON, `${JSON.stringify(nextPackageJson, null, 2)}\n`);

function readWorkspaceVersion(toml) {
  const match = toml.match(/^\[workspace\.package\][\s\S]*?^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error("workspace.package.version not found in Cargo.toml");
  }
  return match[1];
}
