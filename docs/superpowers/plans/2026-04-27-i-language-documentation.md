# i Language Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the `i` language Rust project skeleton and write the complete end-state user-facing documentation. After this plan, the docs describe a fully functional language; the implementation does not exist yet. Subsequent plans will implement the language to match the docs.

**Architecture:** Cargo workspace with two crates planned (`i-lang` library, `i` binary). Documentation lives as plain markdown in `docs/`. Code examples live as standalone `.i` files in `examples/` and are referenced from docs. No doc site generator yet — plain markdown viewed on GitHub or locally is sufficient for v1.

**Tech Stack:** Rust 1.75+ (Cargo), plain markdown for docs, `.i` extension for example programs. No frameworks, no doc generators, no test runners yet (those come in Plan 2).

**Spec reference:** `docs/superpowers/specs/2026-04-27-i-language-design.md`

---

## File Structure

After this plan, the repo will look like:

```
pare/                           # parent dir; user may rename to i-lang/
├── Cargo.toml                  # workspace root
├── README.md                   # GitHub landing — what is i
├── LICENSE                     # MIT
├── .gitignore                  # Rust + macOS standard
├── docs/
│   ├── README.md               # docs index / nav
│   ├── tour.md                 # narrative learning path
│   ├── syntax.md               # full syntax reference
│   ├── types.md                # type system manual
│   ├── effects.md              # effect system manual
│   ├── patterns.md             # pattern matching reference
│   ├── stdlib.md               # standard library reference
│   ├── modules.md              # modules and imports
│   ├── building.md             # CLI, project layout, how to run
│   ├── limitations.md          # what v1 doesn't do
│   ├── superpowers/            # (already exists) specs, plans
│   └── ...
├── examples/                   # complete .i programs referenced by docs
│   ├── 01-hello.i
│   ├── 02-greet.i
│   ├── 03-shapes.i
│   ├── 04-list-map.i
│   ├── 05-maybe.i
│   ├── 06-result-and-question-mark.i
│   ├── 07-trait.i
│   ├── 08-modules-app.i
│   └── 08-modules-lib.i
├── src/                        # rust sources — empty until Plan 2
│   └── .gitkeep
└── tests/                      # rust tests — empty until Plan 2
    └── .gitkeep
```

**Each doc has one clear responsibility:**
- `tour.md`: introduce the language linearly, no comprehensive coverage
- `syntax.md`: lookup reference for every form, no tutorial narrative
- `types.md`, `effects.md`, `patterns.md`, `modules.md`: deep dives on each axis
- `stdlib.md`: API reference only
- `building.md`: how to install and run, forward-looking (describes commands the impl will provide)
- `limitations.md`: explicit list of out-of-scope features

**Examples (`examples/*.i`) are first-class.** Each example is a complete, runnable program. Docs reference them by path and quote relevant snippets. When the implementation lands, examples become golden tests.

---

## Task 1: Initialize the Rust project skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `LICENSE`
- Create: `src/.gitkeep`
- Create: `tests/.gitkeep`

- [ ] **Step 1: Verify working directory**

Run: `pwd`
Expected: ends in `/code/pare` (or wherever the user has placed the repo).

- [ ] **Step 2: Confirm git is initialized**

Run: `git log --oneline | head -3`
Expected: shows at least the spec commits. If not, run `git init` first.

- [ ] **Step 3: Create `Cargo.toml`**

```toml
[package]
name = "i-lang"
version = "0.1.0"
edition = "2021"
description = "An exercise in sparse-by-default language design"
license = "MIT"
repository = "https://github.com/jonathanthom/i-lang"
readme = "README.md"

[[bin]]
name = "i"
path = "src/main.rs"

[lib]
name = "i_lang"
path = "src/lib.rs"

[dependencies]
# none yet — Plan 2 introduces parser, error reporting

[dev-dependencies]
# none yet
```

- [ ] **Step 4: Create `.gitignore`**

