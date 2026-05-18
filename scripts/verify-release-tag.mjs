import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const ROOT_CARGO_TOML = new URL("../Cargo.toml", import.meta.url);
const NODE_PACKAGE_JSON = new URL("../npm/fastsse_node/package.json", import.meta.url);
const WASM_PACKAGE_JSON = new URL("../npm/fastsse_wasm/package.json", import.meta.url);
const packageJsonFiles = [NODE_PACKAGE_JSON, WASM_PACKAGE_JSON].filter((url) => existsSync(url));

const version = readWorkspaceVersion(readFileSync(ROOT_CARGO_TOML, "utf8"));
const refName = process.env.GITHUB_REF_NAME;

if (!refName) {
  throw new Error("GITHUB_REF_NAME is required");
}

if (refName !== `v${version}`) {
  throw new Error(`release tag ${refName} does not match workspace version v${version}`);
}

for (const url of packageJsonFiles) {
  const contents = JSON.parse(readFileSync(url, "utf8"));
  if (contents.version !== version) {
    throw new Error(
      `version mismatch: Cargo.toml=${version}, ${fileURLToPath(url)}=${contents.version}`,
    );
  }
}

console.log(`release tag ${refName} matches workspace/npm package version ${version}`);

function readWorkspaceVersion(toml) {
  const match = toml.match(/^\[workspace\.package\][\s\S]*?^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error("workspace.package.version not found in Cargo.toml");
  }
  return match[1];
}
