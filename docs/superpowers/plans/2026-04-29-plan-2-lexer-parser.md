# Plan 2 — Lexer, AST, Parser

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the front end of the `i` compiler — a lexer that turns source bytes into a token stream, an AST, and a parser that consumes tokens into AST nodes — covering every form in `docs/syntax.md` and producing a stable, span-bearing tree that later phases (name resolution, types, effects, codegen) will walk.

**Architecture:** Hand-written lexer with synthetic layout tokens (`Indent`/`Dedent`/`Newline`); Pratt-style expression parser layered under a recursive-descent decl/statement parser; AST as `Spanned<Kind>` enums; pretty-printer used both for diagnostics later and for a round-trip property test now.

**Tech Stack:** Rust 1.95, edition 2024. New deps: `insta` (snapshot tests), `proptest` (round-trip). No parser-generator crate — the grammar is small and indentation-sensitive enough that a hand-written parser is clearer than a generated one.

---

## Decisions baked in

These were open questions before the plan; resolved here so they don't reopen during implementation.

1. **`and` / `or` / `not` / `xor` are lexed as keywords** (`KwAnd`, `KwOr`, `KwNot`, `KwXor`). Spec calls them `Std.Bool` functions, but the precedence table in `syntax.md § 11` gives them ranks — meaning the parser needs to dispatch on them by token kind. Lexing as keywords makes that clean. Trade-off: they become reserved words you can't shadow. Worth the simplification.

2. **String escape set (v1):** `\n`, `\t`, `\r`, `\\`, `\"`, `\0`. No `\xNN`, no unicode escapes. Add later if needed.

3. **Negative numeric literals are always two tokens** (`Minus` followed by `IntLit` or `FloatLit`). Unary minus is rank 10 in the precedence table; the parser composes them. No special-casing in the lexer.

4. **`..` is a single token** (`DotDot`). It only appears in `expose Type(..)`. Cleaner than two `Dot` tokens that the parser has to reassemble.

5. **AST nodes carry a `Span`** via `Spanned<Kind>`. Spans are byte offsets into the original source. Two `PartialEq` variants: derived (compares spans) and a `node_eq` helper that ignores spans, used by the round-trip test.

---

## File structure

```
src/
  lib.rs               # re-exports for the binary and tests
  main.rs              # CLI scaffold; Plan 7 fleshes it out
  span.rs              # Span { start: u32, end: u32 } + Spanned<T>
  error.rs             # Error { span, kind } + ErrorKind enum
  token.rs             # Token enum + TokenKind for matching
  lex/
    mod.rs             # public lex(&str) -> Result<Vec<Token>, Error>
    cursor.rs          # byte cursor over source; peek/bump/span helpers
    layout.rs          # indent stack, line-continuation, layout-token emission
  ast.rs               # Expr, Type, Pattern, Decl, File enums; Display impl
  parse/
    mod.rs             # public parse(&[Token]) -> Result<File, Error>
    cursor.rs          # token cursor; peek/bump/expect helpers; error spans
    expr.rs            # Pratt expression parser
    pat.rs             # patterns (match arms, lambda params)
    typ.rs             # type expressions
    decl.rs            # bindings, type/trait/impl decls, module header, use
  pretty.rs            # AST -> source-equivalent String

tests/
  lexer_corpus.rs      # insta::glob! over examples/*.i
  lexer_layout.rs      # hand-written: indent/dedent, line continuation
  lexer_errors.rs      # hand-written: mixed tabs, bad escapes, etc.
  parser_corpus.rs     # insta::glob! over examples/*.i
  parser_features.rs   # insta::glob! over tests/corpus/parser/*.i
  parser_errors.rs     # hand-written: chained comparisons, etc.
  roundtrip.rs         # proptest: parse(pp(parse(s))) == parse(s)
  corpus/
    parser/
      lambda-multiline.i
      match-nested.i
      method-chain.i
      construction.i
      record-update.i
      sum-with-fields.i
      newtype-block.i
      type-annotations.i
      list-literal.i
      effect-row.i
      question-postfix.i
      use-cherry-pick.i

snapshots/              # insta-managed; committed
```

**Why this split:** `lex/` and `parse/` each have a `cursor.rs` because the cursor abstraction is reused across the module. `parse/expr.rs` holds the entire Pratt loop in one file — splitting Pratt across files makes it hard to read. Pattern and type parsers get their own files because they recur (patterns appear in match arms and lambda params; types appear in field decls and signatures), and isolating them keeps `expr.rs` focused on expressions.

---

## Testing strategy

Three test layers, each with a defined role. This section is the contract — Task 33 copies it into a permanent doc.

### Layer 1 — Insta corpus tests (bulk regression)

Insta is the Rust ecosystem's snapshot-testing crate. `cargo insta review` walks each diff between generated output and the committed `.snap` file; you accept or reject interactively.

- **Scope:** every `.i` file in `examples/` and every `.i` file in `tests/corpus/parser/`.
- **Mechanism:** `insta::glob!` reads the directory, runs the lexer (or parser) on each file, compares output against a per-file `.snap`.
- **Output format:** custom `Display` on `Token` and AST nodes that prints S-expression-style indented trees. (The default `Debug` is unreadable past three levels of nesting.)
- **Discipline rule:** never run `cargo insta accept`. Always use `cargo insta review` and read each diff. If a snapshot diff is "everything changed," that's a signal to back out the change or commit to a new shape consciously.

### Layer 2 — Hand-written assertion tests (errors and edge invariants)

For inputs where the *exact* output text matters and snapshot drift would silently regress quality.

- **Lexer error cases:** mixed tabs/spaces, unterminated string, bad escape, inconsistent dedent. Tested with `assert_eq!` on `Error { span, kind }`.
- **Parser error cases:** chained comparison (`a < b < c`), empty match block, malformed kwargs.
- **Layout invariants:** indent/dedent at boundaries, line continuation after operators, paren-depth suppression.

### Layer 3 — Round-trip property test

Once the pretty-printer exists, one `proptest` test:

```rust
#[test]
fn parse_pretty_roundtrip() {
    proptest!(|(src in valid_program_strategy())| {
        let ast1 = parse(&lex(&src).unwrap()).unwrap();
        let printed = pretty(&ast1);
        let ast2 = parse(&lex(&printed).unwrap()).unwrap();
        prop_assert!(ast_eq_ignoring_spans(&ast1, &ast2));
    });
}
```

For v1 we use a *seeded corpus*, not a generator: the strategy reads the same `examples/*.i` and `tests/corpus/parser/*.i` files. A real proptest generator that produces valid `i` programs would re-encode the grammar and is out of scope for Plan 2. The round-trip property still catches the bulk of pretty-printer/parser asymmetries when run over a corpus that exercises every form.

### What snapshot files look like

`tests/snapshots/lexer_corpus__01-hello.snap`:

```
---
source: tests/lexer_corpus.rs
expression: tokens
input_file: examples/01-hello.i
---
KwModule              @ 0..6
UpperIdent "Main"     @ 7..11
Newline               @ 11..12
Indent                @ 12..16
KwExpose              @ 16..22
LowerIdent "main"     @ 23..27
Newline               @ 27..28
Dedent                @ 28..28
...
Eof                   @ N..N
```

Spans included so that lexer changes that shift positions are visible in review.

---

## Tasks

### Phase 1 — Foundation

#### Task 1: Add deps and module skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/span.rs`, `src/error.rs`, `src/token.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add insta and proptest to Cargo.toml**

```toml
[dev-dependencies]
insta = { version = "1", features = ["glob"] }
proptest = "1"
```

- [ ] **Step 2: Create `src/span.rs`**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span { pub start: u32, pub end: u32 }

impl Span {
    pub fn new(start: u32, end: u32) -> Self { Self { start, end } }
    pub fn merge(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[derive(Clone, PartialEq)]
pub struct Spanned<T> { pub span: Span, pub node: T }

impl<T: std::fmt::Debug> std::fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.node.fmt(f)
    }
}
```

- [ ] **Step 3: Create `src/error.rs`** with empty `ErrorKind` enum (variants added per task)

```rust
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Error { pub span: Span, pub kind: ErrorKind }

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    // populated by later tasks
}
```

- [ ] **Step 4: Create `src/token.rs`** with empty enum (variants added in Task 2)

```rust
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token { pub span: Span, pub kind: TokenKind }

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // populated in Task 2
    Eof,
}
```

- [ ] **Step 5: Wire up `src/lib.rs`**

```rust
pub mod span;
pub mod error;
pub mod token;
```

- [ ] **Step 6: Verify build**

Run: `cargo build`
Expected: clean build with warnings about unused enum variants

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/span.rs src/error.rs src/token.rs
git commit -m "Plan 2 Task 1: foundation modules and dev-deps"
```

---

#### Task 2: Token enum

**Files:**
- Modify: `src/token.rs`

- [ ] **Step 1: Replace `TokenKind` with the full set**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),

    // Identifiers
    LowerIdent(String),
    UpperIdent(String),

    // Keywords (reserved lowercase)
    KwType, KwMatch, KwModule, KwExpose, KwUse, KwAs,
    KwTrait, KwImpl,
    KwAnd, KwOr, KwNot, KwXor,

    // The four binding operators
    Colon, Equals, Arrow, Dot,

    // Punctuation
    LParen, RParen, LBracket, RBracket, Comma,
    Bang, Question, DotDot,

    // Arithmetic / comparison / concat (desugar in parser)
    Plus, Minus, Star, Slash, Caret,
    EqEq, SlashEq, Lt, LtEq, Gt, GtEq,
    PlusPlus,

    // Layout (lexer-synthetic)
    Newline, Indent, Dedent,

    // Sentinel
    Eof,
}
```

- [ ] **Step 2: Add a one-line classifier**

```rust
impl TokenKind {
    pub fn is_layout(&self) -> bool {
        matches!(self, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent)
    }
}
```

- [ ] **Step 3: Build to verify**

Run: `cargo build`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/token.rs
git commit -m "Plan 2 Task 2: full TokenKind enum"
```

---

#### Task 3: Lexer scaffold

**Files:**
- Create: `src/lex/mod.rs`, `src/lex/cursor.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/lex/cursor.rs`**

```rust
pub(super) struct Cursor<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(src: &'a str) -> Self { Self { src: src.as_bytes(), pos: 0 } }
    pub fn pos(&self) -> u32 { self.pos as u32 }
    pub fn at_end(&self) -> bool { self.pos >= self.src.len() }
    pub fn peek(&self) -> Option<u8> { self.src.get(self.pos).copied() }
    pub fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }
    pub fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }
    pub fn bump_if(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) { self.pos += 1; true } else { false }
    }
}
```

- [ ] **Step 2: Create `src/lex/mod.rs`**

```rust
mod cursor;

use crate::error::Error;
use crate::span::Span;
use crate::token::{Token, TokenKind};
use cursor::Cursor;

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut cur = Cursor::new(src);
    let mut out = Vec::new();
    // Tasks 4-12 fill this in. Today we just emit Eof.
    let span = Span::new(cur.pos(), cur.pos());
    out.push(Token { span, kind: TokenKind::Eof });
    Ok(out)
}
```

- [ ] **Step 3: Add to `src/lib.rs`**

```rust
pub mod lex;
```

- [ ] **Step 4: Write the harness test in `tests/lexer_corpus.rs`**

```rust
use i_lang::lex::lex;

#[test]
fn empty_program() {
    let toks = lex("").unwrap();
    assert_eq!(toks.len(), 1);
    assert!(matches!(toks[0].kind, i_lang::token::TokenKind::Eof));
}
```

- [ ] **Step 5: Run test**

Run: `cargo test --test lexer_corpus`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lex/ src/lib.rs tests/lexer_corpus.rs
git commit -m "Plan 2 Task 3: lexer scaffold emitting only Eof"
```


---

### Phase 2 — Lexer

#### Task 4: Punctuation and operators

**Files:**
- Modify: `src/lex/mod.rs`
- Create: `tests/lexer_punct.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/lexer_punct.rs
use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn single_char_punct() {
    assert_eq!(kinds("()[],.:= !?"), vec![
        LParen, RParen, LBracket, RBracket, Comma,
        Dot, Colon, Equals, Bang, Question, Eof,
    ]);
}