```
# Rust
/target
Cargo.lock

# macOS
.DS_Store

# Editors
.vscode/
.idea/
*.swp
*.swo
```

Note: include `Cargo.lock` for libraries; for binaries you'd typically commit it. Since this is both, defer to library convention until v1 ships, then revisit.

- [ ] **Step 5: Create `LICENSE` (MIT)**

```
MIT License

Copyright (c) 2026 Jonathan Thom

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OF OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 6: Create empty source dirs**

```bash
mkdir -p src tests examples
touch src/.gitkeep tests/.gitkeep
```

- [ ] **Step 7: Verify Cargo recognizes the project**

Run: `cargo check 2>&1 | head -20`
Expected: error about missing `src/main.rs` and `src/lib.rs`. That's fine — we'll add them in Plan 2. The point is Cargo parses `Cargo.toml`.

If `cargo` fails to read the manifest, the error is in the TOML — fix and re-run.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml .gitignore LICENSE src/.gitkeep tests/.gitkeep
git commit -m "Initialize Rust project skeleton

Cargo workspace stub with library + binary entry points; MIT license;
.gitignore. No source files yet — those come in the implementation plans.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Write the README

**Files:**
- Create: `README.md`

The repo's README is what someone sees on GitHub. It should:
- Pitch what `i` is in two sentences
- Show one tiny code sample
- List goals + non-goals
- Link to the docs
- Mention that the language is not yet implemented (set expectations)

- [ ] **Step 1: Create `README.md`**

```markdown
# i

A tiny, statically typed, compiled-ish language whose surface is as sparse as
possible without giving up safety. Think Roc-grade safety in a syntax pared
down to four operators.

```i
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5

main =
    p1 = Point(x = 0, y = 0)
    p2 = Point(x = 3, y = 4)
    print! "distance: " ++ show (p1.distance p2)
```

## What this is

A learning project, designed as a real language. The goal is something usable
at the end — not a toy. The design priorities, in order:

1. **Aesthetic minimalism.** A program looks like the idea it expresses.
2. **Fits in your head.** The whole core can be learned in an afternoon.
3. **Ergonomic.** Sparse to *help* you, not to puzzle you.

## Status

Design complete. Documentation in progress. **The language is not yet
implemented.** This repo currently contains:

- The design spec (`docs/superpowers/specs/`)
- The end-state user docs (`docs/`)
- The implementation plans (`docs/superpowers/plans/`)

The Rust implementation arrives in subsequent plans.

## Documentation

- [Tour](docs/tour.md) — start here
- [Syntax reference](docs/syntax.md)
- [Type system](docs/types.md)
- [Effect system](docs/effects.md)
- [Pattern matching](docs/patterns.md)
- [Standard library](docs/stdlib.md)
- [Modules](docs/modules.md)
- [Building and running](docs/building.md)
- [Limitations](docs/limitations.md)

## License

MIT. See [LICENSE](LICENSE).
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "Add README

Set expectations: design done, docs in progress, no implementation yet.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Create the docs index

**Files:**
- Create: `docs/README.md`

This is the docs landing page (linked from the repo README). It restates the
nav and gives a one-line description of each doc.

- [ ] **Step 1: Create `docs/README.md`**

```markdown
# i — Documentation

The end-state documentation for the `i` language. Some of this describes
features that are not yet implemented. The "Status" line at the top of each
doc indicates whether the implementation is in place.

## Learning path

Read in this order if you're new:

1. [Tour](tour.md) — narrative introduction with runnable examples
2. [Building and running](building.md) — install, project layout, CLI
3. [Pattern matching](patterns.md), then [Types](types.md), then [Effects](effects.md)
4. [Modules](modules.md)
5. [Standard library](stdlib.md) — reference

## Reference (random access)

- [Syntax](syntax.md) — every form, every operator
- [Standard library](stdlib.md) — every type and function in v1
- [Limitations](limitations.md) — what v1 doesn't do

## Specs and plans

- [Design spec](superpowers/specs/2026-04-27-i-language-design.md)
- [Plans](superpowers/plans/) — implementation roadmap
```

