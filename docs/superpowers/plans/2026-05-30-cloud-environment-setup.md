# Cloud Environment Setup + Bootstrap Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a reproducible cloud (managed-agents) environment for the `i` compiler while removing the only real duplication between its setup script and the repo's existing bootstrap mechanisms.

**Architecture:** The repo's `Makefile` is already the canonical interface for routine actions; the cloud script's job is *host provisioning + project bootstrap*. We add a single `make setup` target for project bootstrap (reused by README + cloud script), make `mise.toml` the single source of truth for the toolchain (version *and* components) consumed identically by all three environments — local Mac, GitHub CI, and Claude cloud — and check the canonical cloud config into the repo so it isn't trapped in a web form.

**Tech Stack:** GNU Make, mise (toolchain pinning, via `jdx/mise-action` in CI), Rust 1.95.0 (cargo, rustfmt, clippy), husky (npm-managed git hooks), GitHub Actions, Anthropic managed-agents cloud environments.

---

## Concurrency / conflict note

Another terminal is doing unrelated work with several files dirty (seen at
various points: `tests/check_end_to_end.rs`, `README.md`, `docs/checker.md`,
`docs/superpowers/plans/PROGRESS.md`). To stay fully isolated, this work runs in
a dedicated git worktree (`.claude/worktrees/cloud-env-setup`, branch
`worktree-cloud-env-setup`) branched from `e86675b`. Any overlap (notably
`README.md`) is therefore resolved at merge time, visibly, rather than by
clobbering the other session's working copy. Still use explicit `git add` of
this plan's files only (never `git add -A`).

## Commit conventions (from CLAUDE.md)

This is build-config/docs work, not a numbered plan task, so use a clear
verb-led headline (not the `Plan N Task M:` form). No `.rs` changes, so the
pre-commit code-review step is skipped. Every commit must still pass the husky
pre-commit hook (`make ci`) and carry the trailer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

## File structure

| File | Responsibility | Change |
| --- | --- | --- |
| `Makefile` | Canonical task interface | Add `setup` target (project bootstrap) |
| `README.md` | Human setup instructions | Point setup step at `make setup` |
| `mise.toml` | Single source of truth for toolchain (version + components), read by local / CI / cloud | Declare `rustfmt,clippy` components on the rust pin |
| `.github/workflows/ci.yml` | CI definition | Install toolchain via `jdx/mise-action` (reads `mise.toml`) instead of `dtolnay/rust-toolchain@stable`; drop per-job `components:` |
| `docs/cloud-environment.md` | Version-controlled record of the cloud env config | New file |

---

### Task 1: Add a `make setup` target for project bootstrap

The Makefile is the canonical interface for every routine action; "bootstrap
the repo" should be a target rather than prose in the README. `make setup`
wires the husky pre-commit hook (`npm install` → `prepare`) and pre-fetches the
`insta`/`proptest` dev-deps (`cargo fetch`). `cargo fetch` is cheap locally and
saves a network round-trip in the cloud, so it belongs in the shared target.

**Files:**
- Modify: `Makefile` (`.PHONY` line, `help` recipe, new `setup` recipe)

- [ ] **Step 1: Confirm the target does not exist yet (red)**

Run: `make setup`
Expected: FAIL — `make: *** No rule to make target 'setup'.  Stop.`

- [ ] **Step 2: Add `setup` to `.PHONY` and the help text**

In `Makefile`, change the `.PHONY` line from:

```makefile
.PHONY: help fmt fmt-check lint test ci dev clean rev
```

to:

```makefile
.PHONY: help setup fmt fmt-check lint test ci dev clean rev
```

And add this line to the `help` recipe, immediately after the opening
`@echo "make fmt        — apply rustfmt"` line is fine, but place it first so
bootstrap reads top-to-bottom:

```makefile
	@echo "make setup      — one-time: install git hooks + fetch deps"
```

- [ ] **Step 3: Add the `setup` recipe**

Add the recipe (place it just before the `fmt:` recipe so order matches help):

```makefile
setup:
	npm install
	cargo fetch
```

- [ ] **Step 4: Run the target to verify it succeeds (green)**

Run: `make setup`
Expected: PASS — `npm install` completes (husky `prepare` runs; `.husky/_/`
present afterward) and `cargo fetch` reports dependencies resolved/downloaded
with no error.

- [ ] **Step 5: Verify help renders the new line**

Run: `make help`
Expected: output includes `make setup      — one-time: install git hooks + fetch deps`.

- [ ] **Step 6: Commit**