#[test]
fn multi_char_operators() {
    assert_eq!(kinds("-> == /= <= >= ++ .."), vec![
        Arrow, EqEq, SlashEq, LtEq, GtEq, PlusPlus, DotDot, Eof,
    ]);
}

#[test]
fn arithmetic_operators() {
    assert_eq!(kinds("+ - * / ^ < >"), vec![
        Plus, Minus, Star, Slash, Caret, Lt, Gt, Eof,
    ]);
}
```

- [ ] **Step 2: Run — confirm it fails**

Run: `cargo test --test lexer_punct`
Expected: FAIL (lexer only emits Eof).

- [ ] **Step 3: Implement in `src/lex/mod.rs`**

Inside `lex()`, before pushing the final `Eof`, loop:

```rust
loop {
    // skip horizontal whitespace (spaces and tabs handled in Task 9)
    while let Some(b' ') = cur.peek() { cur.bump(); }
    let start = cur.pos();
    let kind = match cur.peek() {
        None => break,
        Some(b'(') => { cur.bump(); TokenKind::LParen }
        Some(b')') => { cur.bump(); TokenKind::RParen }
        Some(b'[') => { cur.bump(); TokenKind::LBracket }
        Some(b']') => { cur.bump(); TokenKind::RBracket }
        Some(b',') => { cur.bump(); TokenKind::Comma }
        Some(b'+') => { cur.bump(); if cur.bump_if(b'+') { TokenKind::PlusPlus } else { TokenKind::Plus } }
        Some(b'-') => { cur.bump(); if cur.bump_if(b'>') { TokenKind::Arrow } else { TokenKind::Minus } }
        Some(b'*') => { cur.bump(); TokenKind::Star }
        Some(b'/') => { cur.bump(); if cur.bump_if(b'=') { TokenKind::SlashEq } else { TokenKind::Slash } }
        Some(b'^') => { cur.bump(); TokenKind::Caret }
        Some(b'=') => { cur.bump(); if cur.bump_if(b'=') { TokenKind::EqEq } else { TokenKind::Equals } }
        Some(b'<') => { cur.bump(); if cur.bump_if(b'=') { TokenKind::LtEq } else { TokenKind::Lt } }
        Some(b'>') => { cur.bump(); if cur.bump_if(b'=') { TokenKind::GtEq } else { TokenKind::Gt } }
        Some(b'!') => { cur.bump(); TokenKind::Bang }
        Some(b'?') => { cur.bump(); TokenKind::Question }
        Some(b':') => { cur.bump(); TokenKind::Colon }
        Some(b'.') => { cur.bump(); if cur.bump_if(b'.') { TokenKind::DotDot } else { TokenKind::Dot } }
        Some(c) => return Err(Error {
            span: Span::new(start, start + 1),
            kind: ErrorKind::UnexpectedChar(c as char),
        }),
    };
    let span = Span::new(start, cur.pos());
    out.push(Token { span, kind });
}
```

Add `UnexpectedChar(char)` to `ErrorKind` in `src/error.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test --test lexer_punct`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/mod.rs src/error.rs tests/lexer_punct.rs
git commit -m "Plan 2 Task 4: lex punctuation and operators"
```

---

#### Task 5: Identifiers and keywords

**Files:**
- Modify: `src/lex/mod.rs`
- Create: `tests/lexer_idents.rs`

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::lex::lex;
use i_lang::token::TokenKind::{self, *};

fn kinds(src: &str) -> Vec<TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn lower_ident() {
    assert_eq!(kinds("foo bar_baz x1"), vec![
        LowerIdent("foo".into()), LowerIdent("bar_baz".into()),
        LowerIdent("x1".into()), Eof,
    ]);
}

#[test]
fn upper_ident() {
    assert_eq!(kinds("Point List Maybe"), vec![
        UpperIdent("Point".into()), UpperIdent("List".into()),
        UpperIdent("Maybe".into()), Eof,
    ]);
}

#[test]
fn keywords() {
    assert_eq!(kinds("type match module expose use as trait impl and or not xor"), vec![
        KwType, KwMatch, KwModule, KwExpose, KwUse, KwAs,
        KwTrait, KwImpl, KwAnd, KwOr, KwNot, KwXor, Eof,
    ]);
}

#[test]
fn keyword_lookalike_is_ident() {
    // 'types' is not 'type' — full match required
    assert_eq!(kinds("types typeof"), vec![
        LowerIdent("types".into()), LowerIdent("typeof".into()), Eof,
    ]);
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_idents`
Expected: FAIL.

- [ ] **Step 3: Implement**

In the match arm in `lex()`, before the catch-all error arm:

```rust
Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
    let is_upper = c.is_ascii_uppercase();
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphanumeric() || c == b'_' { cur.bump(); } else { break; }
    }
    let text = std::str::from_utf8(&src.as_bytes()[start as usize..cur.pos() as usize])
        .unwrap().to_string();
    if is_upper {
        TokenKind::UpperIdent(text)
    } else {
        match text.as_str() {
            "type"   => TokenKind::KwType,
            "match"  => TokenKind::KwMatch,
            "module" => TokenKind::KwModule,
            "expose" => TokenKind::KwExpose,
            "use"    => TokenKind::KwUse,
            "as"     => TokenKind::KwAs,
            "trait"  => TokenKind::KwTrait,
            "impl"   => TokenKind::KwImpl,
            "and"    => TokenKind::KwAnd,
            "or"     => TokenKind::KwOr,
            "not"    => TokenKind::KwNot,
            "xor"    => TokenKind::KwXor,
            _        => TokenKind::LowerIdent(text),
        }
    }
}
```

(Pass `src: &str` into the loop scope so the slice works. Refactor as needed.)

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/mod.rs tests/lexer_idents.rs
git commit -m "Plan 2 Task 5: lex identifiers and keywords"
```

---

#### Task 6: Numeric literals

**Files:**
- Modify: `src/lex/mod.rs`, `src/error.rs`
- Create: `tests/lexer_numbers.rs`

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn integers() {
    assert_eq!(kinds("0 42 1000"), vec![
        IntLit(0), IntLit(42), IntLit(1000), Eof,
    ]);
}

#[test]
fn floats() {
    assert_eq!(kinds("3.14 0.0 100.5"), vec![
        FloatLit(3.14), FloatLit(0.0), FloatLit(100.5), Eof,
    ]);
}

#[test]
fn negative_is_two_tokens() {
    // per Plan 2 decision: unary minus + literal
    assert_eq!(kinds("-3 -3.14"), vec![
        Minus, IntLit(3), Minus, FloatLit(3.14), Eof,
    ]);
}

#[test]
fn dot_after_int_is_float() {
    assert_eq!(kinds("3.14"), vec![FloatLit(3.14), Eof]);
}