- [ ] **Step 2: Commit**

```bash
git add docs/README.md
git commit -m "Add docs index"
```

---

## Task 4: Write the tour

**Files:**
- Create: `docs/tour.md`
- Create: `examples/01-hello.i`
- Create: `examples/02-greet.i`
- Create: `examples/03-shapes.i`
- Create: `examples/04-list-map.i`

The tour is the narrative learning doc. It walks a new reader through the
language end-to-end, building one concept on the previous. ~1500-2500 words.

**Required structure (these section headings, in this order):**

1. **Hello, world** — minimal program, introduces `main`, `print!`, `module Main`, `expose`
2. **Values and bindings** — `=`, immutability, `:` for types when you want them
3. **Functions** — `args -> body`, paren-free calls with commas, parens for nesting
4. **Records** — `type Point` block, fields with `:`, methods with `=` and implicit `self`, construction with kwargs
5. **Sum types and pattern matching** — `type Shape` with variants, `expr match`, exhaustiveness
6. **Maybe and Result** — the safety story, `?` sugar, no exceptions
7. **Effects** — what `!` means, why pure functions can't accidentally do IO
8. **Lists** — literal syntax, `map`, `filter`, `fold`
9. **Modules** — one file = one module, `expose`, `use`
10. **Where to go next** — links to the reference docs

**Required examples (full programs, written into `examples/`):**

`examples/01-hello.i`:
```
module Main
    expose main

main =
    print! "hello, world"
```

`examples/02-greet.i`:
```
module Main
    expose main

use Std.IO (print, readLine)

main =
    print! "what's your name?"
    name = readLine!
    print! "hi, " ++ name
```

`examples/03-shapes.i`:
```
module Main
    expose main

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

area = shape ->
    shape match
        Circle r    -> 3.14159 * r^2
        Rect w, h   -> w * h

main =
    s = Circle(radius = 5.0)
    print! "area: " ++ show (area s)
```

`examples/04-list-map.i`:
```
module Main
    expose main

main =
    nums = [1, 2, 3, 4, 5]
    doubled = nums.map x -> x * 2
    print! "doubled: " ++ show doubled
```

**Tone:** like a friendly README walkthrough. Show, don't lecture. Each section
≤ 200 words plus its example. End each section with one sentence pointing
forward.

- [ ] **Step 1: Write `examples/01-hello.i` through `examples/04-list-map.i`**

Each file contains the complete program shown above. No comments. No extra
content. Files must end with a trailing newline.

- [ ] **Step 2: Write `docs/tour.md`**

Use the structure above. Each section embeds the relevant `examples/*.i` as
a code block (copy-paste; the impl will later either include-by-reference or
extract). Reference the example file path in prose: "see `examples/01-hello.i`."

The tour is the only doc that allowed to be opinionated and tutorial-flavored.
All others are reference style.

Open with a 2-3 sentence preface that says: "this is a tour, not a reference;
for full coverage of any topic, see the linked manual."

- [ ] **Step 3: Read what you wrote**

Skim `docs/tour.md` end-to-end. A reader who knows another statically typed
language but has never seen `i` should be able to follow it without
reaching for the reference. If a sentence required outside knowledge to
parse, rewrite it.

- [ ] **Step 4: Commit**

```bash
git add docs/tour.md examples/01-hello.i examples/02-greet.i examples/03-shapes.i examples/04-list-map.i
git commit -m "docs: write the language tour

Narrative introduction: hello world through modules. Four runnable examples
in examples/ that the tour quotes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Write the syntax reference

**Files:**
- Create: `docs/syntax.md`

This is the lookup doc. No tutorial. Every form gets a heading; under each
heading is: a one-sentence description, the formal-ish syntax, and a tiny
example.

**Required structure:**

1. **Lexical**
   - Comments: `# line ...`
   - Identifiers: lowercase = values + type vars; uppercase = types + variants
   - Whitespace and indentation: indentation marks blocks
   - Newlines: terminate expressions; trailing operator continues
   - Literals: `Int`, `Float`, `String`, list literal `[...]`

