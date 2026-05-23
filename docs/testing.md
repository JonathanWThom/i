# Testing strategy

The compiler has three test layers, each with a defined role. Adding a
fourth ad-hoc style ("just write a unit test wherever it's convenient")
defeats the structure: every test belongs to exactly one layer, and the
layers exist so that a failure tells you something specific.

## Layer 1 — Insta corpus tests (bulk regression)

Insta is the Rust ecosystem's snapshot-testing crate. `cargo insta
review` walks each diff between generated output and the committed
`.snap` file; you accept or reject interactively.

- **Scope:** every `.i` file in `examples/` and every `.i` file in
  `tests/corpus/parser/`.
- **Mechanism:** `insta::glob!` reads the directory, runs the lexer
  (or parser) on each file, compares output against a per-file `.snap`.
- **Output format:** custom `Display` on `Token` and AST nodes that
  prints S-expression-style indented trees. (The default `Debug` is
  unreadable past three levels of nesting.)
- **Discipline rule:** never run `cargo insta accept`. Always use
  `cargo insta review` and read each diff. If a snapshot diff is
  "everything changed," that's a signal to back out the change or
  commit to a new shape consciously.

The corpus is small on purpose. Each file targets one syntactic form;
the file name names the form (`lambda-multiline.i`,
`use-cherry-pick.i`, `effect-row.i`, etc.). When you add a new form
to the language, add a corresponding `.i` file.

## Layer 2 — Hand-written assertion tests (errors and edge invariants)

For inputs where the *exact* output text matters and snapshot drift
would silently regress quality.

- **Lexer error cases** (`tests/lexer_errors.rs`): mixed tabs/spaces,
  unterminated string, bad escape, inconsistent dedent. Tested with
  `assert_eq!` on `Error { span, kind }`.
- **Parser error cases** (`tests/parser_errors.rs`): chained
  comparison (`a < b < c`), missing paren, empty match block.
- **Layout invariants** (`tests/lexer_layout.rs`): indent/dedent at
  boundaries, line continuation after operators, paren-depth
  suppression.
- **Per-form parser tests** (`tests/parser_*.rs`): one file per form
  (`parser_calls.rs`, `parser_patterns.rs`, `parser_types.rs`, ...)
  with focused `assert_eq!` over the S-expression Display. These
  pin down the *exact* shape of the AST for each construct, where
  a snapshot diff would be too coarse.

## Layer 3 — Round-trip property test

`tests/roundtrip.rs` asserts the property
`parse(pp(parse(src))) == parse(src)` modulo spans, seeded by every
file in `examples/` and `tests/corpus/parser/`.

For v1 we use a *seeded corpus*, not a generator: a real proptest
generator that produces valid `i` programs would re-encode the grammar
and is out of scope. The round-trip property still catches the bulk
of pretty-printer/parser asymmetries when run over a corpus that
exercises every form.

Failures here usually point at the pretty printer: a missing paren or
a wrong separator that the parser then reads as a different shape.
Fix `src/pretty.rs` and re-run; if the printer is faithful but the
property still fails, the bug is in the parser.

## Conventions

- Snapshot files live in `tests/snapshots/` and are committed.
- Use `cargo insta review`, never `cargo insta accept`.
- Hand-written assertion tests live alongside snapshot tests in
  `tests/` and use plain `assert_eq!` / `matches!`.
- Unit tests on a single struct or trait impl live inline in the same
  file as a `#[cfg(test)] mod tests` block. Integration tests against
  the public API live in `tests/`. Both can coexist; pick by what the
  test is exercising, not by where similar tests already live.
- The corpus is `examples/` plus `tests/corpus/parser/`. Add a corpus
  file when introducing a new syntactic form.

## What snapshot files look like

`tests/snapshots/lexer_corpus__snapshot_examples@01-hello.i.snap`:

```
---
source: tests/lexer_corpus.rs
expression: formatted
input_file: examples/01-hello.i
---
KwModule              @ 0..6
UpperIdent "Main"     @ 7..11
Newline               @ 11..12
Indent                @ 12..16
KwExpose              @ 16..22
LowerIdent "main"     @ 23..27
...
Eof                   @ N..N
```

Spans included so that lexer changes that shift positions are visible
in review.