#[test]
fn dot_method_call_not_float() {
    // "3.foo" would be method call on int; we don't support that, but the
    // tokenizer must not consume the dot if no digit follows.
    // For now: digits.dot.digits is float; digits.dot.alpha is int + dot + ident.
    assert_eq!(kinds("3.foo"), vec![
        IntLit(3), Dot, LowerIdent("foo".into()), Eof,
    ]);
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_numbers`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
Some(c) if c.is_ascii_digit() => {
    while let Some(c) = cur.peek() { if c.is_ascii_digit() { cur.bump(); } else { break; } }
    // Float? Only if dot is followed by a digit.
    if cur.peek() == Some(b'.') && cur.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
        cur.bump(); // consume '.'
        while let Some(c) = cur.peek() { if c.is_ascii_digit() { cur.bump(); } else { break; } }
        let text = &src[start as usize..cur.pos() as usize];
        TokenKind::FloatLit(text.parse().map_err(|_| Error {
            span: Span::new(start, cur.pos()),
            kind: ErrorKind::InvalidNumber,
        })?)
    } else {
        let text = &src[start as usize..cur.pos() as usize];
        TokenKind::IntLit(text.parse().map_err(|_| Error {
            span: Span::new(start, cur.pos()),
            kind: ErrorKind::InvalidNumber,
        })?)
    }
}
```

Add `InvalidNumber` to `ErrorKind`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/mod.rs src/error.rs tests/lexer_numbers.rs
git commit -m "Plan 2 Task 6: lex integer and float literals"
```


---

#### Task 7: String literals with escapes

**Files:**
- Modify: `src/lex/mod.rs`, `src/error.rs`
- Create: `tests/lexer_strings.rs`

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::lex::lex;
use i_lang::error::ErrorKind;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn plain_string() {
    assert_eq!(kinds(r#""hello""#), vec![StringLit("hello".into()), Eof]);
}

#[test]
fn escaped_chars() {
    assert_eq!(kinds(r#""a\nb\tc\\d\"e\rf\0g""#), vec![
        StringLit("a\nb\tc\\d\"e\rf\0g".into()), Eof,
    ]);
}

#[test]
fn empty_string() {
    assert_eq!(kinds(r#""""#), vec![StringLit(String::new()), Eof]);
}

#[test]
fn unterminated() {
    let err = lex(r#""unterminated"#).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnterminatedString));
}

#[test]
fn bad_escape() {
    let err = lex(r#""bad \q escape""#).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::InvalidEscape('q')));
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_strings`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
Some(b'"') => {
    cur.bump();
    let mut s = String::new();
    loop {
        match cur.bump() {
            None => return Err(Error {
                span: Span::new(start, cur.pos()),
                kind: ErrorKind::UnterminatedString,
            }),
            Some(b'"') => break,
            Some(b'\\') => {
                let esc_start = cur.pos() - 1;
                let c = cur.bump().ok_or(Error {
                    span: Span::new(esc_start, cur.pos()),
                    kind: ErrorKind::UnterminatedString,
                })?;
                s.push(match c {
                    b'n'  => '\n',
                    b't'  => '\t',
                    b'r'  => '\r',
                    b'\\' => '\\',
                    b'"'  => '"',
                    b'0'  => '\0',
                    other => return Err(Error {
                        span: Span::new(esc_start, cur.pos()),
                        kind: ErrorKind::InvalidEscape(other as char),
                    }),
                });
            }
            Some(b'\n') => return Err(Error {
                span: Span::new(start, cur.pos()),
                kind: ErrorKind::UnterminatedString,
            }),
            Some(c) => s.push(c as char),
        }
    }
    TokenKind::StringLit(s)
}
```

Add `UnterminatedString` and `InvalidEscape(char)` to `ErrorKind`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/mod.rs src/error.rs tests/lexer_strings.rs
git commit -m "Plan 2 Task 7: lex string literals with escapes"
```

---

#### Task 8: Comments

**Files:**
- Modify: `src/lex/mod.rs`
- Create: `tests/lexer_comments.rs`

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn line_comment_consumed() {
    assert_eq!(kinds("# this is a comment"), vec![Eof]);
}

#[test]
fn trailing_comment() {
    // newline still comes through; layout tasks (9-10) refine this.
    assert_eq!(kinds("x = 1 # comment"), vec![
        LowerIdent("x".into()), Equals, IntLit(1), Eof,
    ]);
}

#[test]
fn comment_does_not_eat_next_line() {
    let src = "# c1\nx";
    assert_eq!(kinds(src), vec![LowerIdent("x".into()), Eof]);
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_comments`
Expected: FAIL.

- [ ] **Step 3: Implement**

In the trivia-skipping prelude of the loop, before reading the next token:

```rust
loop {
    match cur.peek() {
        Some(b' ') | Some(b'\t') => { cur.bump(); }
        Some(b'#') => {
            while let Some(c) = cur.peek() {
                if c == b'\n' { break; }
                cur.bump();
            }
        }
        _ => break,
    }
}
```

(Newlines are still left in the stream; Task 9 handles them.)

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/mod.rs tests/lexer_comments.rs
git commit -m "Plan 2 Task 8: skip line comments"
```

---

#### Task 8.5: Extract token scanners (refactor — no behaviour change)

**Files:**
- Create: `src/lex/scan.rs`
- Modify: `src/lex/mod.rs`

By this point the inline match in `lex()` is the bulk of the file and the
character-class branches obscure the loop's structure. Pull each class into
a pure function so `lex()` becomes a one-screen dispatcher. No tests change;
the full existing test suite is the regression check.

- [ ] **Step 1: Create `src/lex/scan.rs`**

Move these functions out of `lex()`'s match into `scan.rs`, each taking
`&mut Cursor`, the source `&str`, and the start position, and returning
`Result<TokenKind, Error>`:

- `scan_string`     — handles `"` … `"` plus escapes
- `scan_number`     — handles digit run with optional float tail
- `scan_ident_or_keyword` — handles ASCII letter starts, keywords, no underscores
- `scan_underscore` — handles bare `_` vs underscore-in-identifier error

Punctuation stays inline in `lex()`: each branch is one or two lines, and
inlining keeps the dispatch readable.

- [ ] **Step 2: Reduce `lex()` to a dispatch loop**

```rust
pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut cur = Cursor::new(src);
    let mut out = Vec::new();
    loop {
        while let Some(b' ') = cur.peek() { cur.bump(); }
        let start = cur.pos();
        let kind = match cur.peek() {
            None => break,
            Some(b'"') => scan::scan_string(&mut cur, start)?,
            Some(c) if c.is_ascii_digit() => scan::scan_number(&mut cur, src, start)?,
            Some(b'_') => scan::scan_underscore(&mut cur, src, start)?,
            Some(c) if c.is_ascii_alphabetic() => scan::scan_ident_or_keyword(&mut cur, src, start)?,
            // single/multi-char punctuation arms inline as today
            Some(b'(') => { cur.bump(); TokenKind::LParen }
            // ... etc
        };
        out.push(Token { span: Span::new(start, cur.pos()), kind });
    }
    let end = cur.pos();
    out.push(Token { span: Span::new(end, end), kind: TokenKind::Eof });
    Ok(out)
}
```

- [ ] **Step 3: Run the full test suite**

Run: `cargo test`
Expected: every prior test still passes; no new tests added.

- [ ] **Step 4: Commit**

```bash
git add src/lex/scan.rs src/lex/mod.rs
git commit -m "Plan 2 Task 8.5: extract token scanners (refactor)"
```

---

#### Task 9: Newlines and line continuation

**Files:**
- Create: `src/lex/layout.rs`
- Modify: `src/lex/mod.rs`
- Create: `tests/lexer_layout.rs`

This task introduces *only* the newline rule and line continuation. Indent/Dedent come in Task 10.

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn newline_terminates_line() {
    assert_eq!(kinds("x = 1\ny = 2\n"), vec![
        LowerIdent("x".into()), Equals, IntLit(1), Newline,
        LowerIdent("y".into()), Equals, IntLit(2), Newline, Eof,
    ]);
}

#[test]
fn blank_lines_collapsed() {
    // multiple newlines collapse to one
    assert_eq!(kinds("x\n\n\ny\n"), vec![
        LowerIdent("x".into()), Newline,
        LowerIdent("y".into()), Newline, Eof,
    ]);
}

#[test]
fn trailing_operator_continues_line() {
    // Newline suppressed after binary operator
    assert_eq!(kinds("a +\nb"), vec![
        LowerIdent("a".into()), Plus, LowerIdent("b".into()), Eof,
    ]);
}

#[test]
fn open_paren_continues_line() {
    assert_eq!(kinds("f (\n  x\n)"), vec![
        LowerIdent("f".into()), LParen,
        LowerIdent("x".into()), RParen, Eof,
    ]);
}

#[test]
fn comma_continues_line() {
    assert_eq!(kinds("f a,\n  b"), vec![
        LowerIdent("f".into()), LowerIdent("a".into()),
        Comma, LowerIdent("b".into()), Eof,
    ]);
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_layout`
Expected: FAIL.

- [ ] **Step 3: Implement state in `src/lex/layout.rs`**

```rust
use crate::token::TokenKind;

pub(super) struct Layout {
    pub paren_depth: u32,
    pub last_significant: Option<TokenKind>,
}

impl Layout {
    pub fn new() -> Self {
        Self { paren_depth: 0, last_significant: None }
    }

    pub fn note_emitted(&mut self, k: &TokenKind) {
        match k {
            TokenKind::LParen | TokenKind::LBracket => self.paren_depth += 1,
            TokenKind::RParen | TokenKind::RBracket => self.paren_depth = self.paren_depth.saturating_sub(1),
            _ => {}
        }
        if !k.is_layout() { self.last_significant = Some(k.clone()); }
    }

    pub fn suppresses_newline(&self) -> bool {
        if self.paren_depth > 0 { return true; }
        matches!(self.last_significant, Some(
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash
            | TokenKind::Caret | TokenKind::EqEq | TokenKind::SlashEq
            | TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq
            | TokenKind::PlusPlus | TokenKind::Colon | TokenKind::Equals
            | TokenKind::Arrow | TokenKind::Comma
        ))
    }
}
```

- [ ] **Step 4: Wire it into `src/lex/mod.rs`**

In the lex loop, when a `\n` is encountered (it should be a separate match arm, before the existing trivia skip), check `layout.suppresses_newline()`:

```rust
Some(b'\n') => {
    cur.bump();
    if !layout.suppresses_newline() {
        let last_was_newline = matches!(out.last().map(|t| &t.kind), Some(TokenKind::Newline));
        if !last_was_newline && !out.is_empty() {
            let span = Span::new(start, cur.pos());
            out.push(Token { span, kind: TokenKind::Newline });
            layout.note_emitted(&TokenKind::Newline);
        }
    }
    continue;
}
```

After every `out.push(...)` for non-layout tokens, call `layout.note_emitted(&kind)`.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: all PASS, including earlier tests.

- [ ] **Step 6: Commit**

```bash
git add src/lex/layout.rs src/lex/mod.rs tests/lexer_layout.rs
git commit -m "Plan 2 Task 9: newline emission and line continuation"
```


---

#### Task 10: Indent and Dedent

**Files:**
- Modify: `src/lex/layout.rs`, `src/lex/mod.rs`, `src/error.rs`
- Modify: `tests/lexer_layout.rs`

- [ ] **Step 1: Add the failing tests**

```rust
#[test]
fn block_opens_indent_closes_dedent() {
    let src = "main =\n    x\n    y\n";
    assert_eq!(kinds(src), vec![
        LowerIdent("main".into()), Equals, Newline,
        Indent,
        LowerIdent("x".into()), Newline,
        LowerIdent("y".into()), Newline,
        Dedent, Eof,
    ]);
}

#[test]
fn nested_blocks() {
    let src = "a =\n    b =\n        c\n    d\n";
    assert_eq!(kinds(src), vec![
        LowerIdent("a".into()), Equals, Newline,
        Indent,
            LowerIdent("b".into()), Equals, Newline,
            Indent,
                LowerIdent("c".into()), Newline,
            Dedent,
            LowerIdent("d".into()), Newline,
        Dedent, Eof,
    ]);
}

#[test]
fn parens_suppress_layout() {
    // Inside parens, layout tokens are suppressed entirely.
    let src = "f (\n    x\n    y\n)";
    assert_eq!(kinds(src), vec![
        LowerIdent("f".into()), LParen,
        LowerIdent("x".into()), LowerIdent("y".into()),
        RParen, Eof,
    ]);
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_layout`
Expected: FAIL on the new tests.

- [ ] **Step 3: Extend `Layout` in `src/lex/layout.rs`**

```rust
pub(super) struct Layout {
    pub paren_depth: u32,
    pub last_significant: Option<TokenKind>,
    pub indent_stack: Vec<u32>,  // column widths; starts as [0]
}

impl Layout {
    pub fn new() -> Self {
        Self { paren_depth: 0, last_significant: None, indent_stack: vec![0] }
    }
    pub fn top(&self) -> u32 { *self.indent_stack.last().unwrap() }
    // ... existing methods unchanged
}
```

- [ ] **Step 4: Update the newline handler in `src/lex/mod.rs`**

When a `\n` is consumed and not suppressed:
1. Emit `Newline` (if appropriate per Task 9 logic).
2. Read leading whitespace of the next non-blank, non-comment-only line. Determine its column.
3. If `paren_depth > 0`, skip layout entirely.
4. Otherwise:
   - If `column > layout.top()`: push column onto stack, emit `Indent`.
   - If `column < layout.top()`: pop while `column < layout.top()`, emitting `Dedent` per pop. If after popping `column != layout.top()`, error: `InconsistentDedent`.
   - If equal: nothing more to emit.

Sketch:

```rust
fn read_indent(cur: &mut Cursor) -> Option<u32> {
    // Skip blank/comment-only lines, return final column or None at EOF.
    loop {
        let line_start = cur.pos();
        let mut col = 0u32;
        while let Some(c) = cur.peek() {
            match c {
                b' ' => { cur.bump(); col += 1; }
                b'\t' => { cur.bump(); col += 1; } // tabs == 1 col; mixed-tab error in Task 11
                _ => break,
            }
        }
        match cur.peek() {
            Some(b'\n') => { cur.bump(); continue; }
            Some(b'#') => { while let Some(c) = cur.peek() { if c == b'\n' { break; } cur.bump(); } continue; }
            None => return None,
            _ => return Some(col),
        }
        let _ = line_start; // keep for span use if needed
    }
}
```

Then on entering newline handling:

```rust
let col = match read_indent(&mut cur) { Some(c) => c, None => break };
if layout.paren_depth == 0 {
    let pos = cur.pos();
    while col < layout.top() {
        layout.indent_stack.pop();
        out.push(Token { span: Span::new(pos, pos), kind: TokenKind::Dedent });
        layout.note_emitted(&TokenKind::Dedent);
    }
    if col > layout.top() {
        layout.indent_stack.push(col);
        out.push(Token { span: Span::new(pos, pos), kind: TokenKind::Indent });
        layout.note_emitted(&TokenKind::Indent);
    } else if col != layout.top() {
        return Err(Error {
            span: Span::new(pos, pos),
            kind: ErrorKind::InconsistentDedent,
        });
    }
}
```

At EOF, drain the indent stack: emit one `Dedent` per remaining level above 0.

Add `InconsistentDedent` to `ErrorKind`.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lex/layout.rs src/lex/mod.rs src/error.rs tests/lexer_layout.rs
git commit -m "Plan 2 Task 10: Indent/Dedent emission with paren suppression"
```

---

#### Task 11: Mixed tabs and spaces error

**Files:**
- Modify: `src/lex/mod.rs`, `src/error.rs`
- Modify: `tests/lexer_errors.rs` (create if not exists)

- [ ] **Step 1: Write the failing test**

```rust
// tests/lexer_errors.rs
use i_lang::lex::lex;
use i_lang::error::ErrorKind;

#[test]
fn mixed_tabs_and_spaces_in_indent() {
    let src = "main =\n\t x\n";  // tab then space
    let err = lex(src).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::MixedTabsAndSpaces));
}

#[test]
fn spaces_only_ok() {
    let src = "main =\n    x\n";
    assert!(lex(src).is_ok());
}

#[test]
fn tabs_only_ok() {
    let src = "main =\n\tx\n";
    assert!(lex(src).is_ok());
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --test lexer_errors`
Expected: FAIL.

- [ ] **Step 3: Implement**

In `read_indent`, track the *first* whitespace byte seen on a line; if a later byte differs, error. Also track the *file-wide* dominant indent character: once a file uses tabs at any indented line, spaces at any indented line are an error (and vice versa).

Simplest correct approach: store an `Option<u8>` `indent_char` in `Layout`. First time a non-zero indent line is seen, set it. On any subsequent indent that contains the *other* character, raise `MixedTabsAndSpaces`.

Add `MixedTabsAndSpaces` to `ErrorKind`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lex/ src/error.rs tests/lexer_errors.rs
git commit -m "Plan 2 Task 11: error on mixed tabs and spaces"
```

---

#### Task 11.5: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`

A minimum bar for green-on-main: every push and PR runs three jobs in
parallel — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
and `cargo test`. The clippy/test jobs cache the build between runs via
`Swatinem/rust-cache@v2` so subsequent CI runs finish in ~30s instead of
~3min. `RUSTFLAGS: -D warnings` is set globally so any compile warning
also fails CI.

`cargo doc` is *not* in CI yet — we have no doc comments to check. Add
later when there's a public API worth documenting.

- [ ] **Step 1: Run clippy locally first**

Run: `cargo clippy --all-targets -- -D warnings`
Fix any lints before adding the workflow, so the first CI run is green.

- [ ] **Step 2: Create `.github/workflows/ci.yml`** (full content above).

- [ ] **Step 3: Verify locally**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: all three commands exit 0.

- [ ] **Step 4: Commit and push**

```bash
git add .github/workflows/ci.yml
git commit -m "Plan 2 Task 11.5: CI on every push (fmt + clippy + test)"
git push
```

Watch the first run on GitHub; expected green across all three jobs.

---

#### Task 12: Lexer corpus snapshot tests

**Files:**
- Create: `tests/lexer_corpus_glob.rs`
- Modify: `src/token.rs` (add `Display`)

- [ ] **Step 1: Implement `Display` on `Token`**

```rust
// src/token.rs
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<22} @ {:?}", self.kind_label(), self.span)
    }
}

impl Token {
    fn kind_label(&self) -> String {
        match &self.kind {
            TokenKind::IntLit(n)         => format!("IntLit {}", n),
            TokenKind::FloatLit(n)       => format!("FloatLit {}", n),
            TokenKind::StringLit(s)      => format!("StringLit {:?}", s),
            TokenKind::LowerIdent(s)     => format!("LowerIdent {:?}", s),
            TokenKind::UpperIdent(s)     => format!("UpperIdent {:?}", s),
            other                        => format!("{:?}", other),
        }
    }
}
```

- [ ] **Step 2: Write the corpus test**

```rust
// tests/lexer_corpus_glob.rs
use i_lang::lex::lex;

#[test]
fn snapshot_examples() {
    insta::glob!("../examples/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).unwrap();
        let formatted: String = toks.iter().map(|t| format!("{}\n", t)).collect();
        insta::assert_snapshot!(formatted);
    });
}
```

- [ ] **Step 3: Run and review**

Run: `cargo test --test lexer_corpus_glob`
Expected: tests fail because no snapshots exist yet.

Run: `cargo insta review`
Expected: walks each pending snapshot. **Read each one carefully** before accepting. Confirm the token stream looks right against the source `.i` file.

- [ ] **Step 4: Commit snapshots**

```bash
git add tests/lexer_corpus_glob.rs tests/snapshots/ src/token.rs
git commit -m "Plan 2 Task 12: lexer corpus snapshots over examples/"
```


---

#### Task 12.5: Makefile and pre-commit hook

**Files:**
- Create: `Makefile`
- Create: `.husky/pre-commit`
- Create: `package.json`
- Modify: `.github/workflows/ci.yml`, `.gitignore`, `README.md`

A single source of truth for "what does CI run." `Makefile` exposes
targets `fmt`, `fmt-check`, `lint`, `test`, `ci` (= fmt-check + lint +
test), `dev`, `rev` (cargo insta review), `clean`. The CI workflow
calls `make <target>` per job so the workflow and the local command
stay in sync.

A husky-managed pre-commit hook runs `make ci` before every commit.
husky needs Node — the only Node dependency in this project. After
clone, contributors run `npm install`, which triggers husky's
`prepare` script and installs the hook.

Trade-off note: husky pulls a JS toolchain into a Rust project. The
Rust-native alternative is `cargo-husky` (a crate). Swap is a one-line
edit if Node ever becomes a problem.

- [ ] **Step 1: Create the Makefile** (full content above).

- [ ] **Step 2: Rewire `.github/workflows/ci.yml`**

Each job becomes a single `make <target>` call (`make fmt-check`,
`make lint`, `make test`).

- [ ] **Step 3: Add husky**

```sh
npm init -y
npm install --save-dev husky
npx husky init   # creates .husky/pre-commit and adds prepare script
```

Edit `.husky/pre-commit` to one line: `make ci`.

Trim `package.json` to the essentials (drop the auto-generated
`main`, `keywords`, `bugs`, `homepage`, `repository`, broken `test`
script). Keep only `name`, `private: true`, `description`, the
`prepare` script, and `devDependencies`.

- [ ] **Step 4: gitignore `node_modules/`**

- [ ] **Step 5: README — Development section**

Document `make ci`, `make dev`, `make rev`, and the `npm install`
one-time setup for the pre-commit hook.

- [ ] **Step 6: Verify locally**

Run: `make ci`
Expected: all three sub-commands pass.

The Task 12.5 commit itself is the first run of the pre-commit hook —
if `make ci` fails, the commit aborts.

- [ ] **Step 7: Commit**

```bash
git add Makefile .github/workflows/ci.yml .husky/ package.json \
        .gitignore README.md
git commit -m "Plan 2 Task 12.5: Makefile and husky pre-commit hook"
```

---

### Phase 3 — AST

#### Task 13: AST data types

**Files:**
- Create: `src/ast.rs`
- Modify: `src/lib.rs`

This task defines the data shape only. No parsing yet. The shapes follow `docs/syntax.md` directly.

- [ ] **Step 1: Create `src/ast.rs`**

```rust
use crate::span::Spanned;

pub type Expr = Spanned<ExprKind>;
pub type Type = Spanned<TypeKind>;
pub type Pattern = Spanned<PatternKind>;
pub type Decl = Spanned<DeclKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub module: Option<ModuleHeader>,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleHeader {
    pub name: String,
    pub exposes: Vec<Expose>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expose {
    Value(String),
    Type { name: String, with_constructors: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    Var(String),         // lowercase identifier in expression position
    Ctor(String),        // uppercase identifier in expression position
    List(Vec<Expr>),
    Paren(Box<Expr>),    // for round-trip fidelity; later passes can flatten

    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    UnaryOp { op: UnaryOp, expr: Box<Expr> },

    Lambda { params: Vec<Pattern>, body: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },             // juxtaposition / comma
    MethodCall { receiver: Box<Expr>, method: String },     // before args; args attach via Call
    FieldAccess { receiver: Box<Expr>, field: String },     // p.x
    Construct { type_name: String, fields: Vec<KwArg> },    // Type(x = 0, y = 0)
    Update { value: Box<Expr>, fields: Vec<KwArg> },        // instance(x = 5)

    Bang(Box<Expr>),     // postfix !  (effectful call marker)
    Question(Box<Expr>), // postfix ?

    Match { scrutinee: Box<Expr>, arms: Vec<MatchArm> },
    Block(Vec<BlockItem>),  // indented block; last item is the value
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div, Pow, Eq, Ne, Lt, Le, Gt, Ge, Concat, And, Or, Xor }

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp { Neg, Not }

#[derive(Debug, Clone, PartialEq)]
pub struct KwArg { pub name: String, pub value: Expr }

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm { pub pattern: Pattern, pub body: Expr }

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Binding(Decl),  // a Binding decl
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Var(String),
    Lit(LitPat),
    Ctor { name: String, args: Vec<Pattern> },           // Constructor a, b
    Record { type_name: String, fields: Vec<FieldPat> }, // Point(x = a, y = b)
    Tuple(Vec<Pattern>),                                 // (a, b)
    List(Vec<Pattern>),                                  // [a, b]
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitPat { Int(i64), Float(f64), Str(String) }

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPat { pub field: String, pub pattern: Pattern }

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Var(String),                                         // lowercase: type variable
    Named { name: String, args: Vec<Type> },             // List a; Tree a
    Function { params: Vec<Type>, effect: Option<EffectRow>, result: Box<Type> },
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectRow {
    Empty,                       // ! ()
    Named(Vec<String>),          // ! IO, ! IO + State, etc.  (multi-effect: TBD lexically)
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    Binding {
        name: String,
        ty: Option<Type>,        // optional annotation
        value: Option<Expr>,     // None for sig-only
    },
    TypeDecl {
        name: String,
        params: Vec<String>,     // type parameters (lowercase)
        body: TypeBody,
    },
    TraitDecl {
        name: String,
        type_var: String,
        methods: Vec<Decl>,      // each is a Binding with no value (signature)
    },
    ImplDecl {
        trait_name: String,
        target: Type,
        methods: Vec<Decl>,      // each is a Binding with a value
    },
    Use {
        path: Vec<String>,
        kind: UseKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBody {
    Newtype(Type),                     // type Foo = T
    Block(Vec<TypeMember>),            // type Foo \n  ...
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMember {
    Field { name: String, ty: Type },
    Method(Decl),                      // a Binding with a value
    Variant {
        name: String,
        body: VariantBody,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantBody {
    Bare,
    Single(Type),                      // Variant : T
    Fields(Vec<TypeMember>),           // indented field block
}

#[derive(Debug, Clone, PartialEq)]
pub enum UseKind {
    Whole,                             // use Std.IO
    Cherry(Vec<String>),               // use Std.IO (print, readLine)
    Alias(String),                     // use Std.Float as F
}
```

- [ ] **Step 2: Add a `node_eq` helper for span-blind comparison**

```rust
impl File {
    pub fn node_eq(&self, other: &File) -> bool {
        // Strip spans by re-derivation. Simplest: serialize both via Display
        // and compare. (Display impl in Task 14.)
        // For Task 13, this is a stub returning self == other; updated in Task 14.
        self == other
    }
}
```

- [ ] **Step 3: Wire to lib**

```rust
// src/lib.rs
pub mod ast;
```

- [ ] **Step 4: Build to verify**

Run: `cargo build`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/ast.rs src/lib.rs
git commit -m "Plan 2 Task 13: AST data types"
```

---

#### Task 14: AST custom Display

**Files:**
- Modify: `src/ast.rs`

The default `Debug` print is unreadable past a few levels. Implement an indented S-expression-style `Display` for `File`. This becomes the snapshot format for parser tests.

- [ ] **Step 1: Write the failing test**

```rust
// in src/ast.rs at the bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn sp<T>(node: T) -> Spanned<T> { Spanned { span: Span::new(0, 0), node } }

    #[test]
    fn display_simple_let() {
        let file = File {
            module: None,
            decls: vec![sp(DeclKind::Binding {
                name: "x".into(),
                ty: None,
                value: Some(sp(ExprKind::IntLit(1))),
            })],
        };
        let out = format!("{}", file);
        assert_eq!(out.trim(), "(file\n  (let x (int 1)))");
    }
}
```

- [ ] **Step 2: Run — confirm fail**

Run: `cargo test --lib ast`
Expected: FAIL.

- [ ] **Step 3: Implement Display recursively**

A small writer that tracks indent depth and emits `(node ...)` forms. Each AST variant gets a printing rule. Use `std::fmt::Write` and an indent counter; the conventions are:

- `(file <decls>)` at top level
- `(let NAME EXPR)` for bindings
- `(int N)`, `(float N)`, `(str "...")` for literals
- `(var NAME)`, `(ctor NAME)` for identifiers
- `(call FUNC ARGS)` for calls
- `(lambda (PARAMS) BODY)` for lambdas
- `(. RECEIVER FIELD)` for field access
- `(match SCRUTINEE (ARMS))` for match
- One node per line when the body is non-trivial; collapse simple atoms onto one line.

Update `node_eq` in Task 13 to compare `format!("{}", self)` against `format!("{}", other)`. Span-blind by construction.

`Display` is implemented on `File` (top level) and `ExprKind` (so parser tests in Task 16+ can format individual expressions). The shared `Printer` exposes both `write_file(&File)` and `write_expr_kind(&ExprKind)` so they reuse the same indent rules.

- [ ] **Step 4: Run tests**

Run: `cargo test --lib ast`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ast.rs
git commit -m "Plan 2 Task 14: AST Display for snapshot tests"
```


---

### Phase 4 — Parser

The parser builds bottom-up: scaffold → atoms → Pratt expressions → calls/postfix → lambdas → match → patterns → types → bindings → decls → top level. Each task adds *one* form and is testable against a small `.i` snippet.

#### Task 15: Parser scaffold

**Files:**
- Create: `src/parse/mod.rs`, `src/parse/cursor.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create the cursor**

```rust
// src/parse/cursor.rs
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub(super) struct Cursor<'a> {
    toks: &'a [Token],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(toks: &'a [Token]) -> Self { Self { toks, pos: 0 } }

    pub fn peek(&self) -> &Token { &self.toks[self.pos] }
    pub fn peek_kind(&self) -> &TokenKind { &self.toks[self.pos].kind }
    pub fn at_end(&self) -> bool { matches!(self.peek_kind(), TokenKind::Eof) }

    pub fn bump(&mut self) -> &Token {
        let t = &self.toks[self.pos];
        if !self.at_end() { self.pos += 1; }
        t
    }

    pub fn check(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(k)
    }

    pub fn eat(&mut self, k: &TokenKind) -> bool {
        if self.check(k) { self.bump(); true } else { false }
    }

    pub fn expect(&mut self, k: TokenKind, expected: &'static str) -> Result<&Token, Error> {
        if self.check(&k) {
            Ok(self.bump())
        } else {
            let span = self.peek().span;
            Err(Error {
                span,
                kind: ErrorKind::Unexpected {
                    found: format!("{:?}", self.peek_kind()),
                    expected,
                },
            })
        }
    }
}
```

- [ ] **Step 2: Add the new error variant**

In `src/error.rs`:

```rust
Unexpected { found: String, expected: &'static str },
```

- [ ] **Step 3: Create `src/parse/mod.rs`**

```rust
mod cursor;
mod expr;     // created in Task 16
mod pat;      // created in Task 21
mod typ;      // created in Task 23
mod decl;     // created in Task 24

use crate::ast::File;
use crate::error::Error;
use crate::token::Token;
use cursor::Cursor;

pub fn parse(toks: &[Token]) -> Result<File, Error> {
    let mut cur = Cursor::new(toks);
    decl::parse_file(&mut cur)
}
```

- [ ] **Step 4: Stub `parse_file` in `src/parse/decl.rs`**

```rust
use super::cursor::Cursor;
use crate::ast::File;
use crate::error::Error;

pub(super) fn parse_file(cur: &mut Cursor) -> Result<File, Error> {
    Ok(File { module: None, decls: vec![] })  // expanded in Task 26
}
```

- [ ] **Step 5: Stub `expr.rs`, `pat.rs`, `typ.rs` so the module tree compiles**

Each is a one-line `pub(super) fn placeholder() {}` for now.

- [ ] **Step 6: Add to lib**

```rust
// src/lib.rs
pub mod parse;
```

- [ ] **Step 7: Build**

Run: `cargo build`
Expected: clean (with unused-fn warnings).

- [ ] **Step 8: Commit**

```bash
git add src/parse/ src/lib.rs src/error.rs
git commit -m "Plan 2 Task 15: parser scaffold"
```

---

#### Task 16: Atom expressions

**Files:**
- Modify: `src/parse/expr.rs`
- Create: `tests/parser_atoms.rs`

Atoms are the leaves of expressions: literals, identifiers, parenthesized expressions, list literals.

- [ ] **Step 1: Write the failing test**

```rust
// tests/parser_atoms.rs
use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;  // expose a test helper

fn parse(src: &str) -> String {
    let toks = lex(src).unwrap();
    let e = parse_expr_for_test(&toks).unwrap();
    format!("{}", e.node)  // Use Display impl on ExprKind
}

#[test]
fn int_literal() { assert_eq!(parse("42"), "(int 42)"); }
#[test]
fn float_literal() { assert_eq!(parse("3.14"), "(float 3.14)"); }
#[test]
fn string_literal() { assert_eq!(parse(r#""hi""#), r#"(str "hi")"#); }
#[test]
fn lower_var() { assert_eq!(parse("foo"), "(var foo)"); }
#[test]
fn upper_ctor() { assert_eq!(parse("None"), "(ctor None)"); }
#[test]
fn paren_group() { assert_eq!(parse("(42)"), "(paren (int 42))"); }
#[test]
fn list_literal() { assert_eq!(parse("[1, 2, 3]"), "(list (int 1) (int 2) (int 3))"); }
#[test]
fn empty_list() { assert_eq!(parse("[]"), "(list)"); }
```

- [ ] **Step 2: Implement `parse_atom` in `src/parse/expr.rs`**

```rust
use super::cursor::Cursor;
use crate::ast::{Expr, ExprKind};
use crate::error::{Error, ErrorKind};
use crate::span::{Span, Spanned};
use crate::token::TokenKind;

pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    parse_atom(cur)  // Pratt added in Task 17
}

pub(super) fn parse_atom(cur: &mut Cursor) -> Result<Expr, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::IntLit(n)        => { cur.bump(); Ok(Spanned { span: start, node: ExprKind::IntLit(n) }) }
        TokenKind::FloatLit(n)      => { cur.bump(); Ok(Spanned { span: start, node: ExprKind::FloatLit(n) }) }
        TokenKind::StringLit(s)     => { cur.bump(); Ok(Spanned { span: start, node: ExprKind::StringLit(s) }) }
        TokenKind::LowerIdent(s)    => { cur.bump(); Ok(Spanned { span: start, node: ExprKind::Var(s) }) }
        TokenKind::UpperIdent(s)    => { cur.bump(); Ok(Spanned { span: start, node: ExprKind::Ctor(s) }) }
        TokenKind::LParen => {
            cur.bump();
            let e = parse_expr(cur)?;
            let close = cur.expect(TokenKind::RParen, "`)`")?.span;
            Ok(Spanned { span: start.merge(close), node: ExprKind::Paren(Box::new(e)) })
        }
        TokenKind::LBracket => {
            cur.bump();
            let mut items = Vec::new();
            if !cur.check(&TokenKind::RBracket) {
                items.push(parse_expr(cur)?);
                while cur.eat(&TokenKind::Comma) {
                    items.push(parse_expr(cur)?);
                }
            }
            let close = cur.expect(TokenKind::RBracket, "`]`")?.span;
            Ok(Spanned { span: start.merge(close), node: ExprKind::List(items) })
        }
        _ => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", cur.peek_kind()),
                expected: "an expression",
            },
        }),
    }
}
```

- [ ] **Step 3: Expose test helper in `src/parse/mod.rs`**

```rust
#[doc(hidden)]
pub fn parse_expr_for_test(toks: &[Token]) -> Result<crate::ast::Expr, Error> {
    let mut cur = Cursor::new(toks);
    expr::parse_expr(&mut cur)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test parser_atoms`
Expected: PASS (after `Display` on `ExprKind` exists from Task 14).

- [ ] **Step 5: Commit**

```bash
git add src/parse/expr.rs src/parse/mod.rs tests/parser_atoms.rs
git commit -m "Plan 2 Task 16: parse atomic expressions"
```

---

#### Task 17: Pratt expression parser

**Files:**
- Modify: `src/parse/expr.rs`
- Create: `tests/parser_precedence.rs`

This is the core. Implement a Pratt loop that handles ranks 5–10 of the precedence table (comparison, concat, +/-, */ , ^, unary -). Lambdas (rank 1) and logical ops (ranks 2–4) come in Task 18; calls and postfix in Task 19.

- [ ] **Step 1: Write the failing tests**

```rust
// tests/parser_precedence.rs
use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn add() { assert_eq!(p("1 + 2"), "(+ (int 1) (int 2))"); }
#[test]
fn add_left_assoc() { assert_eq!(p("1 + 2 + 3"), "(+ (+ (int 1) (int 2)) (int 3))"); }
#[test]
fn mul_higher_than_add() { assert_eq!(p("1 + 2 * 3"), "(+ (int 1) (* (int 2) (int 3)))"); }
#[test]
fn pow_right_assoc() { assert_eq!(p("2 ^ 3 ^ 2"), "(^ (int 2) (^ (int 3) (int 2)))"); }
#[test]
fn unary_minus() { assert_eq!(p("-3"), "(neg (int 3))"); }
#[test]
fn compare_non_assoc() {
    let toks = lex("a < b < c").unwrap();
    let err = parse_expr_for_test(&toks).unwrap_err();
    assert!(matches!(err.kind, i_lang::error::ErrorKind::ChainedComparison));
}
#[test]
fn concat_right() { assert_eq!(p(r#""a" ++ "b" ++ "c""#), r#"(++ (str "a") (++ (str "b") (str "c")))"#); }
```

- [ ] **Step 2: Implement Pratt**

The standard pattern: `parse_expr_bp(cur, min_bp)`. Each operator has a left-binding-power (`lbp`) and right-binding-power (`rbp`); right-assoc means `rbp = lbp - 1`, left-assoc means `rbp = lbp`. Non-assoc is detected by re-checking after the loop.

```rust
pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    parse_expr_bp(cur, 0)
}

fn parse_expr_bp(cur: &mut Cursor, min_bp: u8) -> Result<Expr, Error> {
    // Prefix: unary minus, then atom.
    let mut lhs = if cur.eat(&TokenKind::Minus) {
        let rhs = parse_expr_bp(cur, 100)?;  // unary minus rank 10 — tighter than infix
        let span = rhs.span;
        Spanned { span, node: ExprKind::UnaryOp { op: UnaryOp::Neg, expr: Box::new(rhs) } }
    } else {
        parse_atom(cur)?
    };

    loop {
        let (op, lbp, rbp) = match infix_for(cur.peek_kind()) {
            Some(t) => t,
            None => break,
        };
        if lbp < min_bp { break; }

        // Non-associative: after consuming one rhs, check we're not seeing the
        // same precedence band again.
        if matches!(op, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge) {
            cur.bump();
            let rhs = parse_expr_bp(cur, rbp)?;
            let span = lhs.span.merge(rhs.span);
            if let Some((_, _, _)) = infix_for(cur.peek_kind()).filter(|(o, _, _)|
                matches!(o, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge))
            {
                return Err(Error { span: cur.peek().span, kind: ErrorKind::ChainedComparison });
            }
            lhs = Spanned { span, node: ExprKind::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) } };
            continue;
        }

        cur.bump();
        let rhs = parse_expr_bp(cur, rbp)?;
        let span = lhs.span.merge(rhs.span);
        lhs = Spanned { span, node: ExprKind::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) } };
    }

    Ok(lhs)
}

