import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const packageDir = join(rootDir, "npm", "fastsse_wasm");
const packageJsonPath = join(packageDir, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));

for (const file of ["fastsse_bg.wasm", "fastsse.d.ts", "fastsse.js"]) {
  assert.ok(
    existsSync(join(packageDir, file)),
    `${file} is missing; run \`pnpm run build:wasm\` before pack checking`,
  );
}

assert.equal(packageJson.name, "@matesinc/fastsse-wasm");
assert.equal(packageJson.type, "module");
assert.equal(packageJson.main, "fastsse.js");
assert.equal(packageJson.types, "fastsse.d.ts");
assert.equal(packageJson.publishConfig?.access, "public");
assert.equal(packageJson.publishConfig?.provenance, true);

const output = execFileSync("npm", ["pack", "--dry-run", "--json"], {
  cwd: packageDir,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "pipe"],
});
const [packument] = JSON.parse(output);

assert.equal(packument.name, "@matesinc/fastsse-wasm");
assert.equal(packument.version, packageJson.version);
assert.equal(packument.entryCount, packument.files.length);

const actualPaths = packument.files.map((file) => file.path).sort();
const expectedPaths = [
  "LICENSE",
  "README.md",
  "fastsse_bg.wasm",
  "fastsse.d.ts",
  "fastsse.js",
  "package.json",
].sort();

assert.deepEqual(actualPaths, expectedPaths);

const wasmEntry = packument.files.find((file) => file.path === "fastsse_bg.wasm");
assert.ok(wasmEntry?.size > 0, "fastsse_bg.wasm must be present and non-empty");

console.log(
  `WASM npm pack dry-run ok: ${packument.name}@${packument.version} (${actualPaths.length} files)`,
);