```bash
git add Makefile
git commit -m "Add make setup target for project bootstrap

The Makefile is the canonical interface for routine actions, but
bootstrapping a clone (install git hooks, fetch deps) lived only as prose in
the README. make setup unifies it so README and the cloud setup script share
one definition.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Point the README setup step at `make setup`

Replace the bare `npm install` instruction so there's one documented bootstrap
command and the README and cloud script stay in sync.

**Files:**
- Modify: `README.md` (the husky/pre-commit setup block, around lines 45–51)

- [ ] **Step 1: Read the current setup block**

Run: `sed -n '40,55p' README.md` (read-only; confirm exact wording before editing)
Expected: a block mentioning the husky pre-commit hook and `npm install` as the
one-time wiring step.

- [ ] **Step 2: Replace the command, keep the explanation**

In `README.md`, change the fenced command from:

```sh
npm install      # one-time; installs husky and runs `prepare`
```

to:

```sh
make setup       # one-time; installs the git hook (npm) and fetches deps
```

Keep the surrounding sentence that explains the Node dependency exists only for
the pre-commit hook and the compiler itself is pure Rust.

- [ ] **Step 3: Verify the command in the doc actually works**

Run: `make setup`
Expected: PASS (same as Task 1 Step 4) — the documented command is real and green.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "Point README setup step at make setup

Single documented bootstrap command, in sync with the new Makefile target.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Declare toolchain components in `mise.toml`

Make `mise.toml` the single source of truth for the toolchain — version *and*
the `rustfmt`/`clippy` components `make ci` needs — so a box provisioned purely
from `mise install` is CI-ready and neither the cloud script nor CI re-types
`1.95.0`. Verified against mise docs: mise honors a `components` field, drives
rustup under the hood, and defaults to rustup's configured profile, so the
declared components are installed regardless of the host's default profile
(GitHub runners / cloud images use rustup's minimal profile, which omits them
unless declared). This task is the foundation Task 4 (CI) depends on.

**Files:**
- Modify: `mise.toml`

- [ ] **Step 1: Record current behavior (red-ish baseline)**

Run: `cat mise.toml`
Expected: `[tools]` with `rust = "1.95.0"` and **no** component declaration —
i.e. nothing guarantees rustfmt/clippy on a fresh `mise install`.

- [ ] **Step 2: Add components to the rust pin**

In `mise.toml`, change:

```toml
[tools]
rust = "1.95.0"
```

to:

```toml
[tools]
rust = { version = "1.95.0", components = "rustfmt,clippy" }
```

- [ ] **Step 3: Re-resolve the toolchain and verify components**

Run: `mise install`
Then run: `cargo fmt --version` and `cargo clippy --version`
Expected: both resolve under the 1.95.0 toolchain with no "component not
installed" error — proving a `mise install`-only box can run `make ci`.

- [ ] **Step 4: Confirm `make ci` is unaffected**

Run: `make ci`
Expected: PASS (`fmt-check + lint + test`). The change only affects
provisioning, not the Rust sources, so existing tests/snapshots are untouched.

- [ ] **Step 5: Commit**

```bash
git add mise.toml
git commit -m "Declare rustfmt/clippy components in mise.toml

Makes mise.toml the single source of truth for the toolchain so a box
provisioned from mise install alone is CI-ready, and the cloud setup script
needn't duplicate the 1.95.0 version pin.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Align CI to `mise.toml`

CI currently installs the toolchain with `dtolnay/rust-toolchain@stable` — it
floats on whatever stable is newest, independent of the `1.95.0` pin local and
cloud use, and lists components per job. Switch every job to install via
`jdx/mise-action@v2`, which reads `mise.toml`. After this, version *and*
components for all three environments (local Mac, CI, Claude cloud) come from
the single `mise.toml` declaration added in Task 3. Decision (confirmed with
the user): pin only — no separate floating-`stable` job; reproducibility over
early-warning-on-new-compilers.