fn infix_for(k: &TokenKind) -> Option<(BinOp, u8, u8)> {
    use TokenKind::*;
    Some(match k {
        // (op, lbp, rbp)
        EqEq    => (BinOp::Eq,     50, 51),  // non-assoc: handled specially above
        SlashEq => (BinOp::Ne,     50, 51),
        Lt      => (BinOp::Lt,     50, 51),
        LtEq    => (BinOp::Le,     50, 51),
        Gt      => (BinOp::Gt,     50, 51),
        GtEq    => (BinOp::Ge,     50, 51),
        PlusPlus => (BinOp::Concat, 60, 60),  // right-assoc
        Plus    => (BinOp::Add,    70, 71),  // left-assoc
        Minus   => (BinOp::Sub,    70, 71),
        Star    => (BinOp::Mul,    80, 81),
        Slash   => (BinOp::Div,    80, 81),
        Caret   => (BinOp::Pow,    90, 90),  // right-assoc
        _ => return None,
    })
}
```

(Right-assoc when `rbp == lbp`. Left-assoc when `rbp = lbp + 1`. Pick a convention and stick with it; both work as long as the comparison `lbp < min_bp` uses strict `<`.)

Add `ChainedComparison` to `ErrorKind`.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/expr.rs src/error.rs tests/parser_precedence.rs
git commit -m "Plan 2 Task 17: Pratt expression parser for arithmetic and comparison"
```


