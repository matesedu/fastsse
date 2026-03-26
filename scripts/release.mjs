import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const ROOT_CARGO_TOML = new URL("../Cargo.toml", import.meta.url);
const NODE_PACKAGE_JSON = new URL("../npm/fastsse_node/package.json", import.meta.url);

const [, , mode, ...flags] = process.argv;

if (!mode || !["patch", "minor", "alpha", "beta"].includes(mode)) {
  console.error("usage: node scripts/release.mjs <patch|minor|alpha|beta> [--dry-run] [--no-tag]");
  process.exit(1);
}

const dryRun = flags.includes("--dry-run");
const noTag = flags.includes("--no-tag");

const cargoToml = readFileSync(ROOT_CARGO_TOML, "utf8");
const nodePackage = JSON.parse(readFileSync(NODE_PACKAGE_JSON, "utf8"));
const currentVersion = readWorkspaceVersion(cargoToml);

if (nodePackage.version !== currentVersion) {
  throw new Error(
    `version mismatch: Cargo.toml=${currentVersion}, npm/fastsse_node=${nodePackage.version}`,
  );
}

if (!noTag) {
  assertGitRepo();
}

const nextVersion = bumpVersion(currentVersion, mode);
const nextCargoToml = cargoToml.replace(/^version = "([^"]+)"$/m, `version = "${nextVersion}"`);
const nextNodePackage = {
  ...nodePackage,
  version: nextVersion,
};

if (dryRun) {
  console.log(JSON.stringify({ currentVersion, nextVersion, noTag }, null, 2));
  process.exit(0);
}

writeFileSync(ROOT_CARGO_TOML, nextCargoToml);
writeFileSync(NODE_PACKAGE_JSON, `${JSON.stringify(nextNodePackage, null, 2)}\n`);

if (!noTag) {
  execFileSync("git", ["tag", "-a", `v${nextVersion}`, "-m", `v${nextVersion}`], {
    stdio: "inherit",
  });
}

console.log(`released ${currentVersion} -> ${nextVersion}`);

function readWorkspaceVersion(toml) {
  const match = toml.match(/^\[workspace\.package\][\s\S]*?^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error("workspace.package.version not found in Cargo.toml");
  }
  return match[1];
}

function bumpVersion(version, release) {
  const parsed = parseSemver(version);

  if (release === "patch") {
    return `${parsed.major}.${parsed.minor}.${parsed.patch + 1}`;
  }
  if (release === "minor") {
    return `${parsed.major}.${parsed.minor + 1}.0`;
  }

  const label = release;
  if (parsed.prerelease?.label === label) {
    return `${parsed.major}.${parsed.minor}.${parsed.patch}-${label}.${parsed.prerelease.number + 1}`;
  }

  const basePatch = parsed.prerelease ? parsed.patch : parsed.patch + 1;
  return `${parsed.major}.${parsed.minor}.${basePatch}-${label}.0`;
}

function parseSemver(version) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+)\.(\d+))?$/);
  if (!match) {
    throw new Error(`unsupported version format: ${version}`);
  }

  return {
    major: Number.parseInt(match[1], 10),
    minor: Number.parseInt(match[2], 10),
    patch: Number.parseInt(match[3], 10),
    prerelease: match[4]
      ? {
          label: match[4],
          number: Number.parseInt(match[5], 10),
        }
      : null,
  };
}

function assertGitRepo() {
  try {
    execFileSync("git", ["rev-parse", "--is-inside-work-tree"], {
      stdio: "ignore",
    });
  } catch {
    throw new Error(
      "release tagging requires a git repository; use --no-tag for dry local testing",
    );
  }
}
