# Cloud environment (managed agents)

Configuration for running this project in an Anthropic managed-agents cloud
environment. These values are entered in the "New cloud environment" dialog;
they live here so the config is version-controlled and reproducible.

## Dialog fields

| Field | Value |
| --- | --- |
| **Name** | `i-lang` (matches the crate name in `Cargo.toml`) |
| **Network access** | `Full` — the setup script bootstraps mise from `mise.run`, which is **not** in the Trusted allowlist (package registries are, but the installer host isn't), so a Trusted environment aborts the script. Full also keeps the door open for ad-hoc fetches during a session. The repo holds no secrets and the env vars below are non-secret, so the main Full-vs-Trusted risk (a compromised dep exfiltrating credentials) doesn't apply here. Revisit if that ever changes. |
| **Environment variables** | `CARGO_TERM_COLOR=always` (mirrors `.github/workflows/ci.yml`). Nothing secret — the dialog warns these are public. Do **not** set `RUSTFLAGS=-D warnings` globally; it would break dev iteration. `make lint` already applies it to clippy. |

## Two-part setup: UI script + repo hook

Cloud setup is split across two places, following the
[docs' guidance](https://code.claude.com/docs/en/claude-code-on-the-web#setup-scripts-vs-sessionstart-hooks)
("setup script for runtimes the laptop already has; SessionStart hook for
project setup like `npm install`"):

| Concern | Where | Why there |
| --- | --- | --- |
| Install mise + pinned toolchain | **UI setup script** (below) | Cloud-only; cached in the environment snapshot so it runs once, not per session |
| `make setup` + warm build | **`scripts/cloud-bootstrap.sh`** via the `SessionStart` hook in `.claude/settings.json` | Version-controlled, runs from the repo root (`$CLAUDE_PROJECT_DIR`), reproducible |

The split also fixes a real failure: a UI **setup script does not start in the
repo checkout and gets no `$CLAUDE_PROJECT_DIR`**, so `make`/`mise install` ran
against an empty directory (`No rule to make target 'setup'`; mise found no
`mise.toml`). The setup script below now locates the checkout explicitly; the
repo hook gets the correct cwd for free.

## Setup script (UI)

Runs once, before Claude Code launches, and is cached. Provisions only the
toolchain from `mise.toml` — project bootstrap moved to the repo hook.

```bash
#!/bin/bash
set -euo pipefail

# Host provisioning: toolchain version + components come from mise.toml.
# (mise everywhere: locally, in CI via jdx/mise-action, and here.)
if ! command -v mise >/dev/null 2>&1; then
  curl https://mise.run | sh
  export PATH="$HOME/.local/bin:$PATH"
fi
eval "$(mise activate bash)"

# A UI setup script doesn't start in the checkout and gets no
# $CLAUDE_PROJECT_DIR, so find the tree holding our mise.toml before reading it.
cd "$(dirname "$(find "$HOME" /root /workspace /repo -maxdepth 5 -name mise.toml 2>/dev/null | head -n1)")"

mise trust                   # a fresh clone's mise.toml is untrusted by default
mise install                 # -> rust 1.95.0 + rustfmt,clippy (from mise.toml)
```

## SessionStart hook (repo)

`scripts/cloud-bootstrap.sh`, wired by `.claude/settings.json`. Does the project
bootstrap (`make setup`) and warms the build. Gated to cloud (the hook also
fires locally, where devs run `make setup` once themselves per the README).

```bash
#!/bin/bash
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

export PATH="$HOME/.local/bin:$PATH"
if command -v mise >/dev/null 2>&1; then
  eval "$(mise activate bash)"
fi

make setup           # npm git hook + cargo fetch
cargo test --no-run  # warm the first `make ci`
```

### Why this shape

- **Toolchain provisioning stays in the UI script**, not a `make` target — a
  local dev already has mise, and a target running `rustup`/`mise install` would
  mutate their global toolchains as a surprise side effect. It also lives in the
  UI (not the repo hook) so it's cached and doesn't reinstall the toolchain on
  every session.
- **Project bootstrap is in the repo hook**, so it's version-controlled and runs
  from the correct cwd. The tradeoff: SessionStart hooks aren't snapshot-cached,
  so `make setup` + warm build re-run each session. They're cheap; if startup
  latency ever bites, move them into the UI setup script (after the `cd`).
- **Neither script runs `make ci`** — the husky pre-commit hook
  (`.husky/pre-commit`) already runs it on every commit.
- **Version + components are read from `mise.toml`**, never re-typed. This is the
  same file CI consumes via `jdx/mise-action`, so local, CI, and cloud share one
  toolchain definition. (Verified: a CI run installed `rustc 1.95.0 (59807616e)`
  — identical to local.)

### Fallback if `mise` is awkward on the base image

Replace the provisioning block with a rustup install that still reads the pin
from `mise.toml` rather than hardcoding it:

```bash
ver="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' mise.toml | head -n1)"
rustup toolchain install "$ver" --component rustfmt clippy
rustup default "$ver"
```
