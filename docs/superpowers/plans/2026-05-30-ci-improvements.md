# CI Improvements Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get CI off deprecated Node-20 actions before GitHub forces the
cutover (Node 24 default from 2026-06-16, Node 20 removed 2026-09-16),
then claw back the speed lost when we disabled the mise cache to fix the
rustfmt/clippy regression — ideally restoring toolchain caching *and*
sharing the build cache across jobs without reintroducing the
missing-component failure.

**Context:** The CI currently runs three parallel jobs (`fmt`, `clippy`,
`test`), each installing the toolchain via `jdx/mise-action@v2` with
`cache: false`. The `cache: false` was a correctness fix (see
`2026-05-30-cloud-environment-setup.md`, Task 4 amendment): mise installs
rust via rustup into `~/.rustup`, which the mise cache doesn't cover, so a
cache hit dropped rustfmt/clippy. The cost is that every job now does a
full toolchain install on every run. This plan keeps correctness fixed
while looking for the speed back.

**Tech Stack:** GitHub Actions, mise (`jdx/mise-action`), Swatinem/rust-cache, Rust 1.95.0 (pinned via `mise.toml`).

---

## Commit conventions (from CLAUDE.md)

This is build-config work, not a numbered plan task, so use clear verb-led
headlines (not the `Plan N Task M:` form). No `.rs` changes, so the
pre-commit code-review step is skipped. Every commit must pass the husky
pre-commit hook (`make ci`), carry the trailer, and — per the working
motion — be pushed right after committing.

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

## Verification model

CI config can't be exercised locally — the action behaviour only runs on
GitHub. So each task's "verification" is a pushed CI run: confirm all
three jobs stay green, and read the run timings (`gh run view <id>
--json jobs --jq '.jobs[] | {name, startedAt, completedAt}'`, or the
per-step durations in the web UI) to measure the speed effect. Record
cold-cache vs warm-cache numbers where a task touches caching, since the
whole point is the warm-cache path.

## File structure

| File | Responsibility | Change |
| --- | --- | --- |
| `.github/workflows/ci.yml` | CI definition | Bump action majors; revisit cache + job topology |
| `mise.toml` | Toolchain source of truth | Unchanged unless Task 2 needs a cache-path hint |
| `docs/superpowers/plans/2026-05-30-cloud-environment-setup.md` | Cloud/CI toolchain record | Cross-reference the final CI shape |

---

### Task 1: Bump deprecated actions to Node-24 majors

GitHub flagged `actions/checkout@v4` and `jdx/mise-action@v2` as running
on the deprecated Node 20 runtime. The current majors are
`actions/checkout@v6` and `jdx/mise-action@v4` (both Node 24);
`Swatinem/rust-cache@v2` (latest v2.9.1) was *not* flagged, so it already
runs on Node 24 — pin it to the exact patch for reproducibility while
we're here. This is the time-sensitive task; do it first and independently
of the caching exploration so the deadline is covered even if Tasks 2–4
take longer.

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Bump the action versions**

In `.github/workflows/ci.yml`:
- `actions/checkout@v4` → `actions/checkout@v6` (all three jobs)
- `jdx/mise-action@v2` → `jdx/mise-action@v4` (all three jobs; keep
  `with: cache: false` for now — Task 2 revisits it)
- `Swatinem/rust-cache@v2` → `Swatinem/rust-cache@v2.9.1` (clippy, test)

- [ ] **Step 2: Validate the YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('yaml ok')"`
Expected: `yaml ok`.

- [ ] **Step 3: Commit, push, verify the run**

Commit (`Bump CI actions to Node-24 majors`), push, then
`gh run watch <id> --exit-status`. Expected: all three jobs green, and the
run's annotations no longer carry the Node-20 deprecation warning.

---

### Task 2: Restore toolchain caching without losing components

`cache: false` makes every job reinstall the toolchain. Investigate
whether caching can return without the rustfmt/clippy regression. Two
candidate mechanisms, to be compared empirically:

1. **`jdx/mise-action@v4`'s own cache** — the v4 bump may change what the
   action caches. Re-enable (`cache: true`) on a throwaway run and check
   the warm-cache path: does `cargo fmt` still find rustfmt, or does the
   `~/.rustup` gap from the amendment persist?
2. **Explicit `actions/cache` of `~/.rustup` + `~/.local/share/mise`**,
   keyed on `hashFiles('mise.toml')`. If the rustup toolchain is cached
   too, the component loss can't recur.

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Measure the baseline**

From Task 1's run, record per-job wall-clock and the toolchain-install
step duration. This is the `cache: false` cost to beat.

- [ ] **Step 2: Spike mechanism 1 (mise-action cache: true)**

Flip one job to `cache: true`, push, and inspect the warm run (second push
so the cache is populated). Confirm whether rustfmt/clippy survive a cache
hit. If they do *and* it's faster, this is the simplest win.

- [ ] **Step 3: Spike mechanism 2 if needed (explicit ~/.rustup cache)**

If mechanism 1 still drops components, add an `actions/cache` step for
`~/.rustup` and `~/.local/share/mise` keyed on `mise.toml`, before the
mise-action step. Verify warm-cache correctness and timing.

- [ ] **Step 3 (decision): Pick the mechanism**

Adopt whichever restores caching with components intact and a measurable
speedup; if neither beats `cache: false` on the warm path, keep
`cache: false` and record why. Either way, document the decision inline in
`ci.yml` (replace/extend the existing `cache: false` comment).

- [ ] **Step 4: Commit, push, verify**

Commit the chosen config, push, verify two consecutive runs (cold then
warm) are green and the warm run is faster.

---

### Task 3: Evaluate job topology and cross-job build-cache sharing

Three parallel jobs each pay a toolchain install; `clippy` and `test` also
each build the crate. Investigate whether a different topology is cheaper
in wall-clock and/or runner-minutes without losing the parallel-signal
benefit (separate green/red per check).

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Compare topologies**

Measure two shapes against Task 2's baseline:
- **A — keep three parallel jobs** (status quo): best wall-clock, 3×
  toolchain install, separate checks.
- **B — single `make ci` job**: one toolchain install, one build, serial
  fmt→clippy→test; loses per-check granularity but may win runner-minutes.

Record wall-clock and total runner-minutes for each.

- [ ] **Step 2: Spike rust-cache sharing (if staying multi-job)**

If topology A wins, try a `Swatinem/rust-cache` `shared-key` so `clippy`
and `test` reuse one build cache. Watch for cache thrash (clippy metadata
vs test binaries differ) — keep it only if warm builds actually speed up.

- [ ] **Step 2 (decision): Pick the topology**

Choose A or B on the measured numbers; note the trade-off (granularity vs
minutes) inline in `ci.yml`.

- [ ] **Step 3: Commit, push, verify**

Commit, push, confirm green and record the final timings.

---

### Task 4 (optional): Compile-speed levers if build time dominates

If after Tasks 2–3 compilation (not toolchain install) is the bottleneck,
evaluate one lever and stop — don't gold-plate a small project's CI:

- `sccache` via mise, or `CARGO_INCREMENTAL`/profile tuning, or splitting
  the `test` build from the `clippy` build cache.

Spike, measure, adopt only if the warm-run win is real. Otherwise close
this task as "not worth it" with the numbers that show why.

---

### Task 5: Document the final CI shape

Once Tasks 1–3 (and maybe 4) settle, capture the resulting CI design so
the next person isn't re-deriving it from workflow YAML.

**Files:**
- Modify: `docs/superpowers/plans/2026-05-30-cloud-environment-setup.md` (or `docs/cloud-environment.md`) — a short "CI shape" note: action versions, cache strategy, job topology, and why.

- [ ] **Step 1: Write the note, commit, push**

Cross-link it from this plan. No CI run needed (docs-only), but the husky
hook still runs `make ci`.

---

## Explicitly out of scope

- Migrating off mise / GitHub Actions — the toolchain-source-of-truth
  decision is settled (`2026-05-30-cloud-environment-setup.md`).
- Adding new CI checks (coverage, MSRV matrix, release automation) — this
  plan is maintenance + speed, not new gates.
