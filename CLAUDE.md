# CLAUDE.md

Guidance for working with me on this project. Read it before doing
anything; it's short.

## What this project is

A small statically-typed compiled-ish language called `i`, built as a
learning exercise. The spec is in `docs/`; plans for each implementation
phase live under `docs/superpowers/plans/`. The compiler is hand-written
Rust — no parser generator, no syntax-tree macro magic. Things should be
readable end-to-end.

## How I want you to work

**One task at a time.** Plans break work into bite-sized tasks (5 steps
each, ~5 minutes per step). Execute one task. Stop. Summarise what you
did in 4-6 lines: files touched, verification result, commit hash, any
deviation from the plan. Wait for me to say "yes" before starting the
next task.

**Auto mode does not change this rhythm.** When the user enables auto
mode, you still work one task at a time and still summarise. Auto mode
means "don't ask me to confirm low-risk decisions inside a task," not
"chain ten tasks together silently."

**Walkthroughs on request.** The 4-6 line summary is the default. When
I ask for a walkthrough — or when a task introduces a new module,
pattern, or substantial design decision and you sense it would help —
expand into:

- **What got built** — scope of the task, what's now possible.
- **File layout** — paths, how new files fit together.
- **Key concepts** — design decisions and patterns, grounded in
  concrete code (file paths, sometimes file:line) rather than abstract
  theory. Small concrete examples beat long prose.
- **Why** — alternatives considered, tradeoffs made.
- **What's deferred** — what's intentionally not here yet, and which
  later task fills it in.

Don't pad. Drop any section with nothing interesting to say. The
walkthrough is a learning tool — pick the parts that actually teach.

**TDD strictly.** Write the failing test first. Run it to confirm it
fails. Write the smallest implementation that makes it pass. Run again
to confirm green. The plan files spell this out per task; follow the
steps as written.

**Code review before commit.** After the tests are green and `make ci`
passes, but *before* the commit, spawn a fresh general-purpose subagent
to review the diff. The subagent gets:

- The task's plan section (verbatim).
- `git diff --cached` (or the equivalent unstaged diff if you haven't
  staged yet) — just the diff, not the whole files.
- A pointer to read `CLAUDE.md` and the relevant spec docs.

Ask the subagent for **2-3 findings maximum, or "clean"**. Each finding
is `file:line — one-line problem`. Priorities, in order: TDD or spec
violations; deviation from the plan that wasn't documented; dead code
or over-engineering; missing WHY comments where the code is surprising;
small simplifications. Tell the subagent NOT to propose fixes, NOT to
re-run tests, and NOT to make edits. Identify issues only.

Then report findings to me inline (still terse — 2-3 lines, or "review
clean"). I'll decide which to fix now, defer to a follow-up, or ignore.
Apply any fixes I approve, then commit. Don't commit until I've seen
the review and given the go-ahead.

If the review surfaces a deviation from the plan that's worth keeping,
amend the plan in the same commit (per the existing plan-amendment
rule).

Skip the review step for documentation-only tasks (no `.rs` changes).

**Pause at interactive checkpoints.** Snapshot review (`cargo insta
review`) and any human-in-the-loop step belongs to me. Generate the
artifact, tell me what to look at, hand off.

## Plans

Plans are the source of truth for what we're building and why. They live
under `docs/superpowers/plans/<date>-<slug>.md`. Follow the
`superpowers:writing-plans` format — header, file structure, testing
strategy, then numbered tasks with TDD steps.

When a decision comes up mid-execution that the plan didn't anticipate,
**amend the plan** before doing the work. Don't carry the decision in
chat history; bake it into the file. Same for deviations: if the plan
said one thing and we did another, edit the plan to match reality and
explain why in the commit.

## Code conventions

**Identifiers in `i` source are camelCase only.** No underscores anywhere
in identifiers. The bare `_` is the wildcard pattern, nothing else. The
lexer enforces this with a useful error.

**Spec is canonical.** When the implementation suggests the spec is
wrong (or under-specified), fix the spec — don't paper over it in code.
Document the change in the commit that touches both.

**Comments are rare.** Only when the WHY isn't obvious from the code:
hidden constraints, workarounds, surprising invariants. Don't restate
what the code does. Don't write multi-paragraph docstrings on internal
functions. Actively prune superfluous comments as you write — a comment
that a reader could delete without losing information is noise, and
should not survive into the commit. Keep only the ones that earn their
place; when in doubt, cut it.

**Span on every token and AST node.** Compile errors are useful only
when they can point at source. Don't drop spans for ergonomics.

**Test placement follows Rust idiom.** Unit tests on a single struct or
trait impl live inline in the same file as a `#[cfg(test)] mod tests`
block. Integration tests against the public API (e.g. `lex()` over many
inputs) live in `tests/`. Both can coexist; pick by what the test is
exercising, not by where similar tests already live. Snapshot tests
(insta) go in `tests/` regardless because the `.snap` files live next
to them.

## Commits

Format the commit headline as `Plan N Task M: <verb-led description>`.
Body explains what changed and why in 2-4 sentences — focus on the
"why" because the diff already shows the "what." Sign the trailer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

`Cargo.lock` is gitignored. Don't try to add it. If a doc change is
tightly coupled to the same task (e.g., spec clarification), fold it
into the same commit via amend rather than creating a fresh one.

**Push is the finalizer.** Once a commit lands (the pre-commit hook's
`make ci` having passed), push it to the current branch's remote without
asking — committing authorises the push. The rhythm is unchanged: one
task, commit, push, summarise, wait for "yes". This covers only ordinary
fast-forward pushes of new commits; force-push, rebase, and reset against
shared history still require asking (see "What to ask before doing").

## Quality bar

Every commit must pass `make ci`:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

A husky pre-commit hook runs `make ci` automatically. If it fails, the
commit aborts — don't bypass with `--no-verify`. Fix the underlying
issue.

## Teaching mode

This is a learning project. When I ask "what's X" or "why did we do Y,"
explain it tied to the actual code on disk — file paths, current state,
specific decisions we made — not abstract theory from a textbook. Use
small concrete examples. Acknowledge tradeoffs and what we deferred.

Short answers beat long ones. Tables beat prose when the content is
genuinely tabular. Avoid restating what I just said.

## What you don't need to ask before doing

- Reading any file in the repo
- Running `cargo`, `make`, `git status`, `git log`
- Running tests at any time
- Making the smallest change to fix a failing test
- Editing `docs/` to clarify wording or fix typos when adjacent to other
  work in the same task
- Pushing new commits to the current branch's remote right after
  committing (ordinary fast-forward, non-force)

## What to ask before doing

- Anything that touches `main`'s history (`git push --force`, rebase,
  reset --hard)
- Adding a dependency to `Cargo.toml` or `package.json`
- Changing the language spec in a way that affects existing examples
- Skipping a planned task or merging two tasks
- Deleting files

## Memory

`/Users/jonathanthom/.claude/projects/-Users-jonathanthom-code-i/memory/`
exists for facts that span conversations. Use it sparingly — most of the
project context belongs here in CLAUDE.md (version-controlled, visible
to me) rather than in memory (local, opaque). Save to memory only when
something is genuinely user-personal or cross-project.