2. **Operators (the four)**
   - `:` has-type
   - `=` bind-value
   - `->` function
   - `.` member access
   - Plus parens (grouping) and `!` (effect marker)

3. **Bindings**
   - `name = expr` — value
   - `name : Type` — type annotation (optional)
   - `name : Type = expr` — both at once

4. **Functions**
   - Definition: `args -> body`
   - Type: `args -> result` or `args ! Effect -> result`
   - No currying; multi-arg is one function
   - Lambdas as values

5. **Calls**
   - Paren-free: `f a, b`
   - Parens for nesting: `f a, (g b, c)`
   - Method call: `instance.method args`
   - Construction: `Type(field = val, ...)`
   - Record update: `instance(field = newVal)`

6. **Type definitions**
   - `type Name` block — closed, indented
   - Records: lowercase fields with `:`
   - Sums: capitalized variants, optionally with their own field block
   - Generics: lowercase type vars, no introducing keyword
   - Newtype: `type Name = T`

7. **Pattern matching**
   - `expr match` followed by indented `pattern -> body` arms
   - Patterns: literal, identifier (binds), constructor with args, list (`[a, b, c]`), `_` (wildcard)
   - Exhaustiveness: required by compiler

8. **Effects**
   - `!` at call site marks effectful call
   - `! Eff` in type signature shows effect row
   - Effects are inferred; rarely written

9. **`?` early-exit**
   - `expr?` unwraps `Ok` or returns `Error` from enclosing function
   - Type rule: enclosing function must return `Result _ e` with same `e`

10. **Modules and imports**
    - `module Name` declares; `expose` lists public names
    - `use Path` imports whole module; `use Path (a, b)` cherry-picks; `use Path as Alias`

11. **Operator desugaring**
    - `+`, `-`, `*`, `/`, `^`, `==`, `<`, etc. — all dispatch via traits
    - Short table: operator → trait method

**Style:** terse, formal-ish. No friendly tone. Examples must be ≤ 3 lines each.

- [ ] **Step 1: Write `docs/syntax.md`**

Cover every section above. Every form in the spec must have an entry. If you
find a form in the spec that doesn't fit any section, add a section.

For each operator/form, the entry must answer: "where does it appear, what
does it do, what does it look like." That's three sentences max.

- [ ] **Step 2: Cross-check against the spec**

Open `docs/superpowers/specs/2026-04-27-i-language-design.md` side by side.
Skim each section. For every concrete syntactic form mentioned in the spec,
verify there is an entry in `docs/syntax.md`. Add missing entries.

- [ ] **Step 3: Commit**

