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

## Post-v1 tracks (not committed; sketched only)

After v1 ships, work splits into independent tracks. Each is a future plan, not a commitment. Approximate ordering — formatter is the natural first post-v1 step because every other tool wants stable canonical layout to depend on.

| # | Plan | Track | Deliverable |
|---|---|---|---|
| 9 | **Formatter (`i fmt`)** | Tooling | Canonical pretty-printer turned into an in-place formatter; `i fmt path/...` rewrites files; CI mode `i fmt --check` exits non-zero on drift. |
| 10 | **Language server** | Tooling | LSP-protocol server: hover types, go-to-definition, find references, diagnostics on save, completions. One binary, one VS Code extension as the reference client. |
| 11 | **Editor plugins** | Tooling | TextMate grammar for syntax highlighting (drives VS Code, Sublime, others); tree-sitter grammar; reference VS Code extension; community-maintained Vim/Neovim and JetBrains adapters as scope allows. |
| 12 | **REPL** | Tooling | Interactive `i repl` with multi-line input, type info on every binding, `:t`/`:k`/`:doc` commands. |
| 13 | **Doc generator** | Tooling | `i doc` extracts type signatures and doc comments from `.i` files, emits browseable HTML/markdown. The stdlib reference becomes generated, not hand-written. |
| 14 | **Bytecode VM (v2)** | Codegen | Compile to a stack-based bytecode; `.ic` artifact runnable by `i exec`. Wins on startup time and execution speed over the tree-walker. |
| 15 | **Native codegen (v3)** | Codegen | Cranelift, LLVM, or custom backend producing standalone binaries. Decision deferred until v2 is shipped. |
| 16 | **Concurrency (actors)** | Runtime | Actor-based message passing on top of the effect system; no shared mutable state across boundaries. Specified after v1 because it's load-bearing on language design. |
| 17 | **Package manager** | Distribution | `i.toml` manifest, dependency resolution, registry. Shipped only when there's enough stdlib and stability that third-party libraries make sense. |
| 18 | **Stdlib expansion** | Library | Networking, async, JSON, regex, time, env. Added as cross-cutting demand emerges, not preemptively. |

These can run in parallel once v1 ships. The tooling plans (9-13) are independent of the codegen plans (14-15) and can be tackled in either order.

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

After v1: see "Post-v1 tracks" above for the sketched-out next plans
(tooling, codegen, concurrency, package manager, stdlib expansion). None
of those are committed yet.

## Currently active plan

→ [Plan 1.5: Spec revisions from external review](2026-04-29-i-language-spec-revisions.md)

## Out of scope for this roadmap

- **Production polish during v1.** Release engineering, `homebrew` formula,
  language server, editor plugins are post-v1 (Plans 9-13).
- **Performance.** The interpreter will be slow. That's fine until v1 is
  feature-complete; optimization happens in the codegen track (Plans 14-15).
- **Backwards compatibility.** Until the language is announced, breaking
  changes are free.
