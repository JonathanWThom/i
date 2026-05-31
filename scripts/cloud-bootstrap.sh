#!/bin/bash
# Project bootstrap for cloud sessions: wire the git hook, fetch deps, warm the
# build. This is the version-controlled half of cloud setup; the UI setup script
# only provisions the toolchain (mise + rust). Runs via the SessionStart hook in
# .claude/settings.json — which fires both locally and in the cloud, so gate to
# cloud: local devs run `make setup` once themselves (see README).
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# mise is installed globally by the UI setup script; ensure it's on PATH and
# active so `cargo` resolves to the pinned 1.95.0 toolchain, not the image's.
export PATH="$HOME/.local/bin:$PATH"
if command -v mise >/dev/null 2>&1; then
  eval "$(mise activate bash)"
fi

make setup           # npm git hook + cargo fetch
cargo test --no-run  # warm the first `make ci`