The repo `mise.toml` pins only `rust`, so a fresh CI checkout installs only the
Rust toolchain (the node/ruby/etc. seen locally come from the user's *global*
mise config, not the repo's). `make fmt-check`/`lint`/`test` need no npm, so no
node tool is required in CI.

**Files:**
- Modify: `.github/workflows/ci.yml` (all three jobs)

- [ ] **Step 1: Record the current workflow (baseline)**

Run: `cat .github/workflows/ci.yml`
Expected: three jobs (`fmt`, `clippy`, `test`) each using
`dtolnay/rust-toolchain@stable`, with `fmt`/`clippy` listing `components:`.

- [ ] **Step 2: Rewrite the workflow to install via mise**

Replace the entire contents of `.github/workflows/ci.yml` with:

```yaml
name: ci

on:
  push:
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  fmt:
    name: cargo fmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jdx/mise-action@v2
      - run: make fmt-check

  clippy:
    name: cargo clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jdx/mise-action@v2
      - uses: Swatinem/rust-cache@v2
      - run: make lint

  test:
    name: cargo test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jdx/mise-action@v2
      - uses: Swatinem/rust-cache@v2
      - run: make test
```

The `components:` lines are gone because `mise.toml` now declares them;
`Swatinem/rust-cache@v2` stays on the compiling jobs (it reads `rustc -V` from
the mise-activated toolchain). `RUSTFLAGS: -D warnings` stays as-is.

- [ ] **Step 3: Validate the YAML and confirm the commands pass locally**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('yaml ok')"`
Expected: `yaml ok` (no parse error).
Then run: `make ci`
Expected: PASS — proves the three commands CI runs are green on the pinned
1.95.0 toolchain. (Actual `jdx/mise-action` behavior is verified on push, Step 5.)

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "Install CI toolchain via mise instead of dtolnay@stable

CI floated on @stable while local and cloud pinned 1.95.0; jdx/mise-action
reads mise.toml so all three environments share one toolchain definition
(version + components). Per-job component lists are dropped since mise.toml
declares them. Pinned only — no floating-stable job, by design.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 5: Verify the real CI run (after push)**

This is the interactive checkpoint — the toolchain-install path only exercises
on GitHub. After the branch is pushed and a PR/commit triggers CI, confirm all
three jobs are green and that the logs show mise installing rust `1.95.0` with
`rustfmt`/`clippy`. Hand this check to the user (or run `gh run watch` if a run
is in flight).

---

### Task 5: Check the cloud environment config into the repo

The managed-agents environment is configured in a web dialog (Name / Network
access / Environment variables / Setup script). Capture the canonical values in
the repo so they're version-controlled, reviewable, and reproducible rather than
trapped in the UI. The setup script reuses `mise install` (Task 3) and
`make setup` (Task 1) so there is one definition of each concern.

**Files:**
- Create: `docs/cloud-environment.md`

- [ ] **Step 1: Create the doc with the canonical config**

Create `docs/cloud-environment.md` with exactly this content:

````markdown
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
- **Version + components are read from `mise.toml`**, never re-typed here.

### Fallback if `mise` is awkward on the base image

Replace the provisioning block with a rustup install that still reads the pin
from `mise.toml` rather than hardcoding it:

```bash
ver="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' mise.toml | head -n1)"
rustup toolchain install "$ver" --component rustfmt clippy
rustup default "$ver"
```
````

- [ ] **Step 2: Sanity-check the doc's commands against reality**

Run: `make setup` and `make ci`
Expected: both PASS — confirming the two repo-side commands the script depends
on are real and green. (`mise install` already verified in Task 3.)

- [ ] **Step 3: Commit**

```bash
git add docs/cloud-environment.md
git commit -m "Document cloud environment config

Checks the managed-agents environment (name, network level, env vars, setup
script) into the repo instead of leaving it in the web dialog. The setup
script reuses mise install and make setup so nothing is duplicated.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Persist this plan to the repo

Plan mode wrote this plan to `~/.claude/plans/`; copy it into the repo's plan
directory so it lives alongside the other plans.

**Files:**
- Create: `docs/superpowers/plans/2026-05-30-cloud-environment-setup.md`

- [ ] **Step 1: Copy the plan file**

```bash
cp ~/.claude/plans/how-does-this-setup-glowing-cupcake.md \
   docs/superpowers/plans/2026-05-30-cloud-environment-setup.md
```

- [ ] **Step 2: Verify it landed**

Run: `ls docs/superpowers/plans/2026-05-30-cloud-environment-setup.md`
Expected: the path prints (file exists).

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-05-30-cloud-environment-setup.md
git commit -m "Add cloud-environment setup plan

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## End-to-end verification

After all tasks, from a clean shell:

1. `make help` — lists `make setup`.
2. `make setup` — wires husky (`.husky/_/` present) and fetches deps, no error.
3. `mise install` then `cargo fmt --version` / `cargo clippy --version` — both
   resolve under 1.95.0 (components present).
4. `make ci` — green (`fmt-check + lint + test`), proving Rust sources are
   untouched.
5. `.github/workflows/ci.yml` parses as valid YAML and references
   `jdx/mise-action@v2` in all three jobs with no `dtolnay`/`components:` left.
6. A trivial throwaway commit fires the pre-commit hook and runs `make ci`
   successfully (then discard it).
7. (Real CI, after push) All three CI jobs green; logs show mise installing
   rust `1.95.0` with `rustfmt`/`clippy`. — interactive checkpoint.
8. (Optional, real cloud) Create the environment using
   `docs/cloud-environment.md` on the **Trusted** network level; confirm the
   session starts and `make ci` passes inside it.

## Self-review notes

- **Spec coverage:** the alignment goal (local Mac = CI = Claude cloud, all
  reading `mise.toml`) maps to Task 3 (declare version + components) + Task 4
  (CI consumes it via mise-action) + Task 5 (cloud script consumes it). The
  bootstrap consolidation maps to Tasks 1–2. Cloud config is Task 5; plan
  persistence is Task 6.
- **No placeholders:** every edit shows exact before/after text and exact
  commands with expected output.
- **Consistency:** the cloud setup script (Task 5) calls `make setup` (Task 1)
  and relies on the components added in Task 3; CI (Task 4) relies on the same
  Task 3 declaration — one `mise.toml` feeds all three.
- **Verification honesty:** `jdx/mise-action`'s real behavior only runs on
  GitHub, so Task 4 splits verification into a local YAML+`make ci` check and a
  post-push CI check (the interactive checkpoint), rather than claiming CI is
  green before it has run.
