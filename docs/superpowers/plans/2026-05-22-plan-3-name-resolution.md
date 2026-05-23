# Plan 3 — Name Resolution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Walk a parsed `File` (or set of files) and, for every name *reference* in expressions, patterns, and types, decide what it refers to: a local binding, a top-level definition in this module, a constructor, an imported value, or a module. Reject the things that ought to be rejected: unknown names, duplicate top-level definitions, duplicate names in the same scope, cyclic module imports.

**Architecture:** Two-pass per module: (1) collect every top-level definition into a module scope, (2) walk each expression with a scope stack pushing locals (lambda params, pattern variables, block let-bindings, synthetic `self`). Cross-module resolution consumes a `ModuleSet` (a `HashMap<ModulePath, File>`) the caller assembles — Plan 3 owns the resolver; the project loader lives in Plan 7. Output is a `Resolution` side-table keyed by `Span` — the AST is unchanged, so the parser and pretty-printer stay decoupled.

**Tech Stack:** Rust 1.95, edition 2024. No new crates — `std::collections::HashMap` is all the data structures we need. Errors reuse `crate::error::{Error, ErrorKind}` with new variants.

---

## Decisions baked in

These were open questions before the plan; they're resolved here so they don't reopen mid-task.

1. **Output is a side-table, not a new `Resolved*` AST.** Each name reference site has a unique `Span` because the parser produces each node from a distinct token range. The resolver fills a `HashMap<Span, ResolvedName>`. The AST is not mutated and not duplicated. Downstream passes (type checker, interpreter) look references up by the AST node's span. Trade-off: span-keyed lookups are O(1) but cost a `HashMap` per file. We accept the cost; the alternative (parallel `ResolvedExpr`/`ResolvedPattern`/`ResolvedType`/... types) duplicates ~15 enums for no semantic benefit at this stage. If span collisions ever appear, we'll add a `NodeId` numbering pass.

2. **Shadowing across nested scopes is allowed; duplicate names in the *same* scope are rejected.** `x = 1` and then `x -> ...` (lambda with param `x`) is fine — inner shadows outer. But two `x =` lines in the same block, or `x x ->` (lambda with two `x` params), are errors. This matches OCaml/Haskell and avoids forbidding the obvious cases where a lambda param naturally has the same name as something already in scope. The spec's "no rebinding" rule is about mutation, not nested-scope shadowing.

3. **Locals (lambda params, pattern vars, block lets, synthetic `self`) get `LocalId`s; top-level defs get `DefId`s.** A `ResolvedName` is one of `Local(LocalId)`, `TopLevel(DefId)`, `Ctor(DefId)`, `Imported { module: ModulePath, name: String }`, `Module(ModulePath)`. The two id types are disjoint integer namespaces; both are u32 newtypes. Storing `Imported` directly (rather than synthesising a `DefId` for each import) keeps cross-module bookkeeping simple — the type checker resolves the target later.