```bash
git add docs/syntax.md
git commit -m "docs: write the syntax reference

Every operator and every form documented terse-style.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Write the type system manual

**Files:**
- Create: `docs/types.md`

A deep dive on the type system. ~2000 words. This is where the rules
live — exhaustiveness, totality, inference, traits, generics.

**Required sections:**

1. **What "strongly typed" means here** — every value has a static type known at compile time; no implicit conversions; no null
2. **Type inference scope** — local + return inferred; parameters annotated when ambiguous
3. **Records** — fields with `:`, methods with `=`, implicit `self`, construction = update
4. **Sum types** — variants, payloads, exhaustiveness checking
5. **Generics** — lowercase = type variable; first use binds; multiple type params
6. **Newtypes** — `type UserId = Int` is *distinct* from `Int`
7. **Traits** — declaration, implementation, operator desugaring, coherence (Haskell-style global, one impl per (trait, type) pair)
8. **Totality** — every function terminates; structural recursion accepted; general recursion requires `corecursive` annotation (provisional, see Limitations)
9. **No null, no exceptions** — `Maybe a` for absence; `Result a e` for failure; `?` sugar covered briefly with link to Errors section in tour
10. **Type signatures: when to write them** — interfaces yes; trivially-inferable bodies no

**Required examples:**
- A trait declaration (`Eq`, `Ord`, or similar)
- An impl (e.g., `impl Eq Point`)
- A generic function (`identity = x -> x` with `: a -> a`)
- A newtype demonstrating distinctness

- [ ] **Step 1: Write `docs/types.md`**

Reference the spec for any ambiguity. Where the spec says "TBD" (e.g.,
`corecursive`), say so explicitly: "v1 does not yet specify general
recursion; structural recursion is accepted."

- [ ] **Step 2: Commit**

```bash
git add docs/types.md
git commit -m "docs: write the type system manual"
```

---

## Task 7: Write the effect system manual

**Files:**
- Create: `docs/effects.md`

A deep dive on `!`. ~1000-1500 words. Most readers will spend most of their
time NOT thinking about effects; the doc should make this clear.

**Required sections:**

1. **What pure means here** — a function with no `!` in its inferred type does no IO, mutation, or other observable effect
2. **The `!` marker** — at call sites only; types pick it up automatically
3. **Effect rows** — `String ! IO -> Unit` reads "String to Unit, with IO"
4. **What effects exist in v1** — `IO` (print, readLine, files), `State` (Ref operations); that's it
5. **Why mutation isn't built-in** — every "modification" produces a new value; `Ref` exists for genuine mutable state
6. **What you don't have to write** — effect annotations in your own functions; the inferencer handles them
7. **What this catches** — accidentally calling `print!` from a function you thought was pure; the function's type now has `! IO` and you'll see it where it doesn't belong

**Required example:**

```
module Main
    expose main

# pure — Int -> Int
double = n -> n * 2

# pure — Int -> Int
quadruple = n -> double (double n)

# effectful — Int ! IO -> Unit
shout = n ->
    print! "the number is " ++ show n

# effectful — inferred ! IO
main =
    print! "starting"
    shout 21
    print! "done"
```

This becomes `examples/05-effects.i`.

- [ ] **Step 1: Write `examples/05-effects.i`** with the program above.

- [ ] **Step 2: Write `docs/effects.md`** following the structure above.

- [ ] **Step 3: Commit**

```bash
git add docs/effects.md examples/05-effects.i
git commit -m "docs: write the effect system manual"
```

---

## Task 8: Write the pattern matching reference

**Files:**
- Create: `docs/patterns.md`

A focused doc on `match`. ~800-1200 words.

**Required sections:**

1. **The `match` form** — `expr match` followed by indented arms
2. **Pattern kinds**
   - Literal: `0`, `"hi"`, `True`
   - Identifier: binds the matched value to a name
   - Wildcard: `_`
   - Constructor: `Some v`, `Cons head, tail`
   - List: `[]`, `[a]`, `[a, b]`, `[head, ...tail]` (TBD on rest pattern syntax)
   - Record destructuring: `Point(x = a, y = b)`
3. **Exhaustiveness** — compiler error if a constructor is missed; show an example
4. **Nested patterns** — patterns can contain patterns (`Some (Cons head, _)`)
5. **Guards** — *(open question; mark TBD)*

**Required example:**

```
module Main
    expose main

type Tree a
    Leaf
    Node
        value : a
        left : Tree a
        right : Tree a

count = tree ->
    tree match
        Leaf            -> 0
        Node v, l, r    -> 1 + count l + count r

main =
    t = Node(value = 1, left = Leaf, right = Node(value = 2, left = Leaf, right = Leaf))
    print! "count: " ++ show (count t)
