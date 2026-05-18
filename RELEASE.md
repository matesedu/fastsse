# Release Process

This repository publishes a Rust core crate plus JavaScript packages generated
from the Node and WASM bindings.

## Versioning

- Keep `Cargo.toml` workspace version as the source of truth.
- Keep npm package versions synchronized with the workspace version.
- Use semver for public behavior changes, including Rust API, Node API, WASM API,
  wire-format behavior, and supported platform changes.
- Treat pre-1.0 minor releases as compatibility boundaries.

## Dry Run

Preview a version bump without writing files or creating a tag:

```bash
vp run release:patch -- --dry-run --no-tag
```

Use `minor`, `alpha`, or `beta` for the corresponding release line.

## Release Checklist

Run the full CI-equivalent check from a clean worktree:

```bash
vp install
vp run fmt -- --check
vp run lint
vp run check
vp run test
vp run build:node
vp run build:wasm
vp run audit:supply-chain
vp run package:dry-run
```

Inspect package contents before publishing:

```bash
pnpm run package:dry-run
```

Create the release bump and annotated tag:

```bash
vp run release:patch
```

Push the release commit and tag together after review.

## Supply-chain Policy

Release checks run both advisory and license gates:

```bash
pnpm run audit:supply-chain
```

This runs `cargo deny check`, `pnpm audit --audit-level moderate`, and the npm
license allowlist in `scripts/audit-npm-licenses.mjs`.

Maintain exceptions deliberately:

- Rust advisory exceptions belong in `deny.toml` under `[advisories].ignore`.
  Include the advisory ID, a short comment in the reviewing PR, and a follow-up
  issue for removal.
- Rust license exceptions belong in `deny.toml` under `[licenses].exceptions`
  only after confirming the package, version range, and redistribution impact.
- npm license exceptions belong in `scripts/audit-npm-licenses.mjs` only after
  maintainers confirm the license terms are compatible with publishing public
  npm artifacts.

## MSRV

The minimum supported Rust version is declared in `Cargo.toml` and pinned in the
`msrv` Nix shell. CI keeps latest-stable checks and also runs:

```bash
nix develop .#msrv --command cargo check --locked -p fastsse -p fastsse-node --all-targets
nix develop .#msrv --command cargo check --locked -p fastsse-wasm --target wasm32-unknown-unknown
```

When bumping MSRV, update `workspace.package.rust-version`, `rust-toolchain.toml`,
and `flake.nix` in the same change.

## Trusted Publishing

Tags matching `v*.*.*` run `.github/workflows/release.yml`.

Before the first trusted release, publish the initial crate/package versions
manually if the registry requires bootstrapping, then configure trusted
publishers for:

- crates.io: `fastsse`, `fastsse-node`, and `fastsse-wasm`, restricted to the
  `release.yml` workflow and the `crates-io` environment.
- npm: `@matesinc/fastsse-node` and `@matesinc/fastsse-wasm`, restricted to the
  `release.yml` workflow and the `npm` environment.

The release workflow uses GitHub OIDC instead of long-lived publish tokens,
performs crate and npm package dry-runs before publishing, uploads package
artifacts, and generates GitHub build provenance attestations for the `.crate`
and `.tgz` files.
