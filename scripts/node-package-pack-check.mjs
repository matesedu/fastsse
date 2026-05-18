import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const packageDir = join(rootDir, "npm", "fastsse_node");
const packageJsonPath = join(packageDir, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
const nativeAddonPath = join(packageDir, "fastsse.node");

assert.ok(
  existsSync(nativeAddonPath),
  "fastsse.node is missing; run `pnpm --dir npm/fastsse_node build` before pack checking",
);

assert.notEqual(packageJson.private, true, "package must be publishable");
assert.equal(packageJson.publishConfig?.access, "public");
assert.equal(packageJson.exports?.["."]?.import, "./index.mjs");
assert.equal(packageJson.exports?.["."]?.require, "./index.js");
assert.equal(packageJson.exports?.["."]?.types, "./index.d.ts");

const output = execFileSync("npm", ["pack", "--dry-run", "--json"], {
  cwd: packageDir,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "pipe"],
});
const [packument] = JSON.parse(output);

assert.equal(packument.name, "@matesinc/fastsse-node");
assert.equal(packument.version, packageJson.version);
assert.equal(packument.entryCount, packument.files.length);

const actualPaths = packument.files.map((file) => file.path).sort();
const expectedPaths = [
  "LICENSE",
  "README.md",
  "fastsse.node",
  "index.d.ts",
  "index.js",
  "index.mjs",
  "package.json",
].sort();

assert.deepEqual(actualPaths, expectedPaths);

const nativeFiles = actualPaths.filter((path) => path.endsWith(".node"));
assert.deepEqual(nativeFiles, ["fastsse.node"]);

const nativeEntry = packument.files.find((file) => file.path === "fastsse.node");
assert.ok(nativeEntry?.size > 0, "fastsse.node must be present and non-empty");

console.log(
  `npm pack dry-run ok: ${packument.name}@${packument.version} (${actualPaths.length} files)`,
);