---

#### Task 18: Logical operators and lambda

**Files:**
- Modify: `src/parse/expr.rs`
- Modify: `tests/parser_precedence.rs`

Adds ranks 1–4 of the precedence table: lambda (lowest), `or`, `and`, `not`. Lambda is right-associative and greedy; the body `parse_expr_bp(cur, 0)` consumes everything until an outer construct claims the next token (per `syntax.md § 5`).

- [ ] **Step 1: Add the failing tests**

```rust
#[test]
fn lambda_simple() {
    assert_eq!(p("x -> x + 1"),
        "(lambda ((var x)) (+ (var x) (int 1)))");
}

#[test]
fn lambda_multi_param() {
    assert_eq!(p("a b -> a + b"),
        "(lambda ((var a) (var b)) (+ (var a) (var b)))");
}

#[test]
fn lambda_body_greedy() {
    // The body consumes the whole remaining expression at this level.
    assert_eq!(p("x -> x + 1 + 2"),
        "(lambda ((var x)) (+ (+ (var x) (int 1)) (int 2)))");
}

#[test]
fn or_is_left_or_right() {
    // 'or' is rank 2; spec doesn't specify assoc — treat as left for now,
    // since it desugars to a function call which is left-by-juxtaposition.
    assert_eq!(p("a or b or c"),
        "(or (or (var a) (var b)) (var c))");
}

#[test]
fn and_higher_than_or() {
    assert_eq!(p("a or b and c"),
        "(or (var a) (and (var b) (var c)))");
}

#[test]
fn not_prefix() {
    assert_eq!(p("not x"), "(not (var x))");
}
```

