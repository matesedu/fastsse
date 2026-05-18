import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT_CARGO_TOML = new URL("../Cargo.toml", import.meta.url);
const ROOT_CARGO_LOCK = new URL("../Cargo.lock", import.meta.url);
const NODE_PACKAGE_JSON = new URL("../npm/fastsse_node/package.json", import.meta.url);
const WASM_PACKAGE_JSON = new URL("../npm/fastsse_wasm/package.json", import.meta.url);
const PACKAGE_JSON_FILES = [
  ["node", NODE_PACKAGE_JSON],
  ["wasm", WASM_PACKAGE_JSON],
];

const [, , mode, ...flags] = process.argv;

if (!mode || !["patch", "minor", "alpha", "beta"].includes(mode)) {
  console.error("usage: node scripts/release.mjs <patch|minor|alpha|beta> [--dry-run] [--no-tag]");
  process.exit(1);
}

const dryRun = flags.includes("--dry-run");
const noTag = flags.includes("--no-tag");

const cargoToml = readFileSync(ROOT_CARGO_TOML, "utf8");
const packageJsons = readPackageJsons(PACKAGE_JSON_FILES);
const currentVersion = readWorkspaceVersion(cargoToml);

for (const packageJson of packageJsons) {
  if (packageJson.contents.version !== currentVersion) {
    throw new Error(
      `version mismatch: Cargo.toml=${currentVersion}, ${packageJson.path}=${packageJson.contents.version}`,
    );
  }
}

if (!noTag) {
  assertGitRepo();
  assertCleanWorkingTree();
}

const nextVersion = bumpVersion(currentVersion, mode);
const nextCargoToml = setWorkspaceFastsseDependencyVersion(
  setWorkspaceVersion(cargoToml, nextVersion),
  nextVersion,
);
const nextPackages = packageJsons.map((packageJson) => ({
  ...packageJson,
  contents: {
    ...packageJson.contents,
    version: nextVersion,
  },
}));

if (dryRun) {
  console.log(
    JSON.stringify(
      {
        currentVersion,
        nextVersion,
        noTag,
        workspaceDependency: `fastsse = "=${nextVersion}"`,
        packages: nextPackages.map(({ name, path }) => ({ name, path })),
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

writeFileSync(ROOT_CARGO_TOML, nextCargoToml);
for (const packageJson of nextPackages) {
  writeFileSync(packageJson.url, `${JSON.stringify(packageJson.contents, null, 2)}\n`);
}
refreshCargoLock();

if (!noTag) {
  const releaseFiles = [
    ROOT_CARGO_TOML,
    ROOT_CARGO_LOCK,
    ...nextPackages.map(({ url }) => url),
  ].filter((url) => existsSync(url) && shouldStageReleaseFile(url));

  execFileSync("git", ["add", ...releaseFiles.map((url) => fileURLToPath(url))], {
    stdio: "inherit",
  });
  execFileSync("git", ["commit", "-m", `chore: release v${nextVersion}`], {
    stdio: "inherit",
  });
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

function readPackageJsons(entries) {
  return entries
    .filter(([, url]) => existsSync(url))
    .map(([name, url]) => ({
      name,
      url,
      path: fileURLToPath(url),
      contents: JSON.parse(readFileSync(url, "utf8")),
    }));
}

function setWorkspaceVersion(toml, version) {
  const nextToml = toml.replace(/^version = "([^"]+)"$/m, `version = "${version}"`);
  if (nextToml === toml) {
    throw new Error("workspace.package.version not found in Cargo.toml");
  }
  return nextToml;
}

function setWorkspaceFastsseDependencyVersion(toml, version) {
  const nextToml = toml.replace(
    /^fastsse = \{ path = "crates\/fastsse", version = "=[^"]+" \}$/m,
    `fastsse = { path = "crates/fastsse", version = "=${version}" }`,
  );
  if (nextToml === toml) {
    throw new Error("workspace.dependencies.fastsse exact version not found in Cargo.toml");
  }
  return nextToml;
}

function refreshCargoLock() {
  execFileSync("cargo", ["metadata", "--format-version=1", "--no-deps"], {
    stdio: "ignore",
  });
}

function shouldStageReleaseFile(url) {
  const path = fileURLToPath(url);
  try {
    execFileSync("git", ["ls-files", "--error-unmatch", path], {
      stdio: "ignore",
    });
    return true;
  } catch {
    try {
      execFileSync("git", ["check-ignore", "-q", path], {
        stdio: "ignore",
      });
      return false;
    } catch {
      return true;
    }
  }
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

function assertCleanWorkingTree() {
  const status = execFileSync("git", ["status", "--porcelain"], {
    encoding: "utf8",
  });
  if (status.trim() !== "") {
    throw new Error("release tagging requires a clean working tree");
  }
}