```

This becomes `examples/06-tree.i`.

- [ ] **Step 1: Write `examples/06-tree.i`** with the program above.

- [ ] **Step 2: Write `docs/patterns.md`**.

For TBD items (rest patterns, guards), state plainly: "v1 has not yet
specified rest patterns and guards. They will be added in a later spec
revision."

- [ ] **Step 3: Commit**

```bash
git add docs/patterns.md examples/06-tree.i
git commit -m "docs: write pattern matching reference"
```

---

## Task 9: Write the standard library reference

**Files:**
- Create: `docs/stdlib.md`
- Create: `examples/07-result.i`

Pure reference: every type and function in v1's stdlib, grouped by module.
~2000 words. No prose tutorial; this is the "go look up `List.fold`" doc.

**Required modules and contents (matching spec § "v1 standard library"):**

Each entry has the form:
```
### name : signature

One-line description.

[example, if non-obvious]
```

1. **`Std.Bool`** — `True`, `False`, `and`, `or`, `not`
2. **`Std.Int`** — arithmetic (via `Add`/`Sub`/`Mul`/`Div` traits), `compare`, conversion to `Float` and `String`
3. **`Std.Float`** — same shape as Int; plus `sqrt`, `pow`, `sin`, `cos`, etc.
4. **`Std.Char`** — `toUpper`, `toLower`, `isDigit`, `isAlpha`
5. **`Std.String`** — `length`, `++` (concat), `split`, `toChars`, `fromChars`, `contains`, `trim`
6. **`Std.List`** — `map`, `filter`, `fold`, `length`, `reverse`, `head : List a -> Maybe a`, `tail : List a -> Maybe (List a)`, `take`, `drop`, `zip`
7. **`Std.Maybe`** — `None`, `Some`, `withDefault : Maybe a, a -> a`, `map`, `andThen`
8. **`Std.Result`** — `Ok`, `Error`, `withDefault`, `map`, `mapError`, `andThen`. Note: `?` is language sugar, not a stdlib function.
9. **`Std.IO`** — `print`, `println`, `readLine`, `readFile`, `writeFile`. All return `! IO`.
10. **`Std.Ref`** — `Ref a`, `make : a ! State -> Ref a`, `get : Ref a ! State -> a`, `set : Ref a, a ! State -> Unit`

**Required example:**

`examples/07-result.i`:
```
module Main
    expose main

use Std.Parse (parseInt)
use Std.IO (print)

type ParseError
    NotANumber
    OutOfRange

bounded = s, lo, hi ->
    n = parseInt s?
    n < lo or n > hi match
        True   -> Error OutOfRange
        False  -> Ok n

main =
    bounded "42", 0, 100 match
        Ok n      -> print! "got " ++ show n
        Error _   -> print! "bad input"
```

(`Std.Parse` is a pseudo-module here; in v1 stdlib it's actually under
`Std.Int` as `Std.Int.parse`. The example should match the actual stdlib
shape — adjust during writing.)

- [ ] **Step 1: Write `examples/07-result.i`** matching the actual stdlib structure you settle on.

- [ ] **Step 2: Write `docs/stdlib.md`**.

- [ ] **Step 3: Commit**

```bash
git add docs/stdlib.md examples/07-result.i
git commit -m "docs: write standard library reference"
```

---

## Task 10: Write the modules and imports doc

**Files:**
- Create: `docs/modules.md`
- Create: `examples/08-modules-app.i`
- Create: `examples/08-modules-lib.i`

Focused doc on the module system. ~600-1000 words.

**Required sections:**

1. **One file is one module** — file path determines module name
2. **`module` and `expose`** — first line of file declares; everything else is private
3. **`use`** — full module, cherry-pick, or alias
4. **Project layout** — typical directory structure for a multi-file program
5. **Visibility rules** — only names listed in `expose` are accessible from other modules
6. **Circular imports** — *not allowed in v1*; modules form a DAG

**Required example: a two-file program.**

`examples/08-modules-lib.i`:
```
module Geometry
    expose Point, distance

type Point
    x : Float
    y : Float

distance = a, b ->
    ((a.x - b.x)^2 + (a.y - b.y)^2)^0.5
```

`examples/08-modules-app.i`:
```
module Main
    expose main

