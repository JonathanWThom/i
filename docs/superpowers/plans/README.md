# `i` — Implementation Roadmap

The full sequence of plans from "design done" to "useable language." Each plan
is scoped to a single working deliverable, written only when its predecessor
is complete (so each plan can react to what the previous plan revealed).

## Approach: docs-first

The docs describe the end-state language. Examples in `docs/` and `examples/`
become the test suite. Each implementation plan's acceptance criterion is
"the relevant examples now work."

This means: when we hit a wall in implementation, we fix the docs *or* the
implementation, not just one. The docs are not aspirational decoration —
they're the contract.

## Plan sequence

| # | Plan | Deliverable | Acceptance |
|---|---|---|---|
| 1 | **Documentation** | Project skeleton + complete end-state user docs | Every concept in the spec has a user-facing doc; every example file is referenced |
| 1.5 | **Spec revisions from external review** | Spec + docs updated for: space-separated lambda params, effect-polymorphic HOFs, `?` on Maybe, opaque type exports, tuples in v1, expanded stdlib (List helpers + Map + Set) | All docs, examples, and the spec consistently reflect the revised design |
| 2 | **Lexer + parser** | Tokenizer with layout, recursive-descent parser, AST, pretty-printer | Every `examples/*.i` parses; round-trip parse → print → parse is identity |
| 3 | **Name resolution** | Module loader, `expose`/`use`, scope, implicit `Type.` and `self` | Every `examples/*.i` resolves all names; reasonable errors on unresolved names |
| 4 | **Type checker** | Hindley-Milner inference, effect rows, trait constraints, exhaustiveness, totality | Every `examples/*.i` type-checks; `tests/negative/*.i` fail with expected errors |
| 5 | **Tree-walking interpreter** | Evaluate typed AST | Every `examples/*.i` runs and matches `examples/*.out` |
| 6 | **Standard library** | Bool, Int, Float, Char, String, List, Maybe, Result, IO, Ref | `docs/stdlib.md` is fully implemented; stdlib unit tests pass |
| 7 | **Driver / CLI** | `i run`, `i check`, source-span error messages | `docs/building.md` works as described end-to-end |
| 8 | **Golden test harness** | CI runner over `examples/` and negative tests | `cargo test` runs all goldens; broken programs fail with expected output |

## Guiding rules across all plans

1. **TDD.** Every implementation step is "write failing test → implement → green → commit." No bulk implementation followed by "now write tests."
2. **Bite-sized tasks.** 2-5 minutes per step. If a step is bigger, it's secretly multiple steps.
3. **Frequent commits.** Commit after every passing test. Bad commits are easier to revert than to extract from.
4. **No placeholders in plans.** "TBD," "implement later," "handle errors appropriately" are plan failures. Either spec it or note it as out-of-scope explicitly.
5. **Update docs when reality bites.** If implementation reveals the docs are wrong or unclear, update the docs in the same commit. The docs are the contract; broken contracts get fixed.

## When does this end?

The language is "v1 done" when:

- All of `examples/` runs and produces documented output.
- All docs describe currently-working behavior (no forward-looking sections except those explicitly marked v2+).
- `cargo install i-lang && i run hello.i` works on a clean machine.
- A new reader can read `docs/tour.md` and write a small program without consulting the design spec.

After v1: code generation (bytecode VM → native), concurrency (actors),
package management, additional stdlib. None of those are committed yet.

## Currently active plan

→ [Plan 1.5: Spec revisions from external review](2026-04-29-i-language-spec-revisions.md)

## Out of scope for this roadmap

- **Production polish.** No release engineering, no `homebrew` formula, no
  language server, no editor plugins until v1 is functional.
- **Performance.** The interpreter will be slow. That's fine until v1 is
  feature-complete; optimization happens after correctness.
- **Backwards compatibility.** Until the language is announced, breaking
  changes are free.
