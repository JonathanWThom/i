# Cloud environment (managed agents)

Configuration for running this project in an Anthropic managed-agents cloud
environment. These values are entered in the "New cloud environment" dialog;
they live here so the config is version-controlled and reproducible.

## Dialog fields

| Field | Value |
| --- | --- |
| **Name** | `i-lang` (matches the crate name in `Cargo.toml`) |
| **Network access** | `Trusted` — the setup script needs outbound to crates.io and the npm registry; Trusted allowlists package registries. Bump to Full only if a fetch fails on a registry domain. |
| **Environment variables** | `CARGO_TERM_COLOR=always` (mirrors `.github/workflows/ci.yml`). Nothing secret — the dialog warns these are public. Do **not** set `RUSTFLAGS=-D warnings` globally; it would break dev iteration. `make lint` already applies it to clippy. |

## Setup script

Runs once when a session starts, before Claude Code launches. It provisions the
toolchain from `mise.toml` and reuses `make setup` for project bootstrap — no
values are duplicated from the repo.

```bash
#!/bin/bash
set -euo pipefail

# Host provisioning: toolchain version + components come from mise.toml.
# (mise everywhere: locally, in CI via jdx/mise-action, and in this script.)
if ! command -v mise >/dev/null 2>&1; then
  curl https://mise.run | sh
  export PATH="$HOME/.local/bin:$PATH"
fi
eval "$(mise activate bash)"
mise trust                   # a fresh clone's mise.toml is untrusted by default
mise install                 # -> rust 1.95.0 + rustfmt,clippy (from mise.toml)

# Project bootstrap: install git hook (npm) + fetch deps. Shared with README.
make setup

# Cloud-only: pre-build test artifacts so the first `make ci` is fast.
cargo test --no-run
```

### Why this shape

- **Toolchain provisioning stays in the script**, not a `make` target — a local
  dev already has mise, and a target running `rustup`/`mise install` would
  mutate their global toolchains as a surprise side effect.
- **The script does not run `make ci`** — the husky pre-commit hook
  (`.husky/pre-commit`) already runs it on every commit.
- **Version + components are read from `mise.toml`**, never re-typed here. This
  is the same file CI consumes via `jdx/mise-action`, so local, CI, and cloud
  share one toolchain definition. (Verified: a CI run installed
  `rustc 1.95.0 (59807616e)` — identical to local.)

### Fallback if `mise` is awkward on the base image

Replace the provisioning block with a rustup install that still reads the pin
from `mise.toml` rather than hardcoding it:

```bash
ver="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' mise.toml | head -n1)"
rustup toolchain install "$ver" --component rustfmt clippy
rustup default "$ver"
```