4. **Top-level definitions live in two namespaces: *values* and *types*.** A constructor goes in the value namespace (it's used like a function). A type goes in the type namespace. A trait goes in the type namespace too. They can share names without colliding: `type Result a` and `Result = ...` are independent. The resolver routes lookups by syntactic position: `Var` and `Ctor` hit the value namespace; `TypeKind::Named` and `TypeKind::Var` hit the type namespace.

5. **Constructor scope is the constructor's name alone — no `Type.Ctor` form in v1.** When you `expose Shape(..)`, both `Shape` (type) and `Circle`, `Rect` (ctors) are exposed flat. `Circle` resolves on its own, not as `Shape.Circle`. The parser doesn't accept `Shape.Circle` as a constructor reference anyway.

6. **`self` is a synthetic local, injected when entering a `Decl` whose parent is a `TypeMember::Method` with a function body.** Methods are recognised structurally during the top-level walk: any `=` binding inside a `TypeBody::Block` whose value is a `Lambda` (or annotated as one) gets `self` pushed before the body is walked. Field-only bindings (`x : Float`) and constants (`pi = 3.14`) inside a `type` block do not receive `self`.

7. **Module-qualified access is matched longest-prefix-first against the `use` set.** A chain like `Std.IO.print` parses as `FieldAccess(FieldAccess(Var("Std"), "IO"), "print")`. The resolver collects the chain bottom-up, then tries module-path matches: is `Std.IO.print` a module? No. Is `Std.IO` a module that's in scope and exports `print`? If yes, that's the resolution. Is `Std` a module that exports `IO`? Falls through to "value access on a value" (which the type-checker handles later). This rule means modules with dotted names work cleanly without introducing new AST shapes.

8. **No standard library auto-import in this plan.** Resolution treats unknown names as errors. Examples in `examples/` that call `print!` or `show` will fail resolution unless the corresponding test wires up a stub module providing those names. The resolver corpus tests will use minimal hand-written `.i` fixtures rather than the full `examples/` set, with one optional integration test demonstrating multi-file resolution. Plan 6 (stdlib) and Plan 7 (driver) close this gap together.

9. **Effect rows are name-resolved but not validated.** An effect row `! IO` mentions a name `IO`. The resolver looks it up in the type namespace and records the resolution; if the name isn't known, that's an error. Effect-system semantics (composition, subtyping) are Plan 4's job.

10. **Cycle detection runs on the module graph, not the value-dependency graph.** If module `A` uses `B` and `B` uses `A`, that's a cycle and rejected. Within a single module, mutual recursion between top-level bindings is fine (top-level scope is collected upfront, so all top-level names see all others). The plan owes the value-dependency analysis to Plan 4 (let-polymorphism needs it); Plan 3 only does module-level cycle detection.

---

## File structure

```
src/
  resolve/
    mod.rs            # public resolve_file(&File) -> Result<Resolution, Vec<Error>>
                      # and resolve_project(&ModuleSet) -> Result<ProjectResolution, Vec<Error>>
    types.rs          # DefId, LocalId, ResolvedName, DefInfo, Resolution, ModulePath
    scope.rs          # ScopeStack: push_local / lookup_local / collect_top_level
    module_set.rs     # ModuleSet wrapper + cycle detection
    walker.rs         # walk_expr / walk_pattern / walk_type — recursive resolution
  lib.rs              # + pub mod resolve;

tests/
  resolver_top_level.rs  # hand-written: top-level collection, duplicate errors
  resolver_locals.rs     # hand-written: lambdas, patterns, block lets, self
  resolver_modules.rs    # hand-written: use (whole, cherry, alias), qualified access
  resolver_errors.rs     # hand-written: unknown name, duplicate, cycle, unknown module
  resolver_corpus.rs     # insta::glob! over tests/corpus/resolver/*.i
  corpus/
    resolver/
      single-binding.i
      lambda-shadow.i
      match-binds.i
      method-self.i
      ctor-resolve.i
      qualified-call.i
      cherry-pick.i
      alias-import.i
      sum-with-methods.i
```

**Why this split:**

- `resolve/types.rs` is the data model. Everything else references it.
- `resolve/scope.rs` is the scope-stack abstraction. Push/pop/lookup, plus the top-level collection pass. It only knows about locals and one module's top-level — nothing about cross-module.
- `resolve/module_set.rs` is the cross-module layer: holds the resolved imports per module and runs cycle detection. Separating this from `scope.rs` keeps the single-file path easy to test in isolation.
- `resolve/walker.rs` is the recursion: takes a scope stack and an AST node, walks every sub-node, fills the side-table. Splitting walkers per syntactic category (`walk_expr`, `walk_pattern`, `walk_type`) keeps each one small and lets the test for one form not drag in the others.

---

## Testing strategy

The three-layer strategy from `docs/testing.md` (Layer 1 corpus, Layer 2 hand-written, Layer 3 round-trip) is the contract. Plan 3 adds Layers 1 and 2 only — Layer 3 doesn't apply because there's no round-trip property for resolution (it's not invertible).

- **Layer 1 — Insta corpus snapshots.** `tests/corpus/resolver/*.i` files exercise one feature each. Snapshot format is a custom `Display` on `Resolution` that prints each resolved reference one-per-line: `<span> <token-text> -> <resolved-name>`. The format is human-reviewable in `cargo insta review`.

- **Layer 2 — Hand-written assertions.** Per-form tests (`resolver_top_level.rs`, `resolver_locals.rs`, `resolver_modules.rs`) use `assert!` on lookups against the `Resolution` side-table. Error tests (`resolver_errors.rs`) use `matches!` on the returned `Vec<Error>`. These pin down the *exact* behaviour at a finer grain than a snapshot diff would.

Errors are returned as `Vec<Error>` because a single file might have multiple unrelated unresolved names and reporting only the first is unhelpful. Snapshots show only successful resolutions; error tests are hand-written.

---

## Decisions deferred to later plans

- **Trait method resolution** (`Add.add` dispatch from `a + b`) — Plan 4 (type checker), because picking which `impl` to call needs type info.
- **Field-vs-method disambiguation** on `instance.x` — Plan 4, same reason.
- **Stdlib auto-import** — Plan 6 (stdlib) provides the modules; Plan 7 (driver) wires them in.
- **Project file loader** — Plan 7 walks `src/`, parses each `.i`, and assembles the `ModuleSet`. Plan 3's tests build the set inline.
- **Value-dependency analysis** (which top-level bindings depend on which others) — Plan 4 needs this for let-group polymorphism; Plan 3 doesn't.

---

## Task 1: Resolver scaffold and data model

**Files:**
- Create: `src/resolve/mod.rs`
- Create: `src/resolve/types.rs`
- Modify: `src/lib.rs` (add `pub mod resolve;`)
- Modify: `src/error.rs` (add `Unresolved`, `Duplicate*` variants — placeholders, real use comes in later tasks)
- Test: `tests/resolver_top_level.rs`

- [ ] **Step 1: Write the failing test**

`tests/resolver_top_level.rs`:

```rust
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn empty_file_resolves() {
    let src = "module M\n    expose x\n\nx = 1\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    assert_eq!(res.defs.len(), 1);
    assert_eq!(res.defs[0].name, "x");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_top_level -- empty_file_resolves`
Expected: FAIL — `resolve` module / `resolve_file` function / `Resolution::defs` field do not exist.

- [ ] **Step 3: Write minimal implementation**

`src/lib.rs` — add `pub mod resolve;`.

`src/resolve/types.rs`:

```rust
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

pub type ModulePath = Vec<String>;

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedName {
    Local(LocalId),
    TopLevel(DefId),
    Ctor(DefId),
    Imported { module: ModulePath, name: String },
    Module(ModulePath),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefKind {
    Value,
    Type,
    Ctor { of_type: DefId },
    Trait,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefInfo {
    pub id: DefId,
    pub name: String,
    pub kind: DefKind,
    pub span: Span,
}

#[derive(Debug, Default, Clone)]
pub struct Resolution {
    pub defs: Vec<DefInfo>,
    pub refs: HashMap<Span, ResolvedName>,
}
```

`src/resolve/mod.rs`:

```rust
mod types;

pub use types::*;

use crate::ast::File;
use crate::error::Error;

pub fn resolve_file(_file: &File) -> Result<Resolution, Vec<Error>> {
    Ok(Resolution::default())
}
```

Add nothing to `src/error.rs` yet — the test above only exercises the happy path. The first failing test forces the scaffold; later tasks add error variants when they're first needed.

The test currently expects `defs.len() == 1` but the scaffold returns empty. Make the scaffold return a single hard-coded `DefInfo` so the test passes — Task 2 replaces this with the real collection pass.

```rust
pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    if let Some(decl) = file.decls.first() {
        if let crate::ast::DeclKind::Binding { name, .. } = &decl.node {
            res.defs.push(DefInfo {
                id: DefId(0),
                name: name.clone(),
                kind: DefKind::Value,
                span: decl.span,
            });
        }
    }
    Ok(res)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_top_level -- empty_file_resolves`
Expected: PASS.

Also run `make ci` — fmt + clippy + tests — to confirm the new module compiles cleanly under the project bar.

- [ ] **Step 5: Commit**

```bash
git add src/resolve src/lib.rs tests/resolver_top_level.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 1: resolver scaffold

Introduce src/resolve/ with the data model (DefId, LocalId,
ResolvedName, DefInfo, Resolution) and a stub resolve_file that
satisfies a single trivial test. The scaffold deliberately fakes the
top-level collection so the test passes; Task 2 replaces the fake
with the real pass.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Top-level definition collection

Collect every top-level decl into the module's value or type namespace. No expression walking yet.

**Files:**
- Create: `src/resolve/scope.rs`
- Modify: `src/resolve/mod.rs` (use scope.rs; expose `collect_top_level`)
- Modify: `tests/resolver_top_level.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_top_level.rs`:

```rust
#[test]
fn collects_multiple_top_level() {
    let src = "module M\n    expose x, y\n\nx = 1\ny = 2\n\ntype Pair\n    a : Int\n    b : Int\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let names: Vec<&str> = res.defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"Pair"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_top_level -- collects_multiple_top_level`
Expected: FAIL — the stub only collects the first decl.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/scope.rs`:

```rust
use crate::ast::{Decl, DeclKind, File};
use crate::error::Error;
use super::types::{DefId, DefInfo, DefKind, Resolution};

pub(super) fn collect_top_level(file: &File, res: &mut Resolution) -> Vec<Error> {
    let errors = Vec::new();
    for decl in &file.decls {
        collect_decl(decl, res);
    }
    errors
}

fn collect_decl(decl: &Decl, res: &mut Resolution) {
    match &decl.node {
        DeclKind::Binding { name, .. } => push_def(res, name.clone(), DefKind::Value, decl.span),
        DeclKind::TypeDecl { name, .. } => push_def(res, name.clone(), DefKind::Type, decl.span),
        DeclKind::TraitDecl { name, .. } => push_def(res, name.clone(), DefKind::Trait, decl.span),
        DeclKind::ImplDecl { .. } => { /* impls add no top-level name */ }
        DeclKind::Use { .. } => { /* use is handled in module-set tasks */ }
    }
}

fn push_def(res: &mut Resolution, name: String, kind: DefKind, span: crate::span::Span) {
    let id = DefId(res.defs.len() as u32);
    res.defs.push(DefInfo { id, name, kind, span });
}
```

`src/resolve/mod.rs`:

```rust
mod scope;
mod types;

pub use types::*;

use crate::ast::File;
use crate::error::Error;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let errors = scope::collect_top_level(file, &mut res);
    if errors.is_empty() {
        Ok(res)
    } else {
        Err(errors)
    }
}
```

The `empty_file_resolves` test from Task 1 still passes — it had a single `x = 1` and `collect_top_level` produces one `DefInfo` for it.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_top_level`
Expected: both tests PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_top_level.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 2: collect top-level definitions

Replace the Task 1 stub with a real top-level collection pass.
Bindings, type decls, and trait decls each become DefInfo entries;
impls and use-decls are deferred to later tasks. Constructors inside
type bodies are not yet collected — Task 5 adds that.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Duplicate top-level definition errors

**Files:**
- Modify: `src/error.rs` (add `DuplicateDefinition` variant)
- Modify: `src/resolve/scope.rs` (detect duplicates)
- Modify: `tests/resolver_top_level.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_top_level.rs`:

```rust
use i_lang::error::ErrorKind;

#[test]
fn duplicate_value_binding() {
    let src = "module M\n    expose x\n\nx = 1\nx = 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::DuplicateDefinition { name, .. } if name == "x"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_top_level -- duplicate_value_binding`
Expected: FAIL — `ErrorKind::DuplicateDefinition` doesn't exist yet; resolver returns Ok.

- [ ] **Step 3: Write minimal implementation**

`src/error.rs` — add the variant:

```rust
DuplicateDefinition {
    name: String,
    first_span: Span,
},
```

`src/resolve/scope.rs` — track a `HashSet` of names per namespace and emit on conflict. The value and type namespaces are independent — duplicates only within a namespace, not across.

```rust
use std::collections::HashMap;
use crate::ast::{Decl, DeclKind, File};
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use super::types::{DefId, DefInfo, DefKind, Resolution};

pub(super) fn collect_top_level(file: &File, res: &mut Resolution) -> Vec<Error> {
    let mut errors = Vec::new();
    let mut value_seen: HashMap<String, Span> = HashMap::new();
    let mut type_seen: HashMap<String, Span> = HashMap::new();
    for decl in &file.decls {
        collect_decl(decl, res, &mut value_seen, &mut type_seen, &mut errors);
    }
    errors
}

fn collect_decl(
    decl: &Decl,
    res: &mut Resolution,
    value_seen: &mut HashMap<String, Span>,
    type_seen: &mut HashMap<String, Span>,
    errors: &mut Vec<Error>,
) {
    match &decl.node {
        DeclKind::Binding { name, .. } => {
            check_dup(name, decl.span, value_seen, errors);
            push_def(res, name.clone(), DefKind::Value, decl.span);
        }
        DeclKind::TypeDecl { name, .. } => {
            check_dup(name, decl.span, type_seen, errors);
            push_def(res, name.clone(), DefKind::Type, decl.span);
        }
        DeclKind::TraitDecl { name, .. } => {
            check_dup(name, decl.span, type_seen, errors);
            push_def(res, name.clone(), DefKind::Trait, decl.span);
        }
        DeclKind::ImplDecl { .. } | DeclKind::Use { .. } => {}
    }
}

fn check_dup(
    name: &str,
    span: Span,
    seen: &mut HashMap<String, Span>,
    errors: &mut Vec<Error>,
) {
    if let Some(first) = seen.get(name) {
        errors.push(Error {
            span,
            kind: ErrorKind::DuplicateDefinition {
                name: name.to_string(),
                first_span: *first,
            },
        });
    } else {
        seen.insert(name.to_string(), span);
    }
}

fn push_def(res: &mut Resolution, name: String, kind: DefKind, span: Span) {
    let id = DefId(res.defs.len() as u32);
    res.defs.push(DefInfo { id, name, kind, span });
}
```

A binding with a type annotation alone (`greeting : String`) followed by a binding with a value (`greeting = "hi"`) should NOT be a duplicate — the syntax explicitly allows that split form. For now, both produce `DeclKind::Binding` entries with different field combinations (one with `ty.is_some() && value.is_none()`, the other with both). Refine `check_dup` to merge them: a `Binding` with `value.is_none()` only registers the name if it hasn't already been declared with a value, and vice versa. Simplest path: treat "annotation-only" bindings as non-defining and skip the dup check for them.

```rust
DeclKind::Binding { name, value, .. } => {
    if value.is_some() {
        check_dup(name, decl.span, value_seen, errors);
        push_def(res, name.clone(), DefKind::Value, decl.span);
    }
}
```

(Annotation-only bindings will be cross-referenced to their value at type-checking time — Plan 4 — so the resolver can ignore them for now.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_top_level`
Expected: all three tests PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_top_level.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 3: detect duplicate top-level definitions

Add ErrorKind::DuplicateDefinition (carries the first occurrence's
span for diagnostics) and emit it during top-level collection. Value
and type namespaces are independent. Annotation-only bindings
(name : Type without =) don't count as definitions — they pair with
a later value binding, which is what's actually being defined.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Resolve `Var` references against the top-level scope

Walk expressions. For every `ExprKind::Var(name)`, look the name up in the value namespace and record a `ResolvedName::TopLevel(def_id)` in `Resolution::refs` keyed by the var's span. Unknown names produce `ErrorKind::Unresolved`.

**Files:**
- Modify: `src/error.rs` (add `Unresolved`)
- Create: `src/resolve/walker.rs`
- Modify: `src/resolve/mod.rs` (call walker after collect)
- Test: `tests/resolver_locals.rs` (NEW)

- [ ] **Step 1: Write the failing test**

`tests/resolver_locals.rs`:

```rust
use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::{resolve_file, ResolvedName};

#[test]
fn var_resolves_to_top_level() {
    let src = "module M\n    expose y\n\nx = 1\ny = x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // The Var "x" inside "y = x" should appear in refs.
    let x_resolutions: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::TopLevel(_)))
        .collect();
    assert_eq!(x_resolutions.len(), 1);
}

#[test]
fn unknown_var_is_error() {
    let src = "module M\n    expose y\n\ny = unknownName\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "unknownName"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals`
Expected: FAIL — `refs` is empty; `Unresolved` variant doesn't exist.

- [ ] **Step 3: Write minimal implementation**

`src/error.rs` — add:

```rust
Unresolved { name: String },
```

`src/resolve/walker.rs`:

```rust
use crate::ast::{Decl, DeclKind, Expr, ExprKind, File};
use crate::error::{Error, ErrorKind};
use super::types::{DefKind, Resolution, ResolvedName};

pub(super) fn walk_file(file: &File, res: &mut Resolution, errors: &mut Vec<Error>) {
    for decl in &file.decls {
        walk_decl(decl, res, errors);
    }
}

fn walk_decl(decl: &Decl, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
        walk_expr(v, res, errors);
    }
    // TypeDecl/TraitDecl/ImplDecl bodies handled in later tasks.
}

fn walk_expr(e: &Expr, res: &mut Resolution, errors: &mut Vec<Error>) {
    match &e.node {
        ExprKind::Var(name) => resolve_var(name, e.span, res, errors),
        ExprKind::IntLit(_) | ExprKind::FloatLit(_) | ExprKind::StringLit(_) | ExprKind::Ctor(_) => {}
        ExprKind::Paren(inner) => walk_expr(inner, res, errors),
        ExprKind::BinOp { lhs, rhs, .. } => { walk_expr(lhs, res, errors); walk_expr(rhs, res, errors); }
        ExprKind::UnaryOp { expr, .. } => walk_expr(expr, res, errors),
        ExprKind::List(items) => items.iter().for_each(|i| walk_expr(i, res, errors)),
        // Other variants: stub for now; later tasks fill them in.
        _ => {}
    }
}

fn resolve_var(name: &str, span: crate::span::Span, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let Some(def) = res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Value)) {
        res.refs.insert(span, ResolvedName::TopLevel(def.id));
    } else {
        errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
    }
}
```

`src/resolve/mod.rs`:

```rust
mod scope;
mod types;
mod walker;

pub use types::*;

use crate::ast::File;
use crate::error::Error;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let mut errors = scope::collect_top_level(file, &mut res);
    walker::walk_file(file, &mut res, &mut errors);
    if errors.is_empty() { Ok(res) } else { Err(errors) }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals`
Expected: both tests PASS.

Run `cargo test` overall — make sure resolver_top_level tests still pass.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 4: resolve Var against top-level scope

Add walker.rs with a stub walk_expr that handles the trivial cases
and resolves ExprKind::Var against the value namespace. Unknown
names emit ErrorKind::Unresolved. Other expression forms (calls,
lambdas, match) are left as no-ops; later tasks recurse into them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Collect and resolve constructors

Top-level type decls with variants or fields contribute *constructors* to the value namespace. `Circle` from `type Shape\n    Circle\n        radius : Float` becomes a value-namespace entry. Walking expressions, `ExprKind::Ctor(name)` resolves via the same lookup.

**Files:**
- Modify: `src/resolve/scope.rs` (descend into TypeBody to collect ctors)
- Modify: `src/resolve/walker.rs` (handle `ExprKind::Ctor`, `Construct`, `Update`)
- Modify: `tests/resolver_locals.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
#[test]
fn ctor_resolves() {
    let src = "module M\n    expose Shape\n\ntype Shape\n    Circle\n        radius : Float\n    Rect\n        width : Float\n        height : Float\n\nmkCircle = Circle(radius = 1.0)\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // "Circle" should appear in defs as a Ctor.
    use i_lang::resolve::DefKind;
    assert!(res.defs.iter().any(|d| d.name == "Circle" && matches!(d.kind, DefKind::Ctor { .. })));
    // The Construct in mkCircle should resolve.
    let ctor_refs: Vec<_> = res.refs.values().filter(|r| matches!(r, ResolvedName::Ctor(_))).collect();
    assert_eq!(ctor_refs.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- ctor_resolves`
Expected: FAIL — no ctor in defs; Construct isn't walked.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/scope.rs` — in `collect_decl`, descend into `TypeBody::Block` and collect each `TypeMember::Variant` as a Ctor. Record `of_type` as the parent type's `DefId`.

```rust
use crate::ast::{TypeBody, TypeMember, VariantBody};

// In collect_decl, replace the TypeDecl arm:
DeclKind::TypeDecl { name, body, .. } => {
    check_dup(name, decl.span, type_seen, errors);
    let type_id = DefId(res.defs.len() as u32);
    push_def(res, name.clone(), DefKind::Type, decl.span);
    if let TypeBody::Block(members) = body {
        for m in members {
            if let TypeMember::Variant { name: vname, .. } = m {
                // No span on TypeMember currently — use the parent decl's span.
                // (If TypeMember gains its own span later, switch to it.)
                check_dup(vname, decl.span, value_seen, errors);
                let id = DefId(res.defs.len() as u32);
                res.defs.push(DefInfo {
                    id,
                    name: vname.clone(),
                    kind: DefKind::Ctor { of_type: type_id },
                    span: decl.span,
                });
            }
        }
    }
}
```

If the parser does in fact lack a span on `TypeMember::Variant`, accept the parent decl's span as a temporary stand-in. Note this in the commit so a future task knows to add per-variant spans when diagnostics need them.

`src/resolve/walker.rs` — add handling for `Ctor`, `Construct`, `Update`:

```rust
ExprKind::Ctor(name) => {
    if let Some(def) = res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Ctor { .. })) {
        res.refs.insert(e.span, ResolvedName::Ctor(def.id));
    } else {
        errors.push(Error { span: e.span, kind: ErrorKind::Unresolved { name: name.clone() } });
    }
}
ExprKind::Construct { type_name, fields } => {
    if let Some(def) = res.defs.iter().find(|d| d.name == type_name) {
        let resolved = match def.kind {
            DefKind::Ctor { .. } => ResolvedName::Ctor(def.id),
            _ => ResolvedName::TopLevel(def.id),
        };
        res.refs.insert(e.span, resolved);
    } else {
        errors.push(Error { span: e.span, kind: ErrorKind::Unresolved { name: type_name.clone() } });
    }
    for kw in fields { walk_expr(&kw.value, res, errors); }
}
ExprKind::Update { value, fields } => {
    walk_expr(value, res, errors);
    for kw in fields { walk_expr(&kw.value, res, errors); }
}
```

`Construct` looks like `Circle(radius = 1.0)`: the type-name slot can be a type or a ctor. The lookup tries either. (A type with no variants is itself the constructor: `type Point\n    x : Float\n    y : Float` then `Point(x = 0, y = 0)` constructs the record. That's a `DefKind::Type` with field members.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals -- ctor_resolves`
Expected: PASS.

Run all tests + `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 5: resolve constructors

Descend into TypeBody::Block during top-level collection to register
each variant as DefKind::Ctor with a back-reference to its parent
type. Walk ExprKind::Ctor, Construct, and Update against the
combined value/ctor namespace. Construct may target either a Type
(record construction) or a Ctor (sum-variant construction); the
resolver picks based on the def's kind.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Lambda parameter scope

Lambdas introduce parameter bindings visible in the body. Implement the `ScopeStack` with `push`/`pop`/`lookup`. Parameter conflicts (two params with the same name in the same lambda) are errors.

**Files:**
- Modify: `src/resolve/scope.rs` (add `ScopeStack`)
- Modify: `src/resolve/walker.rs` (handle `Lambda`, lookup locals first)
- Modify: `src/error.rs` (add `DuplicateLocal`)
- Modify: `tests/resolver_locals.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
#[test]
fn lambda_param_shadows_top_level() {
    let src = "module M\n    expose f\n\nx = 1\nf = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // The Var "x" inside the lambda body should resolve to a Local, not the top-level x.
    let local_refs: Vec<_> = res.refs.values().filter(|r| matches!(r, ResolvedName::Local(_))).collect();
    assert_eq!(local_refs.len(), 1);
}

#[test]
fn duplicate_lambda_param_is_error() {
    let src = "module M\n    expose f\n\nf = x x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::DuplicateLocal { name } if name == "x"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- lambda_param`
Expected: FAIL — lambda body still resolves `x` to top-level; no duplicate detection.

- [ ] **Step 3: Write minimal implementation**

`src/error.rs`:

```rust
DuplicateLocal { name: String },
```

`src/resolve/scope.rs` — append:

```rust
use super::types::LocalId;

#[derive(Default)]
pub(super) struct ScopeStack {
    frames: Vec<Vec<(String, LocalId)>>,
    next_id: u32,
}

impl ScopeStack {
    pub(super) fn new() -> Self {
        Self { frames: vec![Vec::new()], next_id: 0 }
    }
    pub(super) fn push_frame(&mut self) { self.frames.push(Vec::new()); }
    pub(super) fn pop_frame(&mut self) { self.frames.pop(); }
    pub(super) fn push_local(&mut self, name: &str) -> Result<LocalId, ()> {
        let frame = self.frames.last_mut().unwrap();
        if frame.iter().any(|(n, _)| n == name) {
            return Err(());
        }
        let id = LocalId(self.next_id);
        self.next_id += 1;
        frame.push((name.to_string(), id));
        Ok(id)
    }
    pub(super) fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for frame in self.frames.iter().rev() {
            if let Some((_, id)) = frame.iter().find(|(n, _)| n == name) {
                return Some(*id);
            }
        }
        None
    }
}
```

`src/resolve/walker.rs` — thread the scope through. Switch the walker to a struct so the stack can be reused:

```rust
use super::scope::ScopeStack;
use crate::ast::{Pattern, PatternKind};

pub(super) struct Walker<'a> {
    pub res: &'a mut Resolution,
    pub errors: &'a mut Vec<Error>,
    pub scope: ScopeStack,
}

impl<'a> Walker<'a> {
    pub(super) fn walk_file(&mut self, file: &File) {
        for decl in &file.decls { self.walk_decl(decl); }
    }
    fn walk_decl(&mut self, decl: &Decl) {
        if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
            self.walk_expr(v);
        }
    }
    fn walk_expr(&mut self, e: &Expr) {
        match &e.node {
            ExprKind::Var(name) => self.resolve_var(name, e.span),
            ExprKind::Ctor(name) => self.resolve_ctor(name, e.span),
            ExprKind::Lambda { params, body } => {
                self.scope.push_frame();
                for p in params { self.bind_pattern(p); }
                self.walk_expr(body);
                self.scope.pop_frame();
            }
            ExprKind::BinOp { lhs, rhs, .. } => { self.walk_expr(lhs); self.walk_expr(rhs); }
            ExprKind::UnaryOp { expr, .. } => self.walk_expr(expr),
            ExprKind::Paren(inner) => self.walk_expr(inner),
            ExprKind::List(items) => for i in items { self.walk_expr(i); },
            ExprKind::Construct { type_name, fields } => {
                self.resolve_type_or_ctor(type_name, e.span);
                for kw in fields { self.walk_expr(&kw.value); }
            }
            ExprKind::Update { value, fields } => {
                self.walk_expr(value);
                for kw in fields { self.walk_expr(&kw.value); }
            }
            _ => {} // remaining variants in later tasks
        }
    }
    fn bind_pattern(&mut self, p: &Pattern) {
        // Task 7 fleshes this out; for now, bind only PatternKind::Var.
        if let PatternKind::Var(name) = &p.node {
            if self.scope.push_local(name).is_err() {
                self.errors.push(Error {
                    span: p.span,
                    kind: ErrorKind::DuplicateLocal { name: name.clone() },
                });
            }
        }
    }
    fn resolve_var(&mut self, name: &str, span: Span) {
        if let Some(id) = self.scope.lookup_local(name) {
            self.res.refs.insert(span, ResolvedName::Local(id));
            return;
        }
        if let Some(def) = self.res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Value)) {
            self.res.refs.insert(span, ResolvedName::TopLevel(def.id));
            return;
        }
        self.errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
    }
    fn resolve_ctor(&mut self, name: &str, span: Span) {
        if let Some(def) = self.res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Ctor { .. })) {
            self.res.refs.insert(span, ResolvedName::Ctor(def.id));
        } else {
            self.errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
        }
    }
    fn resolve_type_or_ctor(&mut self, name: &str, span: Span) {
        if let Some(def) = self.res.defs.iter().find(|d| d.name == name) {
            let resolved = match def.kind {
                DefKind::Ctor { .. } => ResolvedName::Ctor(def.id),
                _ => ResolvedName::TopLevel(def.id),
            };
            self.res.refs.insert(span, resolved);
        } else {
            self.errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
        }
    }
}

pub(super) fn walk_file(file: &File, res: &mut Resolution, errors: &mut Vec<Error>) {
    let mut w = Walker { res, errors, scope: ScopeStack::new() };
    w.walk_file(file);
}
```

The free-function `walk_file` keeps `resolve_file` in `mod.rs` unchanged.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals`
Expected: all PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 6: lambda parameter scope

Introduce ScopeStack with push/pop/lookup of LocalIds. Restructure
the walker as a struct so the stack threads through recursion. A
lambda pushes a frame, binds each parameter pattern as a local, walks
the body, pops. Resolving a Var prefers locals over top-level.
Duplicate params in one lambda emit DuplicateLocal.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Pattern variable bindings (match arms)

Patterns can bind variables. `Ctor { args }`, `Tuple`, `List`, and `Record` patterns contain sub-patterns. `Wildcard` and `Lit` bind nothing. A match arm's pattern binds variables visible only in that arm's body.

**Files:**
- Modify: `src/resolve/walker.rs` (recursive `bind_pattern`; handle `Match`)
- Modify: `tests/resolver_locals.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
#[test]
fn match_arm_binds_pattern_vars() {
    let src = "module M\n    expose f\n\nf = x ->\n    x match\n        Some y -> y\n        None -> 0\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    // Add Option to scope by including a type decl.
    let src = "module M\n    expose f\n\ntype Option a\n    Some a\n    None\n\nf = x ->\n    x match\n        Some y -> y\n        None -> 0\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // The Var "y" inside the body of "Some y -> y" resolves to a Local.
    let local_refs: Vec<_> = res.refs.values().filter(|r| matches!(r, ResolvedName::Local(_))).collect();
    // x in scrutinee + y in arm body + x as param = 3 local refs (params count via... actually params don't make refs; only Var usages do).
    // So we expect: x (scrutinee) + y (arm body) = 2 Local refs.
    assert_eq!(local_refs.len(), 2);
}

#[test]
fn pattern_var_out_of_arm_is_error() {
    let src = "module M\n    expose f\n\ntype Option a\n    Some a\n    None\n\nf = x ->\n    z = x match\n        Some y -> y\n        None -> 0\n    y\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "y"
    )));
}
```

(`pattern_var_out_of_arm_is_error` also depends on Task 8 for block let-bindings to even parse the body. If the block walker isn't in yet, this second test is expected to FAIL for now — comment it out or mark `#[ignore]` and re-enable in Task 8's commit.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- match_arm`
Expected: FAIL — match isn't walked; `y` doesn't bind.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/walker.rs` — replace stub `bind_pattern` with recursive version; handle `Match`:

```rust
fn bind_pattern(&mut self, p: &Pattern) {
    match &p.node {
        PatternKind::Wildcard | PatternKind::Lit(_) => {}
        PatternKind::Var(name) => {
            if self.scope.push_local(name).is_err() {
                self.errors.push(Error {
                    span: p.span,
                    kind: ErrorKind::DuplicateLocal { name: name.clone() },
                });
            }
        }
        PatternKind::Ctor { name, args } => {
            // Resolve the constructor name itself.
            self.resolve_ctor(name, p.span);
            for sub in args { self.bind_pattern(sub); }
        }
        PatternKind::Tuple(items) | PatternKind::List(items) => {
            for sub in items { self.bind_pattern(sub); }
        }
        PatternKind::Record { type_name, fields } => {
            self.resolve_type_or_ctor(type_name, p.span);
            for fp in fields { self.bind_pattern(&fp.pattern); }
        }
    }
}
```

Add `Match` to `walk_expr`:

```rust
ExprKind::Match { scrutinee, arms } => {
    self.walk_expr(scrutinee);
    for arm in arms {
        self.scope.push_frame();
        self.bind_pattern(&arm.pattern);
        self.walk_expr(&arm.body);
        self.scope.pop_frame();
    }
}
```

Add `Call`, `MethodCall`, `FieldAccess`, `Bang`, `Question`, `Block` stubs that recurse (don't introduce locals yet — Block bindings come in Task 8):

```rust
ExprKind::Call { func, args } => {
    self.walk_expr(func);
    for a in args { self.walk_expr(a); }
}
ExprKind::MethodCall { receiver, .. } => self.walk_expr(receiver),
ExprKind::FieldAccess { receiver, .. } => self.walk_expr(receiver),
ExprKind::Bang(inner) | ExprKind::Question(inner) => self.walk_expr(inner),
ExprKind::Block(items) => self.walk_block_stub(items),
```

And a stub:

```rust
fn walk_block_stub(&mut self, items: &[crate::ast::BlockItem]) {
    for item in items {
        match item {
            crate::ast::BlockItem::Expr(e) => self.walk_expr(e),
            crate::ast::BlockItem::Binding(_) => {} // Task 8.
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals -- match_arm`
Expected: PASS.

Keep `pattern_var_out_of_arm_is_error` ignored until Task 8.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 7: pattern variable bindings

Recursive bind_pattern handles Var, Ctor, Tuple, List, Record. Match
arms push a frame, bind the pattern, walk the body, pop. Constructor
and record patterns also resolve the type name they reference.
Block let-bindings remain stubbed; Task 8 enables them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Block let-binding scope

A block `name =` introduces a binding visible to *later* items in the same block. Unlike top-level, block bindings are sequential — `a = 1\nb = a` works, but `b = a\na = 1` doesn't.

**Files:**
- Modify: `src/resolve/walker.rs` (real `walk_block`)
- Modify: `tests/resolver_locals.rs` (un-ignore the pattern-out-of-arm test)

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
#[test]
fn block_let_binding_visible_later() {
    let src = "module M\n    expose f\n\nf = x ->\n    a = x + 1\n    b = a + 1\n    b\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let local_refs: Vec<_> = res.refs.values().filter(|r| matches!(r, ResolvedName::Local(_))).collect();
    // x (in a = x+1), a (in b = a+1), b (final expr) = 3.
    assert_eq!(local_refs.len(), 3);
}

#[test]
fn block_let_binding_not_visible_earlier() {
    let src = "module M\n    expose f\n\nf =\n    a = b\n    b = 1\n    a\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "b"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- block_let`
Expected: FAIL — `walk_block_stub` ignores bindings.

- [ ] **Step 3: Write minimal implementation**

Replace the stub:

```rust
fn walk_block(&mut self, items: &[crate::ast::BlockItem]) {
    self.scope.push_frame();
    for item in items {
        match item {
            crate::ast::BlockItem::Expr(e) => self.walk_expr(e),
            crate::ast::BlockItem::Binding(decl) => {
                if let DeclKind::Binding { name, value: Some(v), .. } = &decl.node {
                    self.walk_expr(v);
                    if self.scope.push_local(name).is_err() {
                        self.errors.push(Error {
                            span: decl.span,
                            kind: ErrorKind::DuplicateLocal { name: name.clone() },
                        });
                    }
                }
            }
        }
    }
    self.scope.pop_frame();
}
```

Crucial detail: walk the value *before* pushing the new local. Otherwise `a = a` would self-reference. This makes block let bindings strictly sequential.

Rename `walk_block_stub` callsite to `walk_block`.

Un-ignore the `pattern_var_out_of_arm_is_error` test from Task 7.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals`
Expected: all PASS, including the previously ignored test.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 8: block let-binding scope

A block let-binding becomes visible to later items in the same
block. The walker walks the binding's value before pushing the
name, so a = a fails to resolve (the inner a doesn't see its own
definition). DuplicateLocal fires on a re-bound name within one
block.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Synthetic `self` in type-block methods

Inside a `type` block, an `=` binding whose RHS is a function (a `Lambda`, or any expression in the body of a binding that takes `self` implicitly) gets a synthetic `self` local in its body. The resolver walks methods by descending into `TypeBody::Block` items of kind `Method`.

**Files:**
- Modify: `src/resolve/walker.rs` (descend into methods, inject self)
- Modify: `tests/resolver_locals.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
#[test]
fn self_resolves_in_method() {
    let src = "module M\n    expose Point\n\ntype Point\n    x : Float\n    y : Float\n    sumXY = self.x + self.y\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let self_refs: Vec<_> = res.refs.values().filter(|r| matches!(r, ResolvedName::Local(_))).collect();
    // Two uses of self in the body.
    assert_eq!(self_refs.len(), 2);
}

#[test]
fn self_not_in_scope_outside_method() {
    let src = "module M\n    expose f\n\nf = self\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "self"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- self_`
Expected: FAIL — type bodies aren't walked; `self` is just an unknown name.

- [ ] **Step 3: Write minimal implementation**

In `walk_decl`, descend into `TypeDecl` bodies:

```rust
fn walk_decl(&mut self, decl: &Decl) {
    match &decl.node {
        DeclKind::Binding { value: Some(v), .. } => self.walk_expr(v),
        DeclKind::TypeDecl { body, .. } => self.walk_type_body(body),
        DeclKind::TraitDecl { methods, .. } | DeclKind::ImplDecl { methods, .. } => {
            for m in methods { self.walk_method(m); }
        }
        _ => {}
    }
}

fn walk_type_body(&mut self, body: &crate::ast::TypeBody) {
    use crate::ast::{TypeBody, TypeMember, VariantBody};
    match body {
        TypeBody::Newtype(_) => {} // type expressions handled in Task 10
        TypeBody::Block(members) => {
            for m in members {
                match m {
                    TypeMember::Field { .. } => {}
                    TypeMember::Method(d) => self.walk_method(d),
                    TypeMember::Variant { body: VariantBody::Fields(sub), .. } => {
                        // Nested fields don't have methods directly attached at this AST shape;
                        // ignore for now.
                        let _ = sub;
                    }
                    TypeMember::Variant { .. } => {}
                }
            }
        }
    }
}

fn walk_method(&mut self, decl: &Decl) {
    if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
        self.scope.push_frame();
        let _ = self.scope.push_local("self");
        self.walk_expr(v);
        self.scope.pop_frame();
    }
}
```

The `self` injection happens for any `=` binding inside a `type`/`trait`/`impl` block, not just lambdas. Per spec, a field binding uses `:` and a method binding uses `=`, so the AST already distinguishes them via `TypeMember::Field` vs `TypeMember::Method`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals -- self_`
Expected: both PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 9: synthetic self in methods

Walk TypeDecl, TraitDecl, and ImplDecl bodies. Every Method binding
pushes a frame containing `self` before walking its body. Field
declarations are skipped (no body). Variant bodies are walked
structurally but contribute no method scope of their own. `self`
outside a type/trait/impl block is unresolved.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Resolve type expressions

Types appear in field declarations, type signatures, and `Construct`/`Update` expressions. Names referenced from types live in the *type* namespace (so `Int`, `Float`, `Point` resolve there). Type variables (`a` in `Option a`) are locally bound — for now, treat any lowercase name in type position as a local type variable that resolves to itself without recording a ref.

**Files:**
- Modify: `src/resolve/walker.rs` (add `walk_type`)
- Modify: `tests/resolver_locals.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_locals.rs`:

```rust
use i_lang::resolve::DefKind;

#[test]
fn type_in_annotation_resolves() {
    let src = "module M\n    expose Point\n\ntype Point\n    x : Float\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    // We use a stub-known type by pre-registering Float as a top-level type via parsing it.
    // For this test, expect the resolver to *fail* on Float because it isn't defined.
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "Float"
    )));
}

#[test]
fn known_type_resolves_in_annotation() {
    let src = "module M\n    expose pi\n\ntype Float\n\npi : Float\npi = 3\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // One ref: Float in pi's annotation.
    assert!(res.refs.values().any(|r| matches!(r, ResolvedName::TopLevel(_))));
}
```

Adjust as needed if `TypeDecl { body: TypeBody::Newtype(...) }` parses differently for `type Float\n` (with no body). If it doesn't parse, replace with `type Float\n    dummy : Int\n` or similar; the test's *intent* is to register a type then reference it.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_locals -- type_in_annotation`
Expected: FAIL — types are not walked yet.

- [ ] **Step 3: Write minimal implementation**

Add `walk_type` and route through it from binding annotations, type-body fields, lambda/function signatures, and Construct/Update type-name slots already partly covered.

```rust
fn walk_type(&mut self, t: &crate::ast::Type) {
    use crate::ast::TypeKind;
    match &t.node {
        TypeKind::Var(_name) => {
            // Lowercase type variable. v1: no binding tracking;
            // accept silently. (Plan 4's type-checker scopes these.)
        }
        TypeKind::Named { name, args } => {
            self.resolve_type_name(name, t.span);
            for a in args { self.walk_type(a); }
        }
        TypeKind::Function { params, effect, result } => {
            for p in params { self.walk_type(p); }
            if let Some(crate::ast::EffectRow::Named(names)) = effect {
                for n in names {
                    // Effect names resolve as type names.
                    self.resolve_type_name(n, t.span);
                }
            }
            self.walk_type(result);
        }
        TypeKind::Tuple(items) => for i in items { self.walk_type(i); },
    }
}

fn resolve_type_name(&mut self, name: &str, span: Span) {
    if let Some(def) = self.res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Type | DefKind::Trait)) {
        self.res.refs.insert(span, ResolvedName::TopLevel(def.id));
    } else {
        self.errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
    }
}
```

Then in `walk_decl`, also walk binding annotations:

```rust
DeclKind::Binding { ty, value, .. } => {
    if let Some(t) = ty { self.walk_type(t); }
    if let Some(v) = value { self.walk_expr(v); }
}
```

In `walk_type_body`, walk field types:

```rust
TypeMember::Field { ty, .. } => self.walk_type(ty),
```

In `walk_method`, walk method-binding annotations too if present.

In `walk_expr` for `ExprKind::Construct { type_name, .. }`, the type_name is a String — but it's a *type* reference, not a value. Update `resolve_type_or_ctor` to also consider `DefKind::Type` (it already does, since the only non-Ctor non-Value match is Type).

Per the AST, lambdas don't have parameter type annotations; functions get them via separate annotated bindings. So we don't need to walk types inside `ExprKind::Lambda`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_locals -- type_`
Expected: both PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_locals.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 10: resolve type expressions

walk_type recurses through TypeKind::Named, Function, Tuple; named
types and effect-row names resolve against the type namespace.
Lowercase type variables (TypeKind::Var) are accepted silently —
Plan 4's type checker will scope them properly. Binding type
annotations and field declarations now feed through walk_type.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Module set abstraction and `use Path` whole-module

Cross-module resolution starts. Introduce `ModuleSet` (a `HashMap<ModulePath, File>`) and a `resolve_project` entry point. A `use Path` decl registers `Path` as a visible module in the importing file; the importing file can then write `Path.name` to access exported values.

**Files:**
- Create: `src/resolve/module_set.rs`
- Modify: `src/resolve/mod.rs` (add `resolve_project`, `ModuleSet`)
- Modify: `src/resolve/scope.rs` (track imported module paths per file)
- Modify: `src/resolve/walker.rs` (handle module-qualified `FieldAccess` chains)
- Test: `tests/resolver_modules.rs` (NEW)

- [ ] **Step 1: Write the failing test**

`tests/resolver_modules.rs`:

```rust
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::{resolve_project, ModulePath, ModuleSet, ResolvedName};
use std::collections::HashMap;

fn parse_module(src: &str) -> i_lang::ast::File {
    parse(&lex(src).unwrap()).unwrap()
}

#[test]
fn use_whole_module_resolves_qualified_access() {
    let lib = parse_module("module Geometry\n    expose distance\n\ndistance = a -> a\n");
    let app = parse_module("module Main\n    expose main\n\nuse Geometry\n\nmain = Geometry.distance\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();
    // The expression Geometry.distance should resolve to Imported { module: ["Geometry"], name: "distance" }.
    assert!(main_res.refs.values().any(|r| matches!(
        r,
        ResolvedName::Imported { module, name } if module == &vec!["Geometry".to_string()] && name == "distance"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_modules -- use_whole_module`
Expected: FAIL — `resolve_project`, `ModuleSet` don't exist.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/module_set.rs`:

```rust
use std::collections::HashMap;
use crate::ast::File;
use crate::error::Error;
use super::types::{ModulePath, Resolution};

pub type ModuleSet = HashMap<ModulePath, File>;
pub type ProjectResolution = HashMap<ModulePath, Resolution>;

pub fn resolve_project(set: &ModuleSet) -> Result<ProjectResolution, Vec<Error>> {
    let mut out = ProjectResolution::new();
    let mut all_errors = Vec::new();
    for (path, file) in set {
        match super::resolve_file_in_set(file, set) {
            Ok(res) => { out.insert(path.clone(), res); }
            Err(errs) => all_errors.extend(errs),
        }
    }
    if all_errors.is_empty() { Ok(out) } else { Err(all_errors) }
}
```

`src/resolve/mod.rs`:

```rust
mod module_set;
mod scope;
mod types;
mod walker;

pub use module_set::{resolve_project, ModuleSet, ProjectResolution};
pub use types::*;

use crate::ast::File;
use crate::error::Error;
use std::collections::HashMap;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    resolve_file_in_set(file, &HashMap::new())
}

pub(crate) fn resolve_file_in_set(file: &File, set: &ModuleSet) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let mut errors = scope::collect_top_level(file, &mut res);
    let imports = scope::collect_imports(file, set, &mut errors);
    walker::walk_file(file, &mut res, &mut errors, &imports);
    if errors.is_empty() { Ok(res) } else { Err(errors) }
}
```

`src/resolve/scope.rs` — add `collect_imports`:

```rust
use super::types::ModulePath;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Imports {
    pub modules: Vec<ModulePath>, // whole-module imports
    pub cherries: HashMap<String, (ModulePath, String)>, // local name -> (module, original name)
    pub aliases: HashMap<String, ModulePath>, // local alias -> module path
}

pub(super) fn collect_imports(file: &File, set: &ModuleSet, errors: &mut Vec<Error>) -> Imports {
    let mut imp = Imports::default();
    for decl in &file.decls {
        if let DeclKind::Use { path, kind } = &decl.node {
            if !set.contains_key(path) {
                errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::UnknownModule { path: path.clone() },
                });
                continue;
            }
            match kind {
                crate::ast::UseKind::Whole => imp.modules.push(path.clone()),
                // Cherry and Alias handled in Tasks 12, 13.
                _ => {}
            }
        }
    }
    imp
}
```

Add `ErrorKind::UnknownModule { path: Vec<String> }`.

`src/resolve/walker.rs` — thread imports through `Walker`:

```rust
pub(super) struct Walker<'a> {
    pub res: &'a mut Resolution,
    pub errors: &'a mut Vec<Error>,
    pub scope: ScopeStack,
    pub imports: &'a super::scope::Imports,
}
```

Add module-qualified access handling. When walking `ExprKind::FieldAccess`, collect the chain bottom-up; if the leaf is `Var(Name)` where `Name` is the first component of an imported module path, walk up matching the longest prefix. If a match succeeds, mark the *whole chain expression's span* with `ResolvedName::Imported { module, name: field }` (where `name` is whatever's left after the prefix).

Pragmatic version for Task 11 (no aliases yet; only `use Path` whole-module):

```rust
fn try_resolve_qualified(&mut self, e: &Expr) -> bool {
    // Collect chain: returns Vec<&str> for the dotted path, with the outermost
    // FieldAccess's field name as the *last* element.
    let mut path: Vec<String> = Vec::new();
    let mut node = e;
    loop {
        match &node.node {
            ExprKind::FieldAccess { receiver, field } => {
                path.push(field.clone());
                node = receiver;
            }
            ExprKind::Var(name) => {
                path.push(name.clone());
                break;
            }
            _ => return false,
        }
    }
    path.reverse(); // now in source order: ["Std", "IO", "print"]

    // Try longest module-path prefix first.
    for split in (1..path.len()).rev() {
        let module_path: ModulePath = path[..split].to_vec();
        if self.imports.modules.iter().any(|m| m == &module_path) {
            let name = path[split..].join(".");
            // Only single-segment trailing names are supported here.
            if path.len() - split == 1 {
                self.res.refs.insert(e.span, ResolvedName::Imported {
                    module: module_path,
                    name: path[split].clone(),
                });
                return true;
            } else {
                // Chained method/field access on imported value — record the
                // module hit, and walk the inner FieldAccess as Imported.
                // For Task 11 simplicity, just record the leaf reference.
                self.res.refs.insert(e.span, ResolvedName::Imported {
                    module: module_path,
                    name: path[split].clone(),
                });
                return true;
            }
        }
    }
    false
}
```

Call it from `walk_expr` for `ExprKind::FieldAccess`:

```rust
ExprKind::FieldAccess { receiver, .. } => {
    if self.try_resolve_qualified(e) { return; }
    self.walk_expr(receiver);
}
```

Free function:

```rust
pub(super) fn walk_file(
    file: &File,
    res: &mut Resolution,
    errors: &mut Vec<Error>,
    imports: &super::scope::Imports,
) {
    let mut w = Walker { res, errors, scope: ScopeStack::new(), imports };
    w.walk_file(file);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_modules`
Expected: PASS.

Run all tests + `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_modules.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 11: module set and whole-module use

Introduce ModuleSet (HashMap<ModulePath, File>) and resolve_project
which resolves every file in the set against the rest. `use Path`
registers a module path; qualified access like Geometry.distance
walks the FieldAccess chain bottom-up, matches the longest module
prefix, and records ResolvedName::Imported. Unknown module paths
emit UnknownModule.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: `use Path (a, b)` cherry-pick

Cherry-picked names enter the local value namespace. `use Std.IO (print)` makes `print` resolve as `Imported { module: ["Std", "IO"], name: "print" }` when used unqualified.

**Files:**
- Modify: `src/resolve/scope.rs` (populate `Imports::cherries`)
- Modify: `src/resolve/walker.rs` (consult cherries in `resolve_var`)
- Modify: `tests/resolver_modules.rs`
- Verify cherry-picked names also error when the source module doesn't export them — but exposure checking is Task 15.

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_modules.rs`:

```rust
#[test]
fn use_cherry_pick_brings_names_unqualified() {
    let lib = parse_module("module Geometry\n    expose Point, distance\n\ntype Point\n    x : Float\n    y : Float\n\ndistance = a -> a\n");
    let app = parse_module("module Main\n    expose main\n\nuse Geometry (Point, distance)\n\nmain = distance\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();
    assert!(main_res.refs.values().any(|r| matches!(
        r,
        ResolvedName::Imported { module, name } if module == &vec!["Geometry".to_string()] && name == "distance"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_modules -- use_cherry_pick`
Expected: FAIL — cherry-picked names aren't in scope.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/scope.rs` — extend `collect_imports`:

```rust
UseKind::Cherry(names) => {
    for n in names {
        imp.cherries.insert(n.clone(), (path.clone(), n.clone()));
    }
}
```

`src/resolve/walker.rs` — in `resolve_var`, check cherries before falling through to "unresolved":

```rust
fn resolve_var(&mut self, name: &str, span: Span) {
    if let Some(id) = self.scope.lookup_local(name) {
        self.res.refs.insert(span, ResolvedName::Local(id));
        return;
    }
    if let Some(def) = self.res.defs.iter().find(|d| d.name == name && matches!(d.kind, DefKind::Value)) {
        self.res.refs.insert(span, ResolvedName::TopLevel(def.id));
        return;
    }
    if let Some((module, original)) = self.imports.cherries.get(name) {
        self.res.refs.insert(span, ResolvedName::Imported { module: module.clone(), name: original.clone() });
        return;
    }
    self.errors.push(Error { span, kind: ErrorKind::Unresolved { name: name.to_string() } });
}
```

Same treatment in `resolve_ctor` for upper-name cherries (constructors and type names can also be cherry-picked).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_modules`
Expected: all PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_modules.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 12: cherry-pick use

`use Path (a, b)` puts each name into the importing file's local
namespace, resolving to ResolvedName::Imported. The walker checks
the cherries table after locals and top-level but before erroring.
Both lowercase (values, ctors) and uppercase (types) work because
the AST stores cherry-picked names as plain strings.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: `use Path as Alias` alias

Alias-imports rename a module locally. `use Std.Float as F` makes `F.parse` work; the original `Std.Float.parse` is *not* in scope (per spec).

**Files:**
- Modify: `src/resolve/scope.rs` (populate `Imports::aliases`)
- Modify: `src/resolve/walker.rs` (consult aliases in qualified-access resolution)
- Modify: `tests/resolver_modules.rs`
- Modify: `src/ast/mod.rs`, `src/parse/decl.rs`, `src/ast/display.rs`, `src/pretty.rs` — see "Parser amendment" below.

**Parser amendment (deviation from original task scope).** The failing tests for this task assumed `module Std.Float` (dotted module header) parses. The v1 parser only accepts a single uppercase identifier. `docs/modules.md` § Layout shows dotted module headers as canonical (`module Std.IO`, `module Std.Float`), so this is a parser gap, not a spec extension. The fix is folded into Task 13's commit:

- `ModuleHeader.name: String` → `Vec<String>` (matches how `use` paths are stored).
- `parse_module_header` loops on `Dot` between `expect_upper` calls, mirroring `parse_use_decl`.
- `ast/display.rs` and `pretty.rs` join the segments with `.` when rendering.

Only one construction site of `ModuleHeader` exists (the parser) and no tests inspect `.name`, so the blast radius is small.

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_modules.rs`:

```rust
#[test]
fn use_alias_resolves_qualified() {
    let lib = parse_module("module Std.Float\n    expose parse\n\nparse = x -> x\n");
    let app = parse_module("module Main\n    expose main\n\nuse Std.Float as F\n\nmain = F.parse\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Std".into(), "Float".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();
    assert!(main_res.refs.values().any(|r| matches!(
        r,
        ResolvedName::Imported { module, name }
            if module == &vec!["Std".to_string(), "Float".to_string()] && name == "parse"
    )));
}

#[test]
fn aliased_original_path_is_unavailable() {
    let lib = parse_module("module Std.Float\n    expose parse\n\nparse = x -> x\n");
    let app = parse_module("module Main\n    expose main\n\nuse Std.Float as F\n\nmain = Std.Float.parse\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Std".into(), "Float".into()], lib);
    set.insert(vec!["Main".into()], app);
    let errs = resolve_project(&set).unwrap_err();
    assert!(errs.iter().any(|e| matches!(&e.kind, ErrorKind::Unresolved { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_modules -- alias`
Expected: FAIL — aliases not tracked; the original path still resolves.

- [ ] **Step 3: Write minimal implementation**

`src/resolve/scope.rs`:

```rust
UseKind::Alias(local) => {
    imp.aliases.insert(local.clone(), path.clone());
}
```

`src/resolve/walker.rs` — in `try_resolve_qualified`, before checking `imports.modules`, also check whether the first segment of `path` is an alias. If so, substitute the alias's target:

```rust
// After collecting `path`:
if let Some(target) = self.imports.aliases.get(&path[0]) {
    // Replace path[0..1] with target.
    let mut full = target.clone();
    full.extend(path[1..].iter().cloned());
    // Then run the same longest-prefix match against `target` itself.
    if full.len() > target.len() {
        let module_path = target.clone();
        let name = full[target.len()].clone();
        self.res.refs.insert(e.span, ResolvedName::Imported { module: module_path, name });
        return true;
    }
}
```

For the second test (the original path no longer works): since `use Std.Float as F` doesn't add `Std.Float` to `imports.modules`, the path-as-written falls through and the leaf var `Std` is unresolved (Std is not a module on its own unless separately imported).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_modules`
Expected: all PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/resolve tests/resolver_modules.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 13: aliased use

`use Path as Alias` registers Alias -> Path in the imports table.
Qualified-access resolution first tries the alias table, then the
whole-module list. Aliased imports do NOT register the original
path, matching the spec's "the original path is unavailable in this
file" rule. The leaf var of an unregistered path-prefix falls
through to Unresolved.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: Module cycle detection

A module DAG. If A uses B and B uses A (directly or transitively), error with all members of the cycle named. Per `docs/modules.md § 6` this is v1 behaviour.

**Files:**
- Modify: `src/resolve/module_set.rs` (DFS cycle check before per-file resolution)
- Modify: `src/error.rs` (add `ModuleCycle { members: Vec<Vec<String>> }`)
- Modify: `tests/resolver_modules.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_modules.rs`:

```rust
use i_lang::error::ErrorKind;

#[test]
fn module_cycle_detected() {
    let a = parse_module("module A\n    expose x\n\nuse B\n\nx = 1\n");
    let b = parse_module("module B\n    expose y\n\nuse A\n\ny = 2\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["A".into()], a);
    set.insert(vec!["B".into()], b);
    let errs = resolve_project(&set).unwrap_err();
    assert!(errs.iter().any(|e| matches!(&e.kind, ErrorKind::ModuleCycle { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_modules -- module_cycle`
Expected: FAIL — `ModuleCycle` doesn't exist; resolver still tries to resolve both files normally (and presumably succeeds because the cycle hasn't broken anything yet).

- [ ] **Step 3: Write minimal implementation**

`src/error.rs`:

```rust
ModuleCycle { members: Vec<Vec<String>> },
```

`src/resolve/module_set.rs` — DFS cycle detection on the import graph. Run before per-file resolution; if a cycle is found, return early with a single `ModuleCycle` error per cycle.

```rust
fn detect_cycles(set: &ModuleSet) -> Vec<Error> {
    use std::collections::HashSet;
    let mut visited: HashSet<ModulePath> = HashSet::new();
    let mut stack: Vec<ModulePath> = Vec::new();
    let mut on_stack: HashSet<ModulePath> = HashSet::new();
    let mut errors: Vec<Error> = Vec::new();

    fn dfs(
        node: &ModulePath,
        set: &ModuleSet,
        visited: &mut HashSet<ModulePath>,
        stack: &mut Vec<ModulePath>,
        on_stack: &mut HashSet<ModulePath>,
        errors: &mut Vec<Error>,
    ) {
        if on_stack.contains(node) {
            let cycle_start = stack.iter().position(|m| m == node).unwrap();
            let members = stack[cycle_start..].to_vec();
            errors.push(Error {
                span: crate::span::Span::new(0, 0),
                kind: ErrorKind::ModuleCycle { members },
            });
            return;
        }
        if visited.contains(node) { return; }
        let Some(file) = set.get(node) else { return; };
        stack.push(node.clone());
        on_stack.insert(node.clone());
        for decl in &file.decls {
            if let DeclKind::Use { path, .. } = &decl.node {
                dfs(path, set, visited, stack, on_stack, errors);
            }
        }
        stack.pop();
        on_stack.remove(node);
        visited.insert(node.clone());
    }

    for path in set.keys() {
        dfs(path, set, &mut visited, &mut stack, &mut on_stack, &mut errors);
    }
    errors
}
```

Wire it into `resolve_project`:

```rust
pub fn resolve_project(set: &ModuleSet) -> Result<ProjectResolution, Vec<Error>> {
    let cycle_errs = detect_cycles(set);
    if !cycle_errs.is_empty() { return Err(cycle_errs); }
    // ... rest unchanged
}
```

Note: the cycle DFS uses `Span::new(0, 0)` because the error is about the *module graph*, not any source position. Future task: replace with the span of one of the `use` decls in the cycle.

Use `crate::ast::DeclKind` and `crate::ast::UseKind` imports as needed.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_modules`
Expected: all PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_modules.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 14: module cycle detection

DFS the import graph before per-file resolution. A back-edge to a
module currently on the stack is a cycle; emit ModuleCycle naming
all members from the cycle start to the back-edge. Per spec, modules
form a DAG in v1. The cycle span is zero for now (cycle is a graph
property, not source-positional); a follow-up can attribute it to
one of the `use` decls in the cycle.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 15: Unknown imported name + unexported access

When a cherry-picked name is requested but the source module doesn't `expose` it, that's a compile-time error. Same for qualified access to an unexposed name.

**Files:**
- Modify: `src/resolve/module_set.rs` (build per-module export tables; validate imports)
- Modify: `src/error.rs` (add `NotExposed { module, name }`)
- Modify: `tests/resolver_modules.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_modules.rs`:

```rust
#[test]
fn cherry_pick_unexposed_is_error() {
    let lib = parse_module("module Geometry\n    expose distance\n\nsquare = x -> x\ndistance = a -> a\n");
    let app = parse_module("module Main\n    expose main\n\nuse Geometry (square)\n\nmain = square\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let errs = resolve_project(&set).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::NotExposed { name, .. } if name == "square"
    )));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_modules -- cherry_pick_unexposed`
Expected: FAIL — exposure isn't validated; the resolver just accepts the import.

- [ ] **Step 3: Write minimal implementation**

`src/error.rs`:

```rust
NotExposed { module: Vec<String>, name: String },
```

`src/resolve/module_set.rs` — build an export table per module from `file.module.exposes`, then check each cherry-pick (and each qualified access). Pre-compute exports in `resolve_project`:

```rust
fn exports_of(file: &File) -> Vec<String> {
    let Some(h) = &file.module else { return Vec::new(); };
    h.exposes.iter().flat_map(|e| match e {
        crate::ast::Expose::Value(n) => vec![n.clone()],
        crate::ast::Expose::Type { name, with_constructors } => {
            // For now, just the type name itself; cycle through ctors if .. set.
            // We don't have ctor lists at this layer — accept any name beginning
            // upper-case (rough cut) when with_constructors is true.
            // For Task 15 a simple rule: NotExposed is checked only against
            // explicit Value exposes; type cherries are not validated yet.
            let _ = with_constructors;
            vec![name.clone()]
        }
    }).collect()
}
```

Validate cherries: in `collect_imports` (or in a new helper called from `resolve_project`), after building the imports for a file, check each cherry against the source module's exports:

```rust
pub(super) fn validate_cherries(
    path: &ModulePath,
    imports: &Imports,
    set: &ModuleSet,
    errors: &mut Vec<Error>,
) {
    for (_, (module, original)) in &imports.cherries {
        let Some(src_file) = set.get(module) else { continue; };
        let exposed = super::module_set::exports_of(src_file);
        if !exposed.contains(original) {
            errors.push(Error {
                span: crate::span::Span::new(0, 0), // ideally the use-decl span
                kind: ErrorKind::NotExposed { module: module.clone(), name: original.clone() },
            });
        }
    }
    let _ = path;
}
```

(`exports_of` is `pub(super)`.)

Call `validate_cherries` from `resolve_file_in_set` after collecting imports.

For qualified-access validation: skip for Task 15 — easier to layer once the per-file ctor exposure rule is figured out. Add a note in the commit body.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test resolver_modules`
Expected: all PASS.

Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/resolve tests/resolver_modules.rs
git commit -m "$(cat <<'EOF'
Plan 3 Task 15: validate cherry-picked imports against exports

Build an export table per file from its `expose` clauses and check
each cherry-picked name against the source module's exports.
Emits NotExposed for absent names. Qualified-access exposure
validation (e.g. Geometry.private) is left for a follow-up; the
ctor-exposure rule (Type vs Type(..)) is the moving piece.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 16: Resolver corpus snapshot tests

Snapshot the `Resolution` for each fixture in `tests/corpus/resolver/`. Format is a custom `Display` on `Resolution` that prints, per resolved reference, `<span> <resolved-name>` one per line, sorted by span.

**Files:**
- Modify: `src/resolve/types.rs` (`Display for Resolution`)
- Create: `tests/corpus/resolver/*.i` (9 fixtures listed below)
- Create: `tests/resolver_corpus.rs`

- [ ] **Step 1: Write the failing test**

`tests/resolver_corpus.rs`:

```rust
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn snapshot_resolver_corpus() {
    insta::glob!(env!("CARGO_MANIFEST_DIR"), "tests/corpus/resolver/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).unwrap();
        let file = parse(&toks).unwrap();
        let res = resolve_file(&file).expect("corpus fixtures must resolve");
        insta::assert_snapshot!(format!("{}", res));
    });
}
```

Create the fixtures:

```
tests/corpus/resolver/single-binding.i:
    module M
        expose x

    x = 1

tests/corpus/resolver/lambda-shadow.i:
    module M
        expose f

    x = 1
    f = x -> x

tests/corpus/resolver/match-binds.i:
    module M
        expose f

    type Option a
        Some a
        None

    f = o ->
        o match
            Some y -> y
            None   -> 0

tests/corpus/resolver/method-self.i:
    module M
        expose Point

    type Point
        x : Float
        y : Float
        sumXY = self.x + self.y

tests/corpus/resolver/ctor-resolve.i:
    module M
        expose make

    type Shape
        Circle
            radius : Float
        Rect
            width : Float
            height : Float

    make = Circle(radius = 1.0)

tests/corpus/resolver/sum-with-methods.i:
    module M
        expose Pair

    type Pair
        a : Int
        b : Int
        sum = self.a + self.b
```

Then `qualified-call.i`, `cherry-pick.i`, `alias-import.i` need cross-module support — those work with `resolve_file` if we keep them simple (no cross-module references) OR they're skipped from this snapshot test in favour of single-file fixtures. For Task 16, keep the corpus single-file (six fixtures above). Task 17 below adds the multi-file integration test.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test resolver_corpus`
Expected: FAIL — `Display for Resolution` not implemented (compile error or empty output).

- [ ] **Step 3: Write minimal implementation**

`src/resolve/types.rs` — add Display:

```rust
impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "defs:")?;
        for d in &self.defs {
            writeln!(f, "  {:?} {} ({:?})", d.id, d.name, d.kind)?;
        }
        writeln!(f, "refs:")?;
        let mut entries: Vec<_> = self.refs.iter().collect();
        entries.sort_by_key(|(s, _)| (s.start, s.end));
        for (span, name) in entries {
            writeln!(f, "  {:?} -> {:?}", span, name)?;
        }
        Ok(())
    }
}
```

Run `cargo test --test resolver_corpus` — it will fail until snapshots are accepted. Tell the user:

> Run `cargo insta review` to inspect each `.snap.new`. Accept or reject per file. Do NOT use `cargo insta accept`.

- [ ] **Step 4: Hand off for snapshot review**

Stop here. Tell the user:

> Six new snapshot files are ready for review. Run `cargo insta review` and approve each one. When done, say "accepted" and I'll commit them.

- [ ] **Step 5: Commit (after user accepts)**

```bash
git add src/resolve tests/corpus tests/resolver_corpus.rs tests/snapshots
git commit -m "$(cat <<'EOF'
Plan 3 Task 16: resolver corpus snapshots

Six single-file fixtures (one per resolution feature) snapshotted
via insta::glob!. Resolution gains a Display impl that prints defs
then refs (sorted by span). Snapshots are human-reviewed; never run
cargo insta accept. Multi-file scenarios get their own integration
test in Task 17.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 17: Multi-file integration test

A single integration test that builds a small two-module project (a `Geometry` library and a `Main` app) inline, runs `resolve_project`, and asserts every expected resolution.

**Files:**
- Modify: `tests/resolver_modules.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/resolver_modules.rs`:

```rust
#[test]
fn two_module_project_resolves_end_to_end() {
    let lib = parse_module("module Geometry\n    expose Point, distance\n\ntype Point\n    x : Float\n    y : Float\n\ndistance = a b -> a.x - b.x\n");
    let app = parse_module("module Main\n    expose main\n\nuse Geometry (Point, distance)\n\nmain =\n    p1 = Point(x = 0, y = 0)\n    p2 = Point(x = 3, y = 4)\n    distance p1, p2\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();

    // We expect refs for: Point (in construct, twice), distance (call), p1, p2 (call args).
    let imported: Vec<_> = main_res.refs.values().filter(|r| matches!(r, ResolvedName::Imported { .. })).collect();
    assert!(imported.len() >= 3, "expected at least 3 imported refs, got {}", imported.len());

    // p1, p2 should resolve as locals in their use sites.
    let locals: Vec<_> = main_res.refs.values().filter(|r| matches!(r, ResolvedName::Local(_))).collect();
    assert!(locals.len() >= 2, "expected at least 2 local refs, got {}", locals.len());
}
```

- [ ] **Step 2: Run test to verify it fails or passes**

Run: `cargo test --test resolver_modules -- two_module_project`
Expected: PASS — if all prior tasks landed correctly, this should pass without new code. If it fails, the failure points at a gap (e.g., `distance p1, p2` parses as `Call { func: distance, args: [p1, p2] }` — the resolver must recurse into `Call` args; verify Task 7 wired that up).

If it fails, write minimal code to make it pass (likely a missed recursion case). Otherwise treat this as a "regression confirmation" test and proceed to commit.

- [ ] **Step 3: Write any missing code**

(If Step 2 passed, this step is a no-op — leave it empty and move on.)

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: everything PASS. Run `make ci`.

- [ ] **Step 5: Commit**

```bash
git add tests/resolver_modules.rs src/resolve
git commit -m "$(cat <<'EOF'
Plan 3 Task 17: end-to-end two-module integration test

A small two-module project (Geometry library + Main app) that
exercises type cherry-pick, value cherry-pick, type construction
with kwargs, paren-free call, and block let-bindings together.
This pins down that the per-form tests compose correctly when wired
through resolve_project. If it had failed, the failure would point
at the gap; it passed unchanged, confirming Tasks 1-15 compose.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 18: Document the resolver

Write a short doc explaining what name resolution does, the two-namespace model, the side-table output, and what's deferred to later plans. Plug it into the docs index and PROGRESS.md.

**Files:**
- Create: `docs/resolution.md`
- Modify: `docs/README.md` (add a link)
- Modify: `docs/superpowers/plans/PROGRESS.md` (mark Plan 3 done)

- [ ] **Step 1: Draft `docs/resolution.md`**

Write a doc with these sections:

1. **What it does** — one paragraph: walks an AST, decides what every name refers to, fills a side-table.
2. **Two namespaces** — value (bindings, ctors, imported values, cherry-picked names, lambda params, pattern vars, block lets, `self`); type (type names, traits, effect-row names).
3. **Output: `Resolution`** — `defs` vector, `refs` map; how to look up a reference (key by AST node's span).
4. **Scope rules** — top-level is whole-file (mutual recursion allowed); block lets are sequential; lambda params shadow; pattern vars scope to one arm; `self` injected for type-block methods.
5. **Cross-module** — three `use` forms (whole, cherry, alias); longest-prefix matching for qualified access; aliased imports hide the original path.
6. **Errors** — `Unresolved`, `DuplicateDefinition`, `DuplicateLocal`, `UnknownModule`, `NotExposed`, `ModuleCycle`.
7. **What's deferred** — trait-method dispatch, field/method disambiguation, stdlib auto-import, project loader, value-dependency analysis for let-poly groups.

Aim for ~150 lines. The audience is someone who's just landed on the project and needs to know where resolution sits in the pipeline.

- [ ] **Step 2: Link from `docs/README.md`**

Add under "Reference (random access)":

```
- [Name resolution](resolution.md) — what every identifier refers to
```

- [ ] **Step 3: Update PROGRESS.md**

Change line 26 from:

```
## Phase 2: Implementation — Plan 2 (lexer + parser) DONE
```

to leave that section as-is and append a new section under "Later v1 phases":

```
## Phase 3: Implementation — Plan 3 (name resolution) DONE
- [x] Resolver scaffold + data model (Task 1)
- [x] Top-level collection + duplicate detection (Tasks 2-3)
- [x] Var, Ctor, expression walker (Tasks 4-5)
- [x] Locals: lambda, patterns, blocks, self (Tasks 6-9)
- [x] Type expressions (Task 10)
- [x] Cross-module: use, cherry, alias, cycles, exposure (Tasks 11-15)
- [x] Corpus + integration tests (Tasks 16-17)
- [x] Resolver documentation (Task 18)
```

And remove the corresponding `- [ ] Name resolution — Plan 3 (TBD)` line under "Later v1 phases".

- [ ] **Step 4: Run `make ci`**

No tests changed, so nothing should break. Confirm clean.

- [ ] **Step 5: Commit**

```bash
git add docs/resolution.md docs/README.md docs/superpowers/plans/PROGRESS.md
git commit -m "$(cat <<'EOF'
Plan 3 Task 18: document the resolver

resolution.md explains the resolver's role, the two-namespace
model, the side-table output shape, scope rules, cross-module
behaviour, and the error catalogue. Plus what's deferred to later
plans (trait dispatch, field/method disambiguation, project
loader, value-dependency analysis). PROGRESS.md gets a Phase 3
section. Plan 3 is now complete.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```
