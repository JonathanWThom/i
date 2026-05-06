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

**TDD strictly.** Write the failing test first. Run it to confirm it
fails. Write the smallest implementation that makes it pass. Run again
to confirm green. Commit. The plan files spell this out per task; follow
the steps as written.

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
functions.

**Span on every token and AST node.** Compile errors are useful only
when they can point at source. Don't drop spans for ergonomics.

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