- [ ] **Step 2: Extend the Pratt loop**

In `parse_expr_bp`, add a prefix branch for `KwNot` (rank 4). Add infix entries for `KwAnd` (rank 3) and `KwOr` (rank 2):

```rust
KwOr  => (BinOp::Or,  20, 21),
KwAnd => (BinOp::And, 30, 31),
```

For lambda (rank 1, lowest of all), do *not* fold it into Pratt — give it its own front-end:

```rust
pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    // Try lambda first: <param-list> '->' <expr>
    if looks_like_lambda(cur) {
        return parse_lambda(cur);
    }
    parse_expr_bp(cur, 0)
}

fn looks_like_lambda(cur: &Cursor) -> bool {
    // A lambda starts with one or more LowerIdent (or a tuple pattern)
    // followed by Arrow. Single-token lookahead is not enough; scan ahead.
    let mut i = 0;
    loop {
        match cur.peek_n(i) {
            Some(TokenKind::LowerIdent(_)) => { i += 1; }
            Some(TokenKind::LParen) => return false, // tuple-pattern lambda: handled later
            Some(TokenKind::Arrow) if i > 0 => return true,
            _ => return false,
        }
    }
}
```

(Add `peek_n(usize) -> Option<&TokenKind>` to `Cursor`.)

`parse_lambda` reads space-separated lower idents as param patterns, then `->`, then `parse_expr_bp(cur, 0)` for the body.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/expr.rs src/parse/cursor.rs tests/parser_precedence.rs
git commit -m "Plan 2 Task 18: lambda, and/or/not"
```

---

#### Task 19: Calls and postfix operators

**Files:**
- Modify: `src/parse/expr.rs`
- Create: `tests/parser_calls.rs`

Calls are juxtaposition: `f a, b, c` means `f` applied to `[a, b, c]`. Postfix `.field`, `!`, `?` bind tighter than calls per the precedence table (rank 12 vs 11).

- [ ] **Step 1: Write the failing tests**

```rust
// tests/parser_calls.rs
use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn paren_free_call() {
    assert_eq!(p("add 3, 4"), "(call (var add) (int 3) (int 4))");
}

#[test]
fn nested_call_with_parens() {
    assert_eq!(p("add 3, (mul 4, 5)"),
        "(call (var add) (int 3) (paren (call (var mul) (int 4) (int 5))))");
}

#[test]
fn field_access() {
    assert_eq!(p("p.x"), "(. (var p) x)");
}

#[test]
fn method_chain_atom_only() {
    // .filter binds to double, NOT to (nums.map double)
    assert_eq!(p("nums.map double.filter pred"),
        "(call (. (var nums) map) (call (. (var double) filter) (var pred)))");
}

#[test]
fn chain_on_call_result_needs_parens() {
    assert_eq!(p("(nums.map double).filter pred"),
        "(call (. (paren (call (. (var nums) map) (var double))) filter) (var pred))");
}