use Geometry (Point, distance)
use Std.IO (print)

main =
    p1 = Point(x = 0, y = 0)
    p2 = Point(x = 3, y = 4)
    print! "distance: " ++ show (distance p1, p2)
```

- [ ] **Step 1: Write `examples/08-modules-lib.i`** and `examples/08-modules-app.i`.

- [ ] **Step 2: Write `docs/modules.md`**.

- [ ] **Step 3: Commit**

```bash
git add docs/modules.md examples/08-modules-lib.i examples/08-modules-app.i
git commit -m "docs: write modules and imports reference"
```

---

## Task 11: Write the build/run/CLI doc

**Files:**
- Create: `docs/building.md`

Forward-looking doc: describes the CLI commands the implementation will
provide. Anyone reading this should be able to imagine using `i` even though
it doesn't run yet.

**Required sections:**

1. **Installation** *(forward-looking)* — `cargo install i-lang` (planned)
2. **Project layout** — single-file vs multi-file projects; conventional directory layout
3. **Commands**
   - `i run path/to/main.i` — type-check then run
   - `i check path/to/main.i` — type-check only, no execution
   - `i fmt path/to/main.i` *(planned for later, mark as planned)*
4. **Errors** — what type errors look like (give one example)
5. **Examples** — running the examples from the `examples/` directory

State at the top: **"This doc describes commands that do not yet exist. The
Rust implementation arrives in the next plan."**

- [ ] **Step 1: Write `docs/building.md`**

Sketch a typical type-error message format. Something like:
```
error[type-mismatch]: expected `Int`, found `String`
   at examples/04-list-map.i:5
       doubled = nums.map x -> x * 2
                              ^^^^^^
note: `nums` is a `List String`, but `*` requires `List Int`
```

- [ ] **Step 2: Commit**

```bash
git add docs/building.md
git commit -m "docs: write build/run/CLI reference (forward-looking)"
```

---

## Task 12: Write the limitations doc

**Files:**
- Create: `docs/limitations.md`

Explicit list of what `i` v1 does NOT do, with rationale and what it would
take to add. ~600 words.

**Required entries:**

1. **No tuples** — records cover the same use cases. Add later if needed.
2. **No row polymorphism** — type system simplicity over expressiveness for v1.
3. **No macros / metaprogramming** — conflicts with "fits in your head."
4. **No dependent / refinement / linear types** — same.
5. **No native code generation in v1** — interpreter only; bytecode VM is v2 target, native is v3.
6. **No concurrency in v1** — single-threaded interpreter; actor model planned for v2.
7. **No stdlib outside the v1 list** — no networking, no async, no JSON, no regex. All come later.
8. **No FFI** — cannot call Rust/C from `i` in v1.
9. **No package manager** — single repo, no library installation. Add when language is stable.
10. **`?` only works inside `Result`-returning functions** — not general; that's intentional.
11. **No `return`, `break`, `continue`** — the function's value is the value of its body expression; loops are folds over lists.
12. **Lazy evaluation is not supported** — strict only. `Lazy a` library type may come later.

For each entry: one sentence on the limit, one sentence on rationale, one
sentence on what would change to lift it.

- [ ] **Step 1: Write `docs/limitations.md`** with the entries above.

- [ ] **Step 2: Commit**

```bash
git add docs/limitations.md
git commit -m "docs: enumerate v1 limitations"
```

---

## Task 13: Cross-link and review pass

**Files:**
- Modify: every doc, adding cross-links where appropriate

After all individual docs are written, do a connectivity pass.

- [ ] **Step 1: Re-read each doc with cross-linking in mind**

For each doc, find every place where another doc would deepen understanding,
and add a markdown link. Examples:
- In `tour.md` § Pattern matching: link to `patterns.md`
- In `types.md` § Generics: link to `stdlib.md` § `Std.List`
- In `building.md` § Commands: link to `tour.md` for first-program walkthrough

Don't over-link. If a reader would naturally want to jump to another doc, link
it. If not, don't.

- [ ] **Step 2: Verify every example file is referenced**

Run: `ls examples/`
Run: `grep -r "examples/" docs/ | wc -l`

Every file in `examples/` must be mentioned by at least one doc. If an
example file isn't referenced, either reference it or delete it.

- [ ] **Step 3: Verify every spec section has a doc home**

Open the spec at `docs/superpowers/specs/2026-04-27-i-language-design.md`.
Skim each `##` heading. For each heading, identify which user doc covers
that material. If a heading has no user-doc home, add coverage.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "docs: cross-link review pass

