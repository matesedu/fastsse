import { execFileSync } from "node:child_process";

const allowDirty = process.argv.includes("--allow-dirty");
const commonArgs = ["package", "--locked"];
if (allowDirty) {
  commonArgs.push("--allow-dirty");
}

packageCrate("fastsse");

// The binding crates are publishable through crates.io because their path
// dependency also has an exact version. Before the core crate is published,
// CI still needs a local dry-run, so patch crates.io back to this checkout.
const localFastssePatch = 'patch.crates-io.fastsse.path="crates/fastsse"';
packageCrate("fastsse-node", ["--config", localFastssePatch]);
packageCrate("fastsse-wasm", ["--config", localFastssePatch]);

function packageCrate(crate, extraArgs = []) {
  execFileSync("cargo", [...commonArgs, "-p", crate, ...extraArgs], {
    stdio: "inherit",
  });
}