#[test]
fn postfix_bang() {
    assert_eq!(p("print! \"hi\""),
        r#"(call (! (var print)) (str "hi"))"#);
}

#[test]
fn postfix_question() {
    assert_eq!(p("parseInt s?"),
        "(call (var parseInt) (? (var s)))");
}
```

- [ ] **Step 2: Implement**

Restructure the prefix parsing in Pratt so that after `parse_atom`, postfix operators (`.`, `!`, `?`) bind first, then call juxtaposition wraps the result.

```rust
fn parse_postfix(cur: &mut Cursor) -> Result<Expr, Error> {
    let mut e = parse_atom(cur)?;
    loop {
        match cur.peek_kind() {
            TokenKind::Dot => {
                cur.bump();
                let name = match cur.peek_kind().clone() {
                    TokenKind::LowerIdent(n) | TokenKind::UpperIdent(n) => { cur.bump(); n }
                    _ => return Err(/* expected ident after '.' */),
                };
                let span = e.span.merge(cur.peek().span);
                e = Spanned { span, node: ExprKind::FieldAccess { receiver: Box::new(e), field: name } };
            }
            TokenKind::Bang => {
                cur.bump();
                let span = e.span.merge(cur.peek().span);
                e = Spanned { span, node: ExprKind::Bang(Box::new(e)) };
            }
            TokenKind::Question => {
                cur.bump();
                let span = e.span.merge(cur.peek().span);
                e = Spanned { span, node: ExprKind::Question(Box::new(e)) };
            }
            _ => break,
        }
    }
    Ok(e)
}

fn parse_call(cur: &mut Cursor) -> Result<Expr, Error> {
    let func = parse_postfix(cur)?;
    if !starts_call_arg(cur.peek_kind()) {
        return Ok(func);
    }
    let mut args = vec![parse_expr_bp(cur, 0)?]; // body of first arg
    while cur.eat(&TokenKind::Comma) {
        args.push(parse_expr_bp(cur, 0)?);
    }
    let span = func.span.merge(args.last().unwrap().span);
    Ok(Spanned { span, node: ExprKind::Call { func: Box::new(func), args } })
}

fn starts_call_arg(k: &TokenKind) -> bool {
    matches!(k,
        TokenKind::IntLit(_) | TokenKind::FloatLit(_) | TokenKind::StringLit(_)
        | TokenKind::LowerIdent(_) | TokenKind::UpperIdent(_)
        | TokenKind::LParen | TokenKind::LBracket
    )
}
```

Replace `parse_atom` calls inside `parse_expr_bp` with `parse_call`.

The call-argument body uses `parse_expr_bp(cur, 0)` because lambdas (and everything else) are valid arguments; the comma terminator is handled by the caller, since `,` is not in `infix_for`.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/expr.rs tests/parser_calls.rs
git commit -m "Plan 2 Task 19: calls and postfix operators"
```

---

#### Task 20: Construction and record update

**Files:**
- Modify: `src/parse/expr.rs`
- Create: `tests/parser_construction.rs`

`Type(field = val, ...)` is construction (head is an `UpperIdent`); `instance(field = val, ...)` is record update (head is anything else). Detect by the head; the syntax inside the parens is identical.

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn construction() {
    assert_eq!(p("Point(x = 0, y = 0)"),
        "(construct Point (kw x (int 0)) (kw y (int 0)))");
}

#[test]
fn update() {
    assert_eq!(p("p1(x = 5)"),
        "(update (var p1) (kw x (int 5)))");
}

#[test]
fn nested_construction() {
    assert_eq!(p("Pair(left = Point(x = 0, y = 0), right = None)"),
        "(construct Pair (kw left (construct Point (kw x (int 0)) (kw y (int 0)))) (kw right (ctor None)))");
}
```

- [ ] **Step 2: Implement in `parse_postfix`**

After parsing an atom, if the next token is `LParen` *and* the immediate next non-paren token is a `LowerIdent '='` pattern, treat it as kwargs. Otherwise leave the `LParen` for call-argument parsing... no, actually: the cleanest split is to check the head's shape *before* postfix. If the atom was a `Ctor`, and the next token is `LParen`, it's construction (kwargs required, parens required). If the atom was anything else and the next token is `LParen` followed eventually by a `=`, it's update.

Pragmatic rule: in `parse_postfix`, after building `e`, check `LParen`:

```rust
TokenKind::LParen if looks_like_kwargs(cur) => {
    cur.bump();
    let kwargs = parse_kwargs(cur)?;
    cur.expect(TokenKind::RParen, "`)`")?;
    let span = e.span.merge(cur.peek().span);
    e = match &e.node {
        ExprKind::Ctor(name) => Spanned {
            span,
            node: ExprKind::Construct { type_name: name.clone(), fields: kwargs },
        },
        _ => Spanned {
            span,
            node: ExprKind::Update { value: Box::new(e), fields: kwargs },
        },
    };
}
```

`looks_like_kwargs` peeks ahead: after `LParen`, do we see `LowerIdent` then `Equals`? If yes → kwargs. If no → not kwargs (let it fall through to be treated as a paren-grouped first call argument).

`parse_kwargs` reads `LowerIdent '=' expr` separated by commas.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS, including all earlier tests.

- [ ] **Step 4: Commit**

```bash
git add src/parse/expr.rs tests/parser_construction.rs
git commit -m "Plan 2 Task 20: construction and record update"
```


---

#### Task 21: Patterns

**Files:**
- Create: `src/parse/pat.rs` (replacing the stub)
- Create: `tests/parser_patterns.rs`

Patterns appear in three places: match arms (`Task 22`), lambda params (in scope already, but currently only `LowerIdent`), and tuple/record destructuring inside both. This task implements them as a standalone module.

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::lex::lex;
use i_lang::parse::parse_pattern_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_pattern_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn wildcard()    { assert_eq!(p("_"),       "(wild)"); }
#[test]
fn var_pattern() { assert_eq!(p("x"),       "(pvar x)"); }
#[test]
fn int_pattern() { assert_eq!(p("42"),      "(plit (int 42))"); }
#[test]
fn ctor_no_args() { assert_eq!(p("None"),   "(pctor None)"); }
#[test]
fn ctor_with_args() {
    assert_eq!(p("Some x"),                 "(pctor Some (pvar x))");
}
#[test]
fn tuple_pattern() {
    assert_eq!(p("(a, b)"),                 "(ptuple (pvar a) (pvar b))");
}
#[test]
fn list_pattern() {
    assert_eq!(p("[a, b]"),                 "(plist (pvar a) (pvar b))");
}
#[test]
fn record_pattern() {
    assert_eq!(p("Point(x = a, y = b)"),
        "(precord Point (pf x (pvar a)) (pf y (pvar b)))");
}
```

- [ ] **Step 2: Implement `parse_pattern` in `src/parse/pat.rs`**

Logic by leading token:
- `Underscore` → `Wildcard`. (Wait — `_` lexes as `LowerIdent("_")`. Special-case the string.)
- `LowerIdent("_")` → `Wildcard`
- `LowerIdent(n)` → `Var(n)`
- `IntLit/FloatLit/StringLit` → `Lit`
- `UpperIdent(n)`:
  - if next is `LParen` followed by `LowerIdent =` → `Record`
  - if next starts a pattern (ident, lit, etc) → `Ctor` with comma-separated args
  - else → bare `Ctor` with no args
- `LParen` → tuple pattern (parse comma-separated patterns until `)`)
- `LBracket` → list pattern (parse comma-separated patterns until `]`)

- [ ] **Step 3: Expose test helper**

```rust
#[doc(hidden)]
pub fn parse_pattern_for_test(toks: &[Token]) -> Result<crate::ast::Pattern, Error> {
    let mut cur = Cursor::new(toks);
    pat::parse_pattern(&mut cur)
}
```

- [ ] **Step 4: Update `parse_lambda` to use `parse_pattern` for params**

Replace the `LowerIdent`-only loop in lambda parsing with `parse_pattern` calls. Now `(a, b) -> ...` works.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add src/parse/pat.rs src/parse/mod.rs src/parse/expr.rs tests/parser_patterns.rs
git commit -m "Plan 2 Task 21: pattern parser"
```

---

#### Task 22: Match expressions

**Files:**
- Modify: `src/parse/expr.rs`
- Create: `tests/parser_match.rs`

`expr match` is a postfix keyword: scrutinee first, then `match`, then an `Indent` block of `pattern -> body` arms followed by `Dedent`.

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn match_simple() {
    let src = "n match\n    0 -> \"zero\"\n    _ -> \"other\"";
    assert_eq!(p(src),
        "(match (var n) ((arm (plit (int 0)) (str \"zero\")) (arm (wild) (str \"other\"))))");
}

#[test]
fn match_constructor() {
    let src = "shape match\n    Circle r -> r\n    Rect w, h -> w";
    assert_eq!(p(src),
        "(match (var shape) ((arm (pctor Circle (pvar r)) (var r)) (arm (pctor Rect (pvar w) (pvar h)) (var w))))");
}
```

- [ ] **Step 2: Add `KwMatch` as a postfix in `parse_postfix`**

```rust
TokenKind::KwMatch => {
    cur.bump();
    cur.expect(TokenKind::Newline, "newline before match arms")?;
    cur.expect(TokenKind::Indent, "indented match arms")?;
    let mut arms = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        let pattern = pat::parse_pattern(cur)?;
        cur.expect(TokenKind::Arrow, "`->`")?;
        let body = parse_expr_bp(cur, 0)?;
        // Each arm ends with a Newline (or Dedent for the last)
        cur.eat(&TokenKind::Newline);
        arms.push(MatchArm { pattern, body });
    }
    cur.expect(TokenKind::Dedent, "dedent")?;
    let span = e.span; // approximate; merge with last arm body
    e = Spanned { span, node: ExprKind::Match { scrutinee: Box::new(e), arms } };
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/expr.rs tests/parser_match.rs
git commit -m "Plan 2 Task 22: match expressions"
```

---

#### Task 23: Type expressions

**Files:**
- Create: `src/parse/typ.rs` (replacing stub)
- Create: `tests/parser_types.rs`

Types appear in field declarations, signatures, and the `: T` ascription in single-payload variants. Forms:
- `Lower` → type variable (e.g., `a`)
- `Upper` → named type, possibly with args (`List a`, `Tree (Maybe a)`)
- `T1, T2 -> T3` → function type
- `T1, T2 ! Eff -> T3` → function type with effect row
- `(T1, T2)` → tuple type

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::lex::lex;
use i_lang::parse::parse_type_for_test;

fn t(src: &str) -> String {
    format!("{}", parse_type_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn type_var()        { assert_eq!(t("a"),               "(tvar a)"); }
#[test]
fn named_type()      { assert_eq!(t("Int"),             "(tnamed Int)"); }
#[test]
fn parametric()      { assert_eq!(t("List a"),          "(tnamed List (tvar a))"); }
#[test]
fn function_type()   {
    assert_eq!(t("Int, Int -> Int"),
        "(tfun (tnamed Int) (tnamed Int) (tnamed Int))");
}
#[test]
fn effectful_type()  {
    assert_eq!(t("String ! IO -> Unit"),
        "(tfun (tnamed String) (eff IO) (tnamed Unit))");
}
#[test]
fn empty_effect_row() {
    // pure callback: ! () appears between args and result
    assert_eq!(t("(a -> b ! ())"),
        "(tfun (tvar a) (eff-empty) (tvar b))");
}
```

- [ ] **Step 2: Implement**

Recursive descent over types. Effect row appears between args and `->`. Handle `! ()` as `EffectRow::Empty`.

- [ ] **Step 3: Expose test helper, run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/typ.rs src/parse/mod.rs tests/parser_types.rs
git commit -m "Plan 2 Task 23: type expression parser"
```

---

#### Task 24: Bindings and block bodies

**Files:**
- Modify: `src/parse/decl.rs`, `src/parse/expr.rs`
- Create: `tests/parser_bindings.rs`

Bindings have four forms (`syntax.md § 3`):
1. `name : Type`           — sig only
2. `name : Type = expr`    — annotated
3. `name = expr`           — value
4. `name =` newline+indent — block body

Block bodies are also a kind of expression (the `Block` variant) used inside other expressions when followed by indent.

- [ ] **Step 1: Write the failing tests**

```rust
// see body of test for shape; covers all four forms and a nested block body
```

(Author tests against `parse_file_for_test`.)

- [ ] **Step 2: Implement `parse_binding` in `decl.rs`**

```rust
pub(super) fn parse_binding(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    let name = match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => { cur.bump(); n }
        _ => return Err(/* expected identifier */),
    };

    let ty = if cur.eat(&TokenKind::Colon) {
        Some(typ::parse_type(cur)?)
    } else {
        None
    };

    let value = if cur.eat(&TokenKind::Equals) {
        if cur.check(&TokenKind::Newline) {
            cur.bump();
            cur.expect(TokenKind::Indent, "indented block body")?;
            Some(parse_block(cur)?)
        } else {
            Some(expr::parse_expr(cur)?)
        }
    } else {
        None  // sig-only
    };

    cur.eat(&TokenKind::Newline);
    let span = start.merge(cur.peek().span);
    Ok(Spanned { span, node: DeclKind::Binding { name, ty, value } })
}

fn parse_block(cur: &mut Cursor) -> Result<Expr, Error> {
    let start = cur.peek().span;
    let mut items = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        // A block item is either another binding (lookahead: Lower then : or =)
        // or a bare expression.
        if looks_like_binding(cur) {
            items.push(BlockItem::Binding(parse_binding(cur)?));
        } else {
            let e = expr::parse_expr(cur)?;
            cur.eat(&TokenKind::Newline);
            items.push(BlockItem::Expr(e));
        }
    }
    cur.expect(TokenKind::Dedent, "dedent at block end")?;
    let span = start.merge(cur.peek().span);
    Ok(Spanned { span, node: ExprKind::Block(items) })
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/decl.rs src/parse/expr.rs tests/parser_bindings.rs
git commit -m "Plan 2 Task 24: bindings and block bodies"
```


---

#### Task 25: Type, trait, and impl declarations

**Files:**
- Modify: `src/parse/decl.rs`
- Create: `tests/parser_typedecls.rs`

Forms (`syntax.md § 6, § 12`):
- `type Name params? = T`                    — newtype short form
- `type Name params?` newline indent block   — fields/methods/variants
- `trait Name a` newline indent sigs         — trait
- `impl Trait Type` newline indent methods   — impl

Variants come in three sub-forms: bare, `: T` single-payload, indented field block.

- [ ] **Step 1: Write the failing tests**

Tests over each variant of each form, asserting against `Display` of the parsed `Decl`.

- [ ] **Step 2: Implement**

`parse_type_decl`:
1. Eat `KwType`, then `UpperIdent` for name, then optional comma-separated `LowerIdent` params.
2. If next is `Equals`: parse a single `Type`, build `TypeBody::Newtype`.
3. Else expect `Newline Indent` and read members until `Dedent`.

A type member is one of:
- `LowerIdent : Type` → `Field`
- `LowerIdent = Expr` → `Method` (a `Decl::Binding`)
- `UpperIdent` → variant; sub-shape determined by what follows:
  - `Newline` (or end of block) → `VariantBody::Bare`
  - `Colon` → `VariantBody::Single(Type)`
  - `Newline Indent` → `VariantBody::Fields(...)` (recursive on fields)

`parse_trait_decl`:
- `KwTrait UpperIdent LowerIdent Newline Indent <method-sig>+ Dedent`
- A method sig is a `Binding` with `ty: Some(...)` and `value: None`.

`parse_impl_decl`:
- `KwImpl UpperIdent <type-expr> Newline Indent <method>+ Dedent`
- A method is a `Binding` with `value: Some(...)`.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/decl.rs tests/parser_typedecls.rs
git commit -m "Plan 2 Task 25: type, trait, impl declarations"
```

---

#### Task 26: Module header and use declarations

**Files:**
- Modify: `src/parse/decl.rs`
- Create: `tests/parser_modules.rs`

`parse_file` becomes the real entry point: optional `module` header, then a stream of decls (binding / type / trait / impl / use) at the top level.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn module_header() {
    let src = "module Main\n    expose main, helper\n";
    let f = parse(src);
    assert_eq!(f.module.unwrap().exposes.len(), 2);
}

#[test]
fn use_whole() {
    assert_eq!(parse_use("use Std.IO"), "(use Std.IO whole)");
}

#[test]
fn use_cherry() {
    assert_eq!(parse_use("use Std.IO (print, readLine)"),
        "(use Std.IO (cherry print readLine))");
}

#[test]
fn use_alias() {
    assert_eq!(parse_use("use Std.Float as F"),
        "(use Std.Float (alias F))");
}

#[test]
fn expose_with_constructors() {
    let src = "module Geometry\n    expose Point(..), distance\n";
    let f = parse(src);
    let exp = &f.module.unwrap().exposes;
    assert!(matches!(&exp[0], Expose::Type { with_constructors: true, .. }));
    assert!(matches!(&exp[1], Expose::Value(_)));
}
```

- [ ] **Step 2: Implement**

```rust
pub(super) fn parse_file(cur: &mut Cursor) -> Result<File, Error> {
    let module = if cur.check(&TokenKind::KwModule) {
        Some(parse_module_header(cur)?)
    } else {
        None
    };
    let mut decls = Vec::new();
    while !cur.at_end() {
        decls.push(parse_top_decl(cur)?);
    }
    Ok(File { module, decls })
}

fn parse_module_header(cur: &mut Cursor) -> Result<ModuleHeader, Error> {
    cur.expect(TokenKind::KwModule, "`module`")?;
    let name = expect_upper(cur, "module name")?;
    cur.expect(TokenKind::Newline, "newline after module name")?;
    cur.expect(TokenKind::Indent, "indented expose clause")?;
    cur.expect(TokenKind::KwExpose, "`expose`")?;
    let exposes = parse_expose_list(cur)?;
    cur.eat(&TokenKind::Newline);
    cur.expect(TokenKind::Dedent, "dedent")?;
    Ok(ModuleHeader { name, exposes })
}

fn parse_top_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    match cur.peek_kind() {
        TokenKind::KwUse   => parse_use_decl(cur),
        TokenKind::KwType  => parse_type_decl(cur),
        TokenKind::KwTrait => parse_trait_decl(cur),
        TokenKind::KwImpl  => parse_impl_decl(cur),
        TokenKind::LowerIdent(_) => parse_binding(cur),
        _ => Err(/* unexpected token at top level */),
    }
}

fn parse_use_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    cur.bump(); // KwUse
    let mut path = vec![expect_upper(cur, "module path")?];
    while cur.eat(&TokenKind::Dot) {
        path.push(expect_upper(cur, "module path segment")?);
    }
    let kind = if cur.eat(&TokenKind::KwAs) {
        UseKind::Alias(expect_upper(cur, "alias")?)
    } else if cur.eat(&TokenKind::LParen) {
        let mut names = Vec::new();
        names.push(expect_lower(cur, "name")?);
        while cur.eat(&TokenKind::Comma) {
            names.push(expect_lower(cur, "name")?);
        }
        cur.expect(TokenKind::RParen, "`)`")?;
        UseKind::Cherry(names)
    } else {
        UseKind::Whole
    };
    cur.eat(&TokenKind::Newline);
    let span = start.merge(cur.peek().span);
    Ok(Spanned { span, node: DeclKind::Use { path, kind } })
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parse/decl.rs tests/parser_modules.rs
git commit -m "Plan 2 Task 26: module header, use, and top-level dispatch"
```

---

#### Task 26.5: Split AST and parser modules (refactor — no behaviour change)

**Files:**
- Modify/split: `src/ast.rs`, `src/parse/expr.rs`, `src/parse/decl.rs`
- Likely new: `src/ast/` directory (or sibling files), `src/parse/postfix.rs` (or similar)

By this point the parser surface is feature-complete and the largest files
are carrying multiple concerns each. `ast.rs` mixes data types with the
Printer/Display machinery; `parse/expr.rs` mixes Pratt core with postfix
operators, calls, construct/update, match, and atoms; `parse/decl.rs`
mixes bindings, blocks, type decls, trait/impl, modules, and use. Task 29
(pretty printer) will add another big chunk to whichever module owns
output — splitting now means it lands in a clean home.

No tests change; the full existing suite is the regression check. The aim
is readability, not architectural reshuffling — keep public APIs
unchanged so the rest of the crate doesn't move.

- [ ] **Step 1: Inventory and target line counts**

Run `wc -l src/**/*.rs` and identify the files over ~300 lines. For each,
note the distinct concerns living in it. The goal is roughly one concern
per file and no file dominating the project. Don't pre-commit to a
specific split — pick what's natural given the actual code.

Likely candidates at this point:

- **`src/ast.rs`** — split the Printer/Display code out of the data-types
  file. Either `src/ast/printer.rs` (with `ast.rs` becoming `ast/mod.rs`
  exporting the same items) or a sibling `src/ast_display.rs`. The data
  types are the public surface; the printer is implementation detail.
- **`src/parse/expr.rs`** — Pratt loop and lambda detection are one
  concern; calls, postfix `.` `!` `?`, construct/update, and match arms
  are another. Pull the postfix family into `src/parse/postfix.rs` (or
  similar). Atoms might also move out depending on how it looks.
- **`src/parse/decl.rs`** — bindings and blocks are one concern; type
  decls (with members, variants, recursion) are another; trait/impl/use
  are a third. Type decls are the natural split candidate.

- [ ] **Step 2: Perform the split, file by file**

Move code, adjust `mod` declarations and `use` paths. Keep visibility
(`pub`, `pub(super)`, `pub(crate)`) the same on every moved item so no
caller breaks. Run `cargo build` after each file's split to catch
breakage before piling on the next.

If `ast.rs` becomes `ast/mod.rs`, re-export so `use crate::ast::ExprKind`
still works.

- [ ] **Step 3: Run the full test suite**

Run: `make ci`
Expected: every prior test still passes; no new tests added; clippy and
fmt clean.

- [ ] **Step 4: Commit**

```bash
git add src/ast* src/parse
git commit -m "Plan 2 Task 26.5: split AST and parser modules (refactor)"
```

---

#### Task 27: Parser error tests

**Files:**
- Create: `tests/parser_errors.rs`

Hand-written assertions for the cases where the *exact* error matters.

- [ ] **Step 1: Write the tests**

```rust
use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;

fn parse_err(src: &str) -> ErrorKind {
    parse(&lex(src).unwrap()).unwrap_err().kind
}

#[test]
fn chained_comparison() {
    assert!(matches!(parse_err("x = a < b < c\n"), ErrorKind::ChainedComparison));
}

#[test]
fn match_without_indent() {
    let err = parse_err("x = n match\n");
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}

#[test]
fn missing_paren() {
    let err = parse_err("x = (1 + 2\n");
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}

#[test]
fn empty_match() {
    let src = "x = n match\n    \n";  // indented but no arms
    let err = parse_err(src);
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}
```

- [ ] **Step 2: Run**

Run: `cargo test --test parser_errors`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/parser_errors.rs
git commit -m "Plan 2 Task 27: parser error assertions"
```

---

#### Task 28: Parser corpus snapshot tests

**Files:**
- Create: `tests/parser_corpus.rs`
- Create: `tests/corpus/parser/*.i` (one per syntactic form)

- [ ] **Step 1: Write a corpus file per form**

`tests/corpus/parser/lambda-multiline.i`:
```i
result =
    xs.fold initial, acc x ->
        cleaned = clean x
        acc.append cleaned
```

`tests/corpus/parser/match-nested.i`:
```i
shape = m match
    Some (Cons head, _) -> head
    _                   -> 0
```

(Continue per the file structure list under `tests/corpus/parser/`.)

- [ ] **Step 2: Add the corpus harness**

```rust
// tests/parser_corpus.rs
use i_lang::lex::lex;
use i_lang::parse::parse;

#[test]
fn snapshot_examples() {
    insta::glob!("../examples/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).unwrap();
        let file = parse(&toks).unwrap();
        insta::assert_snapshot!(format!("{}", file));
    });
}

#[test]
fn snapshot_corpus() {
    insta::glob!("corpus/parser/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).unwrap();
        let file = parse(&toks).unwrap();
        insta::assert_snapshot!(format!("{}", file));
    });
}
```

- [ ] **Step 3: Review snapshots**

Run: `cargo test --test parser_corpus`
Expected: tests fail (snapshots pending).

Run: `cargo insta review`
Expected: walks each pending snapshot. Read every one against the source. Confirm AST shape matches your mental model of the program.

- [ ] **Step 4: Commit**

```bash
git add tests/parser_corpus.rs tests/corpus/ tests/snapshots/
git commit -m "Plan 2 Task 28: parser corpus snapshots"
```


---

### Phase 5 — Round trip

#### Task 29: Pretty printer

**Files:**
- Create: `src/pretty.rs`
- Modify: `src/lib.rs`
- Create: `tests/pretty.rs`

The pretty-printer turns an AST back into `i` source. It doesn't have to match the original byte-for-byte (whitespace and comment placement are lost), but it has to round-trip: `parse(pp(parse(s))) == parse(s)` modulo spans.

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::pretty::pretty;

fn rt(src: &str) -> String {
    let ast = parse(&lex(src).unwrap()).unwrap();
    pretty(&ast)
}

#[test]
fn simple_binding() {
    assert!(rt("x = 1\n").contains("x = 1"));
}

#[test]
fn lambda() {
    assert!(rt("add = a b -> a + b\n").contains("a b -> a + b"));
}

#[test]
fn type_block() {
    let src = "type Point\n    x : Float\n    y : Float\n";
    let out = rt(src);
    assert!(out.contains("type Point"));
    assert!(out.contains("x : Float"));
}
```

- [ ] **Step 2: Implement**

A `Printer` struct holding a `String` buffer and indent depth (in spaces). Each AST node gets a print rule. Use the precedence ranks from the table to decide when to wrap with parens — a child whose precedence is lower than the parent's needs parens. (Track parent precedence as a parameter, like `print_expr(&mut self, e: &Expr, parent_bp: u8)`.)

Print conventions:
- 4-space indent per level
- Bindings: `name = value` on one line; if value is a `Block`, write `name =\n` then indent and print items
- Lambdas: `params -> body` on one line
- `match`: `scrutinee match\n` then indented arms, each `pattern -> body`
- Construction: `Type(field = value, ...)`
- Type blocks: `type Name params\n` then indented members

The hard part is keeping line continuation valid. As long as the printer always uses the indented-block form for blocks (never tries to put a block on one line), and never breaks expressions across lines, the output is parseable.

- [ ] **Step 3: Run tests**

Run: `cargo test --test pretty`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/pretty.rs src/lib.rs tests/pretty.rs
git commit -m "Plan 2 Task 29: pretty printer"
```

---

#### Task 30: Round-trip property test

**Files:**
- Create: `tests/roundtrip.rs`

Per the testing strategy: use the corpus as the property-test seed, not a generator. The property is `parse(pp(parse(src))) == parse(src)` modulo spans.

- [ ] **Step 1: Write the test**

```rust
use i_lang::ast::File;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::pretty::pretty;

fn parse_str(src: &str) -> File {
    parse(&lex(src).unwrap()).unwrap()
}

fn roundtrip(src: &str) {
    let ast1 = parse_str(src);
    let printed = pretty(&ast1);
    let ast2 = parse_str(&printed);
    assert!(ast1.node_eq(&ast2),
        "round-trip differs for source:\n{}\nprinted:\n{}", src, printed);
}

#[test]
fn examples_roundtrip() {
    for entry in std::fs::read_dir("examples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("i") { continue; }
        let src = std::fs::read_to_string(&path).unwrap();
        eprintln!("round-tripping {}", path.display());
        roundtrip(&src);
    }
}

#[test]
fn corpus_roundtrip() {
    for entry in std::fs::read_dir("tests/corpus/parser").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("i") { continue; }
        let src = std::fs::read_to_string(&path).unwrap();
        eprintln!("round-tripping {}", path.display());
        roundtrip(&src);
    }
}
```

- [ ] **Step 2: Run**

Run: `cargo test --test roundtrip`
Expected: PASS. If any source fails to round-trip, investigate: usually it's the pretty-printer dropping a paren that the parser then reads differently. Fix `pretty.rs` and re-run.

- [ ] **Step 3: Commit**

```bash
git add tests/roundtrip.rs
git commit -m "Plan 2 Task 30: round-trip property test over corpus"
```

---

### Phase 6 — Documentation

#### Task 31: Document the testing strategy

**Files:**
- Create: `docs/testing.md`
- Modify: `docs/README.md`

Capture the test architecture as durable documentation so future plans don't have to re-derive it.

- [ ] **Step 1: Create `docs/testing.md`**

Content (copy/adapt from this plan's "Testing strategy" section):

```markdown
# Testing strategy

The compiler has three test layers, each with a defined role.

## Layer 1 — Insta corpus tests

[copy of "Layer 1" section, adjusted to remove forward-references]

## Layer 2 — Hand-written assertion tests

[copy of "Layer 2" section]

## Layer 3 — Round-trip property test

[copy of "Layer 3" section]

## Conventions

- Snapshot files live in `tests/snapshots/` and are committed.
- Use `cargo insta review`, never `cargo insta accept`.
- Hand-written assertion tests live alongside snapshot tests in `tests/` and use plain `assert_eq!` / `matches!`.
- The corpus is `examples/` plus `tests/corpus/parser/`. Add a corpus file when introducing a new syntactic form.
```

- [ ] **Step 2: Link from `docs/README.md`**

Add a row to the existing docs index:

```markdown
- [Testing strategy](testing.md) — how the compiler is tested, layer by layer
```

- [ ] **Step 3: Update `PROGRESS.md`**

In `docs/superpowers/plans/PROGRESS.md`, mark Plan 2 done:

```markdown
- [x] Lexer + parser — Plan 2 (2026-04-29-plan-2-lexer-parser.md)
```

- [ ] **Step 4: Commit**

```bash
git add docs/testing.md docs/README.md docs/superpowers/plans/PROGRESS.md
git commit -m "Plan 2 Task 31: document testing strategy"
```

---

## Acceptance

After Plan 2:

- `cargo build` is clean.
- `cargo test` passes — including snapshot tests over `examples/` and `tests/corpus/parser/`, hand-written error-case tests, and the round-trip property test.
- Every `.i` file in `examples/` lexes and parses without error.
- Every syntactic form in `docs/syntax.md` has a corpus file under `tests/corpus/parser/` and a snapshot.
- `docs/testing.md` documents the three-layer strategy.
- `PROGRESS.md` marks Plan 2 done.
- The AST in `src/ast.rs` is the contract Plan 3 (name resolution) consumes — span-bearing, public, and stable enough that Plan 3 won't have to refactor it.

## Out of scope (deferred to later plans)

- Name resolution (Plan 3)
- Type checking (Plan 4)
- Effect inference (Plan 4 or its own plan)
- Interpretation / codegen (Plan 5)
- Standard library (Plan 6)
- CLI driver beyond `cargo build` (Plan 7)
- Diagnostic rendering with source snippets (Plan 7 — for now, errors are `Error { span, kind }` debug-printed)
- Real proptest *generator* of valid programs — only the corpus-seeded round-trip is in scope here
