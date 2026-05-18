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
```

Inspect package contents before publishing:

```bash
pnpm --dir npm/fastsse_node pack --dry-run
pnpm --dir npm/fastsse_wasm pack --dry-run
```

Create the release bump and annotated tag:

```bash
vp run release:patch
```

Push the release commit and tag together after review.