Add inter-doc links and verify spec → doc coverage. Every concept in the
spec is now reachable from the user docs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 14: Write the doc-set summary

**Files:**
- Modify: `README.md` (add a "What's done" section)
- Create: `docs/superpowers/plans/PROGRESS.md` (running progress log)

Stamp the milestone. The docs are done; the implementation isn't.

- [ ] **Step 1: Append to `README.md`**

Update the existing "Status" section to reflect that docs are complete:

```markdown
## Status

**Documentation complete.** The end-state user docs are written; every
example program in `examples/` is a complete, syntactically valid `i`
program — though none of them are runnable yet because the implementation
doesn't exist.

**Implementation: not started.** Plan 2 (lexer + parser) is next.
```

- [ ] **Step 2: Create `docs/superpowers/plans/PROGRESS.md`**

```markdown
# Progress

## Phase 0: Setup
- [x] Cargo skeleton, README, LICENSE — Plan 1, Task 1-2

## Phase 1: Documentation
- [x] Tour
- [x] Syntax reference
- [x] Type system manual
- [x] Effect system manual
- [x] Pattern matching reference
- [x] Standard library reference
- [x] Modules
- [x] Build/run/CLI (forward-looking)
- [x] Limitations
- [x] Cross-link pass

## Phase 2: Implementation (not started)
- [ ] Lexer + parser — Plan 2 (TBD)
- [ ] Name resolution — Plan 3 (TBD)
- [ ] Type checker — Plan 4 (TBD)
- [ ] Interpreter — Plan 5 (TBD)
- [ ] Stdlib — Plan 6 (TBD)
- [ ] Driver / CLI — Plan 7 (TBD)
- [ ] Golden test harness — Plan 8 (TBD)
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/PROGRESS.md
git commit -m "Mark documentation phase complete

End-state docs are done. Implementation phases planned but not started.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## What comes next (out of scope for this plan)

The remaining plans, in order:

- **Plan 2 — Lexer + parser.** Tokenizer with indentation/layout. Recursive-descent parser to AST. Pretty-printer for round-trip testing. Acceptance: every `examples/*.i` parses without error; round-trip identity holds.
- **Plan 3 — Name resolution.** Module loader, `expose`/`use`, scope, implicit `Type.` inside type blocks, implicit `self` inside methods. Acceptance: every `examples/*.i` resolves all names.
- **Plan 4 — Type checker.** Hindley-Milner inference with effect rows, trait constraints, exhaustiveness checking, totality checking. Largest plan. Acceptance: every `examples/*.i` type-checks; intentionally-broken programs in `tests/negative/` fail with the expected error.
- **Plan 5 — Tree-walking interpreter.** Evaluate the typed AST. Acceptance: every `examples/*.i` runs and produces the expected output (golden files).
- **Plan 6 — Standard library.** Implement everything documented in `docs/stdlib.md`. Acceptance: stdlib unit tests pass.
- **Plan 7 — Driver / CLI.** `i run`, `i check`. Real error messages with source spans. Acceptance: docs in `docs/building.md` work as described.
- **Plan 8 — Golden test harness.** A test runner that takes every `examples/*.i`, runs it under `i`, compares output to `examples/*.out`. Acceptance: CI runs all goldens.

Each will be planned only when its predecessor is complete and the docs may
need adjustment based on what the implementation reveals.
