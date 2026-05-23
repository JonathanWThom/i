# Plan 4 — Type Checker (HM core)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Walk a parsed and resolved `File` and, for every expression and pattern, infer a type. Reject programs that don't type-check: mismatched types, occurs-check failures, missing or unknown record fields, non-exhaustive matches, ambiguous type variables. Produce a `Typing` side-table mapping each top-level binding to a `Scheme` and each expression / pattern's `Span` to its inferred `Ty`. Lay the groundwork (`Ty`, `Scheme`, `Subst`, `unify`, `Infer`) that Plans 5–7 extend.

**Architecture:** Algorithm-W-style inference, single recursive pass per expression, eager unification against a mutable substitution. Top-level bindings handled in two phases — phase 1 allocates a fresh tyvar scheme for each top-level def so mutual recursion type-checks; phase 2 infers each body, then generalises unconstrained free variables. Type declarations are pre-collected into a `TypeRegistry` (fields, variants, methods, constructor schemes) before any expression is walked. Output is a `Typing` side-table keyed by `Span` — the AST and `Resolution` are unchanged.

**Tech Stack:** Rust 1.95, edition 2024. No new crates — `std::collections::HashMap` is enough. Errors reuse `crate::error::{Error, ErrorKind}` with new variants added per-task as they're first needed.

---

## Decisions baked in

These were open questions before the plan; they're resolved here so they don't reopen mid-task.

1. **Algorithm W, not constraint generation + solve.** Each expression is inferred recursively; unification is performed eagerly on a mutable `Subst`. Constraints aren't materialised as a separate IR. The trade-off is that error spans are slightly less flexible (we get them from the expression currently being walked, not from a constraint's origin span), but the implementation fits in two files and is easier to read. If error quality ever demands it, we'll move to a constraint set in a later plan.

2. **Output is a side-table, not a typed AST.** Mirrors Plan 3. `Typing.expr_types: HashMap<Span, Ty>` records the resolved type at each expression site after final substitution. `Typing.pattern_types` does the same for patterns. `Typing.schemes` holds per-`DefId` generalised schemes. Downstream passes (interpreter, code gen) read by span and ID. No AST mutation, no `TypedExpr` duplication.

3. **Top-level generalisation only, SCC-by-SCC.** Top-level value bindings are processed in strongly-connected-component order: dependency-sort by which other top-level defs each body references (via the resolver's `refs`), compute SCCs with Tarjan's algorithm, and visit them in topological order. For each SCC: (a) pre-declare a fresh tyvar `Scheme { vars: [], ty: Var(α_i) }` for every binding in the SCC; (b) apply user annotations (lower their `ast::Type` and unify against the tyvar) before inference; (c) infer each body and unify with its tyvar; (d) resolve the schemes through the final substitution; (e) generalise — quantify free tyvars not pinned by other (already-generalised) schemes. Later SCCs see polymorphic schemes for earlier ones, so `id = x -> x; n = id 1; s = id "hi"` typechecks (id is generalised before being used at two types). Mutually recursive bindings land in the same SCC and stay monomorphic *within* that SCC (the standard "no polymorphic recursion" cut). Block let-bindings remain monomorphic — their types are inferred but not generalised. This matches Plan 3's "Plan 4 owns let-poly" promise and the spec (`types.md § 2`).

   *Amendment notice:* the original phrasing here ("phase 2 infers each body, phase 3 generalises the whole file") could not deliver let-polymorphism across top-level uses — each use of `id` during phase 2 would share the same un-generalised tyvar, pinning it to the first use's argument type. SCC-by-SCC processing is the standard ML fix and was made explicit during Task 11 execution.

4. **No type defaulting.** If a top-level binding's type still contains a free variable after generalisation and the variable is not abstracted (because it appears in a position that escapes), that's an `AmbiguousType` error. `xs = []` at module scope is rejected unless annotated. The spec (`types.md § 2`) is explicit: the user writes the annotation, the checker doesn't guess.

5. **Operators are primitive in Plan 4.** `+ - * / ^` accept `Int×Int` or `Float×Float` (homogeneous); `== !=` accept any two values of the same type and return `Bool`; `< <= > >=` accept `Int×Int` or `Float×Float`; `and or xor` accept `Bool×Bool`; `++` accepts `String×String`. Unary `-` accepts `Int` or `Float`; `not` accepts `Bool`. This is a placeholder — Plan 5 replaces all of it with trait dispatch (`Add.add a, b` etc.). No mixed `Int`/`Float` arithmetic (`types.md § 1`).

6. **Effect rows in user-written types are rejected with a useful error.** Parsed `TypeKind::Function { effect: Some(...) }` produces `EffectsNotYetImplemented`. The `!` postfix expression on a call produces the same error. Plan 6 lifts this restriction. Until then, fixtures use pure types only.

7. **`?` operator deferred to Plan 6.** Typing `?` needs the enclosing function's return type and `Maybe`/`Result` awareness. Plan 4 rejects `Question(_)` with `EarlyExitNotYetImplemented`. Pure programs don't need it.

8. **Method-vs-field is resolved during type-check.** The resolver records `self.x` as `Local(self) . x` but doesn't decide whether `x` is a field or zero-arg method. The checker consults the receiver type's `TypeRegistry` entry: if `x` is a field, the access has the field's type; if `x` is a method, it has the method's scheme (instantiated). `MethodCall { receiver, method }` and `FieldAccess { receiver, field }` resolve through the same lookup path. Errors are `UnknownField { type_name, field }` (no such member) or `FieldVsMethodConflict` (ambiguity).

9. **No coercions.** `Int` and `Float` are distinct nominal types. `1 + 1.0` is a `TypeMismatch`, not silently coerced. To convert, the user writes `Std.Int.toFloat n` — but `Std` isn't in scope for Plan 4 corpus fixtures, so fixtures stay homogeneous.

10. **Exhaustiveness is coverage-based, not usefulness/redundancy.** For a `match` whose scrutinee is a sum type, the checker computes the set of variants covered by the arms and errors if any are missing. A wildcard arm (`_ -> ...`) covers everything. The checker does *not* report redundant arms or unreachable cases in Plan 4 — that's a future refinement. Nested constructor patterns recurse: `Some (Some x)` covers only that nested shape. For non-sum scrutinees (primitives), only a wildcard or variable pattern is exhaustive; literal patterns alone are non-exhaustive.

11. **Nominal types use `DefId`, primitives use a small enum.** `Ty::Con(DefId, Vec<Ty>)` for user-declared types and `Ty::Prim(PrimTy)` for `Int / Float / String / Bool / Unit`. Type variables are `Ty::Var(TyVarId)`. Functions are `Ty::Fun(Vec<Ty>, Box<Ty>)`. No effect-row slot in `Ty::Fun` — Plan 6 adds it.

---

## File structure

```
src/
  check/
    mod.rs            # pub fn check_file(&File, &Resolution) -> Result<Typing, Vec<Error>>
    types.rs          # Ty, TyVarId, Scheme, Subst, Typing, PrimTy
    registry.rs       # TypeRegistry: per-DefId type-decl info (fields, variants, methods)
    unify.rs          # apply_subst, occurs, unify
    infer.rs          # Infer context + infer_expr / infer_pattern / infer_block
    exhaust.rs        # exhaustiveness check for match arms
  lib.rs              # + pub mod check;

tests/
  check_literals.rs       # hand-written: literal / var / lambda / app
  check_bindings.rs       # hand-written: top-level mutual rec, generalisation, annotations
  check_records.rs        # hand-written: construction, update, field, method
  check_sums.rs           # hand-written: ctor app, match, exhaustiveness
  check_errors.rs         # hand-written: TypeMismatch, OccursCheck, UnknownField, ...
  check_corpus.rs         # insta::glob! over tests/corpus/check/*.i
  corpus/
    check/
      identity.i
      lambda-app.i
      mutual-rec.i
      record-build-update.i
      sum-match.i
      method-self.i
      newtype.i
      annotated-binding.i
      list-literal.i
```

**Why this split:**

- `check/types.rs` is the data model. Everything else depends on it.
- `check/registry.rs` is the type-decl side: walked once before expressions, gives `infer.rs` O(1) access to fields / variants / methods of any declared type.
- `check/unify.rs` is the algebra: substitution + unification. Pure functions on `Ty` and `Subst`; no `Infer` dependency.
- `check/infer.rs` is the recursion: the `Infer` struct, `infer_expr`, `infer_pattern`, `infer_block`. The big file by volume — but each function fits the screen.
- `check/exhaust.rs` is the match-coverage check. Separated from `infer.rs` because it's a different shape of recursion (over patterns and variant sets) and easier to test in isolation.

---

## Testing strategy

The three-layer strategy from `docs/testing.md` (Layer 1 corpus, Layer 2 hand-written, Layer 3 round-trip) is the contract. Plan 4 uses Layers 1 and 2 only — round-trip doesn't apply (typing isn't invertible).

- **Layer 1 — Insta corpus snapshots.** `tests/corpus/check/*.i` exercise one feature each. Snapshot format is a custom `Display` on `Typing` that prints each top-level scheme one-per-line: `<DefId> <name> : <pretty-Ty>`. The format is human-reviewable in `cargo insta review`. Exhaustiveness errors and other type errors get their own error-corpus fixtures (`.i` files paired with an expected error tag) — but for the first cut, the snapshot suite only covers successful typings.

- **Layer 2 — Hand-written assertions.** Per-form tests (`check_literals.rs`, `check_records.rs`, ...) use `assert_eq!` on inferred `Ty` values, and `matches!` on `Vec<Error>` for error tests. These pin down behaviour at finer grain than a snapshot diff.

Errors are returned as `Vec<Error>` — the checker doesn't stop at the first mismatch in independent top-level bindings. Within a single binding, the checker bails out of the offending sub-tree after recording the error and continues with a placeholder type (a fresh tyvar) so downstream code doesn't cascade.

---

## Decisions deferred to later plans

- **Trait declarations, impls, and operator desugaring** — Plan 5. Plan 4 hard-codes primitive operator types as a placeholder. `trait` and `impl` decls in user code are walked enough to register their names but their bodies are not type-checked.
- **Effect rows, `! IO / ! State / ! Env`, HOF effect-polymorphism, `! ()` empty-row constraint** — Plan 6. Plan 4's `Ty::Fun` carries no effect row; user-written `! Eff` in a type errors out.
- **`?` early-return operator** — Plan 6, alongside effects (since `?` interacts with `Result` and the enclosing function's return type).
- **`!` bang expression** — Plan 6.
- **Totality / termination checking** — Plan 7. Exhaustiveness checking lives here in Plan 4.
- **Stdlib types in scope by default** (`List`, `Maybe`, `Result`, `Std.IO.print`, ...) — Plan 6 ships the stdlib, Plan 7 wires it in. Plan 4 fixtures stub whatever they need (e.g. `type List a` for list literals, `type Maybe a` for `?` follow-ups).
- **Cross-module type-checking** — Plan 7. Plan 4 is single-file: imported names typed via `Imported { module, name }` are rejected with `CrossModuleTypingNotYetImplemented`.
- **Higher-kinded types, type families, dependent types** — out of v1 entirely.

---

## Task 1: Type-checker scaffold and `Ty` representation

**Files:**
- Create: `src/check/mod.rs`
- Create: `src/check/types.rs`
- Modify: `src/lib.rs` (add `pub mod check;`)
- Test: `tests/check_literals.rs`

- [ ] **Step 1: Write the failing test**

`tests/check_literals.rs`:

```rust
use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn empty_top_level_binding_returns_empty_typing() {
    let src = "x = 1\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected check to succeed");
    assert!(typing.expr_types.is_empty());
    assert!(typing.schemes.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals -- empty_top_level_binding_returns_empty_typing`
Expected: FAIL — `check` module / `check_file` function / `Typing` struct do not exist.

- [ ] **Step 3: Write minimal implementation**

`src/lib.rs` — add `pub mod check;`.

`src/check/types.rs`:

```rust
use crate::resolve::DefId;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVarId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimTy {
    Int,
    Float,
    String,
    Bool,
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Var(TyVarId),
    Prim(PrimTy),
    Con(DefId, Vec<Ty>),
    Fun(Vec<Ty>, Box<Ty>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TyVarId>,
    pub ty: Ty,
}

pub type Subst = HashMap<TyVarId, Ty>;

#[derive(Debug, Default, Clone)]
pub struct Typing {
    pub schemes: HashMap<DefId, Scheme>,
    pub expr_types: HashMap<Span, Ty>,
    pub pattern_types: HashMap<Span, Ty>,
}
```

`src/check/mod.rs`:

```rust
mod types;

pub use types::*;

use crate::ast::File;
use crate::error::Error;
use crate::resolve::Resolution;

pub fn check_file(_file: &File, _res: &Resolution) -> Result<Typing, Vec<Error>> {
    Ok(Typing::default())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_literals -- empty_top_level_binding_returns_empty_typing`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/lib.rs tests/check_literals.rs
git commit -m "Plan 4 Task 1: scaffold type checker and Ty representation"
```

---

## Task 2: Substitution and occurs check

**Files:**
- Create: `src/check/unify.rs`
- Modify: `src/check/mod.rs` (add `mod unify;`)
- Test: inline `#[cfg(test)]` in `src/check/unify.rs`

- [ ] **Step 1: Write the failing test**

`src/check/unify.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::types::{PrimTy, Ty, TyVarId};
    use std::collections::HashMap;

    #[test]
    fn apply_subst_replaces_bound_var() {
        let mut s: crate::check::types::Subst = HashMap::new();
        s.insert(TyVarId(0), Ty::Prim(PrimTy::Int));
        let ty = Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Var(TyVarId(1))));
        let out = apply_subst(&ty, &s);
        assert_eq!(
            out,
            Ty::Fun(vec![Ty::Prim(PrimTy::Int)], Box::new(Ty::Var(TyVarId(1))))
        );
    }

    #[test]
    fn occurs_finds_var_inside_fun() {
        let ty = Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Int)));
        assert!(occurs(TyVarId(0), &ty, &HashMap::new()));
        assert!(!occurs(TyVarId(1), &ty, &HashMap::new()));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib check::unify`
Expected: FAIL — `apply_subst` / `occurs` do not exist.

- [ ] **Step 3: Write minimal implementation**

`src/check/unify.rs`:

```rust
use crate::check::types::{Subst, Ty, TyVarId};

pub fn apply_subst(ty: &Ty, subst: &Subst) -> Ty {
    match ty {
        Ty::Var(v) => match subst.get(v) {
            Some(t) => apply_subst(t, subst),
            None => Ty::Var(*v),
        },
        Ty::Prim(p) => Ty::Prim(*p),
        Ty::Con(id, args) => {
            Ty::Con(*id, args.iter().map(|a| apply_subst(a, subst)).collect())
        }
        Ty::Fun(params, result) => Ty::Fun(
            params.iter().map(|p| apply_subst(p, subst)).collect(),
            Box::new(apply_subst(result, subst)),
        ),
    }
}

pub fn occurs(var: TyVarId, ty: &Ty, subst: &Subst) -> bool {
    match apply_subst(ty, subst) {
        Ty::Var(v) => v == var,
        Ty::Prim(_) => false,
        Ty::Con(_, args) => args.iter().any(|a| occurs(var, a, subst)),
        Ty::Fun(ps, r) => ps.iter().any(|p| occurs(var, p, subst)) || occurs(var, &r, subst),
    }
}
```

Wire `mod unify;` in `src/check/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib check::unify`
Expected: PASS — both tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/
git commit -m "Plan 4 Task 2: substitution and occurs check"
```

---

## Task 3: Unification

**Files:**
- Modify: `src/check/unify.rs` (add `unify` and `UnifyError`)
- Modify: `src/error.rs` (add `TypeMismatch`, `OccursCheck`, `ArityMismatch`)

- [ ] **Step 1: Write the failing test**

Add to `src/check/unify.rs`'s `#[cfg(test)] mod tests`:

```rust
#[test]
fn unify_primitive_succeeds_when_equal() {
    let mut s: crate::check::types::Subst = HashMap::new();
    unify(&mut s, &Ty::Prim(PrimTy::Int), &Ty::Prim(PrimTy::Int)).unwrap();
    assert!(s.is_empty());
}

#[test]
fn unify_primitive_fails_when_distinct() {
    let mut s: crate::check::types::Subst = HashMap::new();
    let r = unify(&mut s, &Ty::Prim(PrimTy::Int), &Ty::Prim(PrimTy::Float));
    assert!(matches!(r, Err(UnifyError::Mismatch { .. })));
}

#[test]
fn unify_var_binds_when_unbound() {
    let mut s: crate::check::types::Subst = HashMap::new();
    unify(&mut s, &Ty::Var(TyVarId(0)), &Ty::Prim(PrimTy::Int)).unwrap();
    assert_eq!(s.get(&TyVarId(0)), Some(&Ty::Prim(PrimTy::Int)));
}

#[test]
fn unify_var_with_self_containing_term_is_occurs_check() {
    let mut s: crate::check::types::Subst = HashMap::new();
    let lhs = Ty::Var(TyVarId(0));
    let rhs = Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Int)));
    let r = unify(&mut s, &lhs, &rhs);
    assert!(matches!(r, Err(UnifyError::Occurs(_))));
}

#[test]
fn unify_fun_compares_arities() {
    let mut s: crate::check::types::Subst = HashMap::new();
    let one = Ty::Fun(vec![Ty::Prim(PrimTy::Int)], Box::new(Ty::Prim(PrimTy::Int)));
    let two = Ty::Fun(
        vec![Ty::Prim(PrimTy::Int), Ty::Prim(PrimTy::Int)],
        Box::new(Ty::Prim(PrimTy::Int)),
    );
    assert!(matches!(
        unify(&mut s, &one, &two),
        Err(UnifyError::Arity { .. })
    ));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib check::unify::tests::unify_`
Expected: FAIL — `unify` and `UnifyError` do not exist.

- [ ] **Step 3: Write minimal implementation**

Add to `src/check/unify.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UnifyError {
    Mismatch { left: Ty, right: Ty },
    Occurs(TyVarId),
    Arity { expected: usize, found: usize },
}

pub fn unify(subst: &mut Subst, a: &Ty, b: &Ty) -> Result<(), UnifyError> {
    let a = apply_subst(a, subst);
    let b = apply_subst(b, subst);
    match (a, b) {
        (Ty::Var(x), Ty::Var(y)) if x == y => Ok(()),
        (Ty::Var(v), t) | (t, Ty::Var(v)) => {
            if occurs(v, &t, subst) {
                Err(UnifyError::Occurs(v))
            } else {
                subst.insert(v, t);
                Ok(())
            }
        }
        (Ty::Prim(p), Ty::Prim(q)) if p == q => Ok(()),
        (Ty::Con(id1, args1), Ty::Con(id2, args2)) if id1 == id2 => {
            if args1.len() != args2.len() {
                return Err(UnifyError::Arity {
                    expected: args1.len(),
                    found: args2.len(),
                });
            }
            for (x, y) in args1.iter().zip(args2.iter()) {
                unify(subst, x, y)?;
            }
            Ok(())
        }
        (Ty::Fun(p1, r1), Ty::Fun(p2, r2)) => {
            if p1.len() != p2.len() {
                return Err(UnifyError::Arity {
                    expected: p1.len(),
                    found: p2.len(),
                });
            }
            for (x, y) in p1.iter().zip(p2.iter()) {
                unify(subst, x, y)?;
            }
            unify(subst, &r1, &r2)
        }
        (left, right) => Err(UnifyError::Mismatch { left, right }),
    }
}
```

Add to `src/error.rs`'s `ErrorKind`:

```rust
TypeMismatch { expected: String, found: String },
OccursCheck { var: String },
ArityMismatch { expected: usize, found: usize },
```

(Pretty-printing the `Ty` values into strings is a later concern; for now `format!("{:?}", ty)` is fine. A real `Display for Ty` lands in Task 24's corpus/error-formatting cleanup.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib check::unify::tests::unify_`
Expected: PASS — five new tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/unify.rs src/error.rs
git commit -m "Plan 4 Task 3: unification with occurs and arity checks"
```

---

## Task 4: Inference context and fresh tyvars

**Files:**
- Create: `src/check/infer.rs` (Infer struct skeleton)
- Modify: `src/check/mod.rs` (add `mod infer;`)
- Test: inline `#[cfg(test)] mod tests` in `src/check/infer.rs`

- [ ] **Step 1: Write the failing test**

`src/check/infer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_returns_distinct_ids() {
        let mut infer = Infer::new();
        let a = infer.fresh();
        let b = infer.fresh();
        assert_ne!(a, b);
    }

    #[test]
    fn record_expr_type_stores_and_applies_subst() {
        use crate::check::types::{PrimTy, Ty, TyVarId};
        use crate::span::Span;

        let mut infer = Infer::new();
        let v = infer.fresh();
        infer.subst.insert(v, Ty::Prim(PrimTy::Int));
        let s = Span::new(0, 1);
        infer.record_expr_type(s, Ty::Var(v));
        let typing = infer.into_typing();
        assert_eq!(typing.expr_types.get(&s), Some(&Ty::Prim(PrimTy::Int)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib check::infer`
Expected: FAIL — `Infer` does not exist.

- [ ] **Step 3: Write minimal implementation**

`src/check/infer.rs`:

```rust
use crate::check::types::{Subst, Ty, TyVarId, Typing};
use crate::check::unify::apply_subst;
use crate::error::Error;
use crate::resolve::{DefId, LocalId};
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Infer {
    pub subst: Subst,
    pub locals: HashMap<LocalId, Ty>,
    pub schemes: HashMap<DefId, crate::check::types::Scheme>,
    pub errors: Vec<Error>,
    next_var: u32,
    expr_types: HashMap<Span, Ty>,
    pattern_types: HashMap<Span, Ty>,
}

impl Infer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fresh(&mut self) -> TyVarId {
        let id = TyVarId(self.next_var);
        self.next_var += 1;
        id
    }

    pub fn record_expr_type(&mut self, span: Span, ty: Ty) {
        self.expr_types.insert(span, ty);
    }

    pub fn record_pattern_type(&mut self, span: Span, ty: Ty) {
        self.pattern_types.insert(span, ty);
    }

    pub fn into_typing(self) -> Typing {
        let resolve = |m: HashMap<Span, Ty>| -> HashMap<Span, Ty> {
            m.into_iter()
                .map(|(s, t)| (s, apply_subst(&t, &self.subst)))
                .collect()
        };
        Typing {
            schemes: self.schemes,
            expr_types: resolve(self.expr_types),
            pattern_types: resolve(self.pattern_types),
        }
    }
}
```

Wait — Rust will complain about moving `self.subst` while still calling `resolve` after. Rewrite the closure-free version:

```rust
pub fn into_typing(self) -> Typing {
    let Infer { subst, schemes, expr_types, pattern_types, .. } = self;
    let expr_types = expr_types
        .into_iter()
        .map(|(s, t)| (s, apply_subst(&t, &subst)))
        .collect();
    let pattern_types = pattern_types
        .into_iter()
        .map(|(s, t)| (s, apply_subst(&t, &subst)))
        .collect();
    Typing { schemes, expr_types, pattern_types }
}
```

Wire `mod infer;` in `src/check/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib check::infer`
Expected: PASS — both tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/
git commit -m "Plan 4 Task 4: inference context and fresh tyvars"
```

---

## Task 5: Literal inference

**Files:**
- Modify: `src/check/infer.rs` (add `infer_expr` for literals)
- Modify: `src/check/mod.rs` (wire `check_file` to walk top-level bindings, infer literal RHSs)
- Test: `tests/check_literals.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/check_literals.rs`:

```rust
use i_lang::check::types::{PrimTy, Ty};

fn check_value(src: &str, name: &str) -> Ty {
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let def = res.defs.iter().find(|d| d.name == name).unwrap();
    typing.schemes[&def.id].ty.clone()
}

#[test]
fn int_literal_has_type_int() {
    assert_eq!(check_value("x = 1\n", "x"), Ty::Prim(PrimTy::Int));
}

#[test]
fn float_literal_has_type_float() {
    assert_eq!(check_value("x = 1.5\n", "x"), Ty::Prim(PrimTy::Float));
}

#[test]
fn string_literal_has_type_string() {
    assert_eq!(check_value("x = \"hi\"\n", "x"), Ty::Prim(PrimTy::String));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals`
Expected: FAIL — `schemes[&def.id]` panics (the map is empty).

- [ ] **Step 3: Write minimal implementation**

In `src/check/infer.rs`, add:

```rust
use crate::ast::{Expr, ExprKind};

impl Infer {
    pub fn infer_expr(&mut self, e: &Expr) -> Ty {
        let ty = match &e.node {
            ExprKind::IntLit(_) => Ty::Prim(crate::check::types::PrimTy::Int),
            ExprKind::FloatLit(_) => Ty::Prim(crate::check::types::PrimTy::Float),
            ExprKind::StringLit(_) => Ty::Prim(crate::check::types::PrimTy::String),
            _ => {
                let v = self.fresh();
                Ty::Var(v)
            }
        };
        self.record_expr_type(e.span, ty.clone());
        ty
    }
}
```

In `src/check/mod.rs`, replace `check_file`:

```rust
use crate::ast::DeclKind;
use crate::check::infer::Infer;
use crate::check::types::Scheme;

pub fn check_file(file: &File, res: &Resolution) -> Result<Typing, Vec<Error>> {
    let mut infer = Infer::new();
    for decl in &file.decls {
        if let DeclKind::Binding { name, value: Some(value), .. } = &decl.node {
            let def = res.defs.iter().find(|d| &d.name == name);
            let Some(def) = def else { continue; };
            let ty = infer.infer_expr(value);
            infer.schemes.insert(def.id, Scheme { vars: Vec::new(), ty });
        }
    }
    if infer.errors.is_empty() {
        Ok(infer.into_typing())
    } else {
        let errs = std::mem::take(&mut infer.errors);
        Err(errs)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_literals`
Expected: PASS — three new tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_literals.rs
git commit -m "Plan 4 Task 5: literal inference"
```

---

## Task 6: Top-level signatures and mutual recursion

**Files:**
- Modify: `src/check/mod.rs` (two-phase top-level: pre-declare → infer → generalise)
- Test: `tests/check_bindings.rs`

- [ ] **Step 1: Write the failing test**

`tests/check_bindings.rs`:

```rust
use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn mutual_top_level_recursion_typechecks() {
    // a refers to b, b refers to a — both should resolve to Int.
    let src = "\
a = b
b = 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected mutual-rec to type-check");
    let a = res.defs.iter().find(|d| d.name == "a").unwrap();
    let b = res.defs.iter().find(|d| d.name == "b").unwrap();
    assert_eq!(typing.schemes[&a.id].ty, Ty::Prim(PrimTy::Int));
    assert_eq!(typing.schemes[&b.id].ty, Ty::Prim(PrimTy::Int));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_bindings -- mutual_top_level_recursion_typechecks`
Expected: FAIL — `a = b` raises an `Unresolved`-style panic or the type of `a` is a free `Var`, not `Int`.

- [ ] **Step 3: Write minimal implementation**

Restructure `check_file` into two phases:

```rust
pub fn check_file(file: &File, res: &Resolution) -> Result<Typing, Vec<Error>> {
    let mut infer = Infer::new();

    // Phase 1: pre-declare fresh tyvars for every top-level binding.
    let mut pending: Vec<(crate::resolve::DefId, &Expr, TyVarId)> = Vec::new();
    for decl in &file.decls {
        if let DeclKind::Binding { name, value: Some(value), .. } = &decl.node {
            if let Some(def) = res.defs.iter().find(|d| &d.name == name) {
                let v = infer.fresh();
                infer.schemes.insert(
                    def.id,
                    crate::check::types::Scheme {
                        vars: Vec::new(),
                        ty: Ty::Var(v),
                    },
                );
                pending.push((def.id, value, v));
            }
        }
    }

    // Phase 2: infer each body and unify with its pre-declared tyvar.
    for (_def_id, value, v) in &pending {
        let ty = infer.infer_expr(value);
        if let Err(e) = crate::check::unify::unify(&mut infer.subst, &Ty::Var(*v), &ty) {
            infer.errors.push(unify_error_to_error(value.span, e));
        }
    }
    // Phase 2.5: resolve each scheme through the *final* substitution. Done
    // outside the phase-2 loop because mutual recursion (e.g. `a = b; b = 1`)
    // only pins later bindings' tyvars after their own iteration completes;
    // resolving inside the loop would freeze `a` as `Var(α_b)` before
    // `α_b → Int` is added on the next iteration.
    for (def_id, _value, v) in pending {
        let resolved = crate::check::unify::apply_subst(&Ty::Var(v), &infer.subst);
        infer.schemes.insert(
            def_id,
            crate::check::types::Scheme { vars: Vec::new(), ty: resolved },
        );
    }

    if infer.errors.is_empty() {
        Ok(infer.into_typing())
    } else {
        Err(std::mem::take(&mut infer.errors))
    }
}

fn unify_error_to_error(span: crate::span::Span, e: crate::check::unify::UnifyError) -> Error {
    use crate::check::unify::UnifyError;
    use crate::error::ErrorKind;
    let kind = match e {
        UnifyError::Mismatch { left, right } => ErrorKind::TypeMismatch {
            expected: format!("{:?}", left),
            found: format!("{:?}", right),
        },
        UnifyError::Occurs(v) => ErrorKind::OccursCheck { var: format!("{:?}", v) },
        UnifyError::Arity { expected, found } => {
            ErrorKind::ArityMismatch { expected, found }
        }
    };
    Error { span, kind }
}
```

And add a `Var` arm to `infer_expr` that consults the resolver:

```rust
ExprKind::Var(_) => match res_lookup(&self.res, &e.span) {
    Some(crate::resolve::ResolvedName::TopLevel(def_id)) => {
        match self.schemes.get(&def_id) {
            Some(scheme) => self.instantiate(scheme.clone()),
            None => {
                let v = self.fresh();
                Ty::Var(v)
            }
        }
    }
    _ => {
        let v = self.fresh();
        Ty::Var(v)
    }
},
```

…which requires `Infer` to hold a reference to `&Resolution`. Refactor `Infer::new` to take it:

```rust
pub struct Infer<'a> {
    pub res: &'a Resolution,
    pub subst: Subst,
    // ...
}

impl<'a> Infer<'a> {
    pub fn new(res: &'a Resolution) -> Self { /* default fields, res stored */ }
}
```

Update `check_file` and Task 4/5's tests to pass a resolution. Replace `Infer::default()` calls with `Infer::new(&res)` (use a tiny stub resolution for Task 4's unit tests — `Resolution::default()` already works).

Add an `instantiate(scheme: Scheme) -> Ty` method on `Infer`:

```rust
pub fn instantiate(&mut self, scheme: Scheme) -> Ty {
    let mut s: Subst = HashMap::new();
    for v in &scheme.vars {
        let fresh = self.fresh();
        s.insert(*v, Ty::Var(fresh));
    }
    apply_subst(&scheme.ty, &s)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_bindings -- mutual_top_level_recursion_typechecks` then `cargo test`.
Expected: PASS — mutual recursion resolves correctly, and earlier literal tests still green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_bindings.rs
git commit -m "Plan 4 Task 6: top-level signatures and mutual recursion"
```

---

## Task 7: Variable lookup with instantiation

**Files:**
- Modify: `src/check/infer.rs` (Var arm, Ctor arm — instantiation paths)
- Test: `tests/check_bindings.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_bindings.rs`:

```rust
#[test]
fn alias_binding_takes_referent_type() {
    let src = "\
n = 42
m = n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let m = res.defs.iter().find(|d| d.name == "m").unwrap();
    assert_eq!(typing.schemes[&m.id].ty, Ty::Prim(PrimTy::Int));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_bindings -- alias_binding_takes_referent_type`
Expected: this likely passes already from Task 6 if `Var` looks up correctly — but if `infer_expr`'s `Var` arm is still the default fresh-tyvar fallback, the assertion fails with a `Var(_)` type.

If it already passes, *change* the test to a stronger one that requires instantiation:

```rust
#[test]
fn polymorphic_top_level_instantiates_at_each_use() {
    // After Task 11 generalisation, identity is forall a . a -> a.
    // Two uses at different types must not collide.
    // (For Task 7 we just check that lookup works for a Local — generalisation
    // arrives in Task 11. So pin a weaker variant here.)
    let src = "\
n = 42
m = n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let m = res.defs.iter().find(|d| d.name == "m").unwrap();
    assert_eq!(typing.schemes[&m.id].ty, Ty::Prim(PrimTy::Int));
}
```

- [ ] **Step 3: Write minimal implementation**

Confirm the `Var` arm written in Task 6 looks up `ResolvedName::TopLevel`, instantiates the (currently un-generalised) scheme, and returns the result. Also handle `ResolvedName::Local`:

```rust
Some(crate::resolve::ResolvedName::Local(local_id)) => {
    self.locals
        .get(&local_id)
        .cloned()
        .unwrap_or_else(|| Ty::Var(self.fresh()))
}
```

Add a stub `Ctor` arm that records a fresh tyvar (filled in by Task 16):

```rust
ExprKind::Ctor(_) => Ty::Var(self.fresh()),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS — new test plus all existing tests.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_bindings.rs
git commit -m "Plan 4 Task 7: variable lookup with instantiation"
```

---

## Task 8: Lambda typing

**Files:**
- Modify: `src/check/infer.rs` (Lambda arm)
- Modify: `src/resolve/walker.rs` (record binding-pattern span -> `Local` in `refs`)
- Modify: affected resolver tests (Local ref counts bump by the number of pattern bindings)
- Test: `tests/check_literals.rs` (add)

The resolver currently records `Var` *use* spans in `refs` but not pattern *binding* spans. The checker needs both — given a pattern Var node, it must recover the `LocalId` to know which tyvar to attach to the bound name. Extending `bind_pattern` to insert `p.span -> ResolvedName::Local(id)` makes `refs` complete (every binding site and every use site has an entry) and matches the resolver's symmetry. The cost is bumping Local-ref counts in three existing resolver tests by a small known delta.

- [ ] **Step 1: Write the failing test**

Add to `tests/check_literals.rs`:

```rust
#[test]
fn identity_lambda_has_fun_type() {
    // x -> x  has type a -> a; pre-generalisation it's Var(α) -> Var(α).
    let src = "id = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let id = res.defs.iter().find(|d| d.name == "id").unwrap();
    match &typing.schemes[&id.id].ty {
        Ty::Fun(params, result) => {
            assert_eq!(params.len(), 1);
            assert_eq!(&params[0], result.as_ref()); // same Var on both sides
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals -- identity_lambda_has_fun_type`
Expected: FAIL — the Lambda arm is still the default fresh-tyvar fallback, so the type is a bare `Var`.

- [ ] **Step 3: Write minimal implementation**

Add to `infer_expr`:

```rust
ExprKind::Lambda { params, body } => {
    let mut param_tys = Vec::with_capacity(params.len());
    let mut bound: Vec<crate::resolve::LocalId> = Vec::new();
    for p in params {
        let pt = self.infer_pattern(p);
        param_tys.push(pt.ty);
        bound.extend(pt.bindings);
    }
    let result_ty = self.infer_expr(body);
    // Note: we leave locals in `self.locals` for the rest of this binding.
    // The two-phase top-level loop in check_file gives us a fresh Infer per
    // file (one big context). Cross-binding leakage is harmless because each
    // LocalId is unique across the file (resolver allocates them globally).
    let _ = bound;
    Ty::Fun(param_tys, Box::new(result_ty))
}
```

Pattern inference doesn't exist yet — for Task 8 add a minimal `infer_pattern` that handles only `PatternKind::Var`:

```rust
pub struct PatternResult {
    pub ty: Ty,
    pub bindings: Vec<crate::resolve::LocalId>,
}

pub fn infer_pattern(&mut self, p: &crate::ast::Pattern) -> PatternResult {
    use crate::ast::PatternKind;
    let v = self.fresh();
    let ty = Ty::Var(v);
    self.record_pattern_type(p.span, ty.clone());
    match &p.node {
        PatternKind::Var(_) => {
            if let Some(crate::resolve::ResolvedName::Local(lid)) = self.res.refs.get(&p.span) {
                self.locals.insert(*lid, ty.clone());
                return PatternResult { ty, bindings: vec![*lid] };
            }
            PatternResult { ty, bindings: vec![] }
        }
        _ => PatternResult { ty, bindings: vec![] }, // placeholders, Tasks 17–19 fill in
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_literals -- identity_lambda_has_fun_type`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_literals.rs
git commit -m "Plan 4 Task 8: lambda typing"
```

---

## Task 9: Function application

**Files:**
- Modify: `src/check/infer.rs` (Call arm)
- Test: `tests/check_literals.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_literals.rs`:

```rust
#[test]
fn applied_identity_has_arg_type() {
    let src = "\
id = x -> x
n = id 42
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn arity_mismatch_in_call_reports_error() {
    // `id` takes one arg; pass two.
    let src = "\
id = x -> x
n = id 1 2
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::ArityMismatch { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals -- applied_identity_has_arg_type arity_mismatch_in_call_reports_error`
Expected: FAIL — `Call` arm is the default fresh fallback.

- [ ] **Step 3: Write minimal implementation**

Add the `Call` arm:

```rust
ExprKind::Call { func, args } => {
    let fn_ty = self.infer_expr(func);
    let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();
    let result_v = self.fresh();
    let expected = Ty::Fun(arg_tys, Box::new(Ty::Var(result_v)));
    if let Err(e) = crate::check::unify::unify(&mut self.subst, &fn_ty, &expected) {
        self.errors.push(unify_error_to_error(e.clone(), func.span, &e));
    }
    Ty::Var(result_v)
}
```

Move `unify_error_to_error` into `infer.rs` (or expose it). The errors need to surface the span of the offending sub-tree — for now, `func.span`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS — both new tests and all earlier tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_literals.rs
git commit -m "Plan 4 Task 9: function application"
```

---

## Task 9.5: Correct the precedence-table claim about paren-free call associativity

**Files:**
- Modify: `docs/syntax.md` (precedence table row 11 + a short clarifier in §5)
- Modify: `tests/check_literals.rs` (rewrite the comment on `arity_mismatch_in_call_reports_error` to drop the "gotcha" framing)

**Why this task exists (mid-execution amendment to Plan 4):** Task 9 surfaced what looked like a parser quirk — `id 1 2` parses as `id (1 2)`. The first proposed fix (reject juxtaposed args at parse time) broke the spec's own method-chaining example at `syntax.md:347-349`, which deliberately uses right-associative juxtaposition: `nums.map double.filter pred` parses as `nums.map (double.filter pred)`. That behaviour is pinned by `tests/parser_calls.rs::method_chain_atom_only`. The spec is internally consistent: `.` binds to the immediately preceding atom (§5), juxtaposition then groups right so chains compose without nesting parens. The "no currying" rule (§2) is about *lambda parameters*, not call shape.

**The actual bug** is the precedence table at `syntax.md:814` claiming `function call (juxtaposition)` is **left-associative**. It's right-associative, and that's a deliberate feature. The misleading row is what made `id 1 2` look like a parser bug instead of a documented consequence.

**Decision:** Fix the precedence table to match the spec's actual model and add a short pointer in §5 to the method-chaining section. No parser change; no new behaviour. The Task 9 test's `id 1, 2` (comma-separated) form stays — the comment just stops framing it as a "gotcha."

- [ ] **Step 1: Verify the baseline is green**

Run: `cargo test`
Expected: PASS — Task 9.5 is a docs-only change, so the existing suite is the verification. `tests/parser_calls.rs::method_chain_atom_only` is the load-bearing assertion that pins right-associative juxtaposition; it must stay green.

- [ ] **Step 2: Edit `docs/syntax.md` — precedence table row 11**

Currently:

```
| 11   | function call (juxtaposition)     | left            |
```

Change to:

```
| 11   | function call (juxtaposition)     | right           |
```

- [ ] **Step 3: Edit `docs/syntax.md` — §5 clarification**

In the "Nested call" subsection, after the `add 3, (mul 4, 5)` example, add a short sentence explaining that two juxtaposed call expressions group right-associatively, and point readers at the "Method chaining" subsection as the canonical case where this matters (`nums.map double.filter pred` → `nums.map (double.filter pred)`).

- [ ] **Step 4: Edit `tests/check_literals.rs`**

The comment on `arity_mismatch_in_call_reports_error` currently says:

```
// `id` takes one arg; pass two (comma-separated — `id 1 2` parses as
// `id (1 2)` and produces a TypeMismatch from calling Int as a function).
```

Rewrite to drop the parenthetical and the "gotcha" framing — the test passes a 2-arg call (`id 1, 2`) to a 1-arg function; say that plainly.

- [ ] **Step 5: Run tests and commit**

Run: `cargo test` — expected PASS (no behavioural change).

```bash
git add docs/syntax.md tests/check_literals.rs docs/superpowers/plans/2026-05-22-plan-4-type-checker.md
git commit -m "Plan 4 Task 9.5: correct precedence-table call associativity"
```

Fold the plan-amendment diff into the same commit — the plan revision and the spec change are the same task.

---

## Task 10: Blocks and sequential let-bindings

**Files:**
- Modify: `src/check/infer.rs` (Block arm)
- Modify: `src/resolve/walker.rs` (record block-binding `LocalId` in `refs` at `decl.span`, mirroring the Task 8 pattern-binding tweak)
- Modify: `tests/resolver_locals.rs` (`block_let_binding_visible_later`: ref count 4 → 6 to account for the new binding-site entries)
- Test: `tests/check_bindings.rs` (add)

> **Amendment during execution:** The plan originally wrote the failing-test source as `f = -> ...` assuming a zero-arg lambda syntax — but the spec (`syntax.md:253-256`) only admits zero-arg functions as *effectful procedures* (`: ! Eff -> result`). The pure-block form is `name = <indented block>` — no lambda arrow. Tests rewritten to that form; assertions on `f`'s scheme are now a direct `Int` comparison rather than a `Fun` match. Annotation handling (`lower_type`) deferred to Task 11 where it actually arrives.

- [ ] **Step 1: Write the failing test**

Add to `tests/check_bindings.rs`:

```rust
#[test]
fn block_local_takes_inferred_type() {
    let src = "\
f =
    n = 42
    n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let f = res.defs.iter().find(|d| d.name == "f").unwrap();
    assert_eq!(typing.schemes[&f.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn block_local_is_monomorphic() {
    // id bound inside a block is monomorphic — its tyvar fixes after the first
    // call, so a second call with a different arg type errors.
    let src = "\
result =
    id = x -> x
    n = id 1
    s = id \"hi\"
    n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_bindings -- block_`
Expected: FAIL — Block isn't handled.

- [ ] **Step 3: Write minimal implementation**

First, the resolver tweak — `src/resolve/walker.rs::walk_block` currently allocates a `LocalId` via `push_local` but does **not** record it in `self.res.refs`. The checker needs that mapping. Match the Task 8 pattern-binding tweak: on `Ok(id)`, insert `decl.span -> ResolvedName::Local(id)`; on `Err(())`, keep the existing `DuplicateLocal` error path.

Then add the Block arm in `infer_expr`:

```rust
ExprKind::Block(items) => {
    use crate::ast::{BlockItem, DeclKind};
    let mut last_ty = Ty::Prim(PrimTy::Unit);
    for item in items {
        match item {
            BlockItem::Binding(decl) => {
                if let DeclKind::Binding { value: Some(value), .. } = &decl.node {
                    let value_ty = self.infer_expr(value);
                    if let Some(ResolvedName::Local(lid)) = self.res.refs.get(&decl.span) {
                        self.locals.insert(*lid, value_ty);
                    }
                }
                last_ty = Ty::Prim(PrimTy::Unit);
            }
            BlockItem::Expr(expr) => {
                last_ty = self.infer_expr(expr);
            }
        }
    }
    last_ty
}
```

Type annotations on block-let bindings are ignored here — annotation handling (`lower_type` + unify) lands in Task 11 and applies uniformly to top-level and block bindings.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS — `block_local_takes_inferred_type` and `block_local_is_monomorphic` green; rest still pass.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/resolve/ tests/check_bindings.rs tests/resolver_locals.rs \
        docs/superpowers/plans/2026-05-22-plan-4-type-checker.md
git commit -m "Plan 4 Task 10: blocks and sequential let-bindings"
```

Fold the plan-amendment diff (test source rewrite, resolver-tweak note) into the same commit.

---

## Task 11: Generalisation, SCC-based top-level inference, and type annotations

**Files:**
- Modify: `src/check/mod.rs` (replace single-phase-2 with SCC-by-SCC processing + generalisation; merge sig-only and value-only bindings by name for multi-line annotations)
- Modify: `src/check/infer.rs` (add `lower_type` for annotation parsing; add `free_vars` and `generalise` helpers)
- Modify: `src/error.rs` (add `EffectsNotYetImplemented`, `TuplesNotYetImplemented`)
- Test: `tests/check_bindings.rs` (add)

**Amendment notice (scope expanded during execution):** The original Task 11 plan assumed simple "phase 2 over the whole file, phase 3 generalises at the end" — but as traced during Task 11 execution, that architecture fixes `id`'s tyvar to the type of its first use, so `n = id 1; s = id "hi"` fails. Bundling SCC-based top-level inference in here (rather than a follow-up Task 11.5) lands the polymorphism story in one commit. Multi-line annotations (`double : Int -> Int` on one line, `double = n -> n` on the next — two separate decls per the parser) get the same merge-by-name treatment as inline annotations, since both forms are spec-canonical.

- [ ] **Step 1: Write the failing test**

Add to `tests/check_bindings.rs`:

```rust
#[test]
fn identity_is_generalised_to_forall() {
    let src = "id = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let id = res.defs.iter().find(|d| d.name == "id").unwrap();
    let scheme = &typing.schemes[&id.id];
    assert_eq!(scheme.vars.len(), 1, "expected one quantified var, got {:?}", scheme);
    match &scheme.ty {
        Ty::Fun(params, result) => {
            assert_eq!(params.len(), 1);
            assert_eq!(&params[0], result.as_ref());
            assert!(matches!(params[0], Ty::Var(_)));
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}

#[test]
fn polymorphic_top_level_instantiates_at_each_use() {
    let src = "\
id = x -> x
n = id 1
s = id \"hi\"
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    let s = res.defs.iter().find(|d| d.name == "s").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
    assert_eq!(typing.schemes[&s.id].ty, Ty::Prim(PrimTy::String));
}

#[test]
fn annotation_pins_inferred_type() {
    let src = "\
double : Int -> Int
double = n -> n
";
    // This relies on type-decl-style annotations parsing into a Binding with
    // ty: Some(...). If the parser puts the signature in a separate slot,
    // adjust this test accordingly when running it for the first time.
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let d = res.defs.iter().find(|d| d.name == "double").unwrap();
    let expected = Ty::Fun(
        vec![Ty::Prim(PrimTy::Int)],
        Box::new(Ty::Prim(PrimTy::Int)),
    );
    assert_eq!(typing.schemes[&d.id].ty, expected);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_bindings -- identity_is_generalised polymorphic_top_level annotation_pins`
Expected: FAIL — generalisation hasn't been wired; `id`'s scheme has empty `vars`.

- [ ] **Step 3: Write minimal implementation**

After the phase-2 inference loop in `check_file`, walk every scheme and generalise its free tyvars that aren't pinned anywhere else in the env. Compute `free_vars(env)` as the union of free-vars across every other scheme (after substitution). For a freshly typed `id`, no other top-level binding constrains its tyvar, so it gets quantified:

```rust
fn free_vars(ty: &Ty) -> std::collections::HashSet<TyVarId> {
    let mut out = std::collections::HashSet::new();
    fn walk(ty: &Ty, out: &mut std::collections::HashSet<TyVarId>) {
        match ty {
            Ty::Var(v) => { out.insert(*v); }
            Ty::Prim(_) => {}
            Ty::Con(_, args) => args.iter().for_each(|a| walk(a, out)),
            Ty::Fun(ps, r) => { ps.iter().for_each(|p| walk(p, out)); walk(r, out); }
        }
    }
    walk(ty, &mut out);
    out
}

fn generalise(ty: Ty, env_free: &std::collections::HashSet<TyVarId>) -> Scheme {
    let mine: std::collections::HashSet<TyVarId> =
        free_vars(&ty).difference(env_free).copied().collect();
    Scheme { vars: mine.into_iter().collect(), ty }
}
```

Phase 3 in `check_file`:

```rust
// Resolve every scheme through the final substitution first.
for s in infer.schemes.values_mut() {
    s.ty = crate::check::unify::apply_subst(&s.ty, &infer.subst);
}
// Compute env_free as union of free-vars across schemes whose binding sites
// already appear in the resolver's locals (i.e. lambda params escaping in
// closures). For Plan 4's top-level scope, env_free is empty: top-level
// bindings see only other top-level schemes, and we want each to be
// generalised independently.
let env_free = std::collections::HashSet::new();
let schemes_owned: Vec<_> = infer.schemes.drain().collect();
for (id, sch) in schemes_owned {
    let g = generalise(sch.ty, &env_free);
    infer.schemes.insert(id, g);
}
```

Add `lower_type` in `infer.rs` translating `crate::ast::Type` into `Ty`:

```rust
pub fn lower_type(&mut self, t: &crate::ast::Type) -> Ty {
    use crate::ast::TypeKind;
    match &t.node {
        TypeKind::Named { name, args } => {
            match name.as_str() {
                "Int" => Ty::Prim(crate::check::types::PrimTy::Int),
                "Float" => Ty::Prim(crate::check::types::PrimTy::Float),
                "String" => Ty::Prim(crate::check::types::PrimTy::String),
                "Bool" => Ty::Prim(crate::check::types::PrimTy::Bool),
                "Unit" => Ty::Prim(crate::check::types::PrimTy::Unit),
                _ => {
                    // Look up nominal type in resolver's defs.
                    let def = self.res.defs.iter().find(|d| &d.name == name);
                    match def {
                        Some(d) => Ty::Con(
                            d.id,
                            args.iter().map(|a| self.lower_type(a)).collect(),
                        ),
                        None => {
                            self.errors.push(Error {
                                span: t.span,
                                kind: crate::error::ErrorKind::Unresolved {
                                    name: name.clone(),
                                },
                            });
                            Ty::Var(self.fresh())
                        }
                    }
                }
            }
        }
        TypeKind::Var(_) => Ty::Var(self.fresh()),
        TypeKind::Function { params, effect, result } => {
            if effect.is_some()
                && !matches!(effect, Some(crate::ast::EffectRow::Empty))
            {
                self.errors.push(Error {
                    span: t.span,
                    kind: crate::error::ErrorKind::EffectsNotYetImplemented,
                });
            }
            Ty::Fun(
                params.iter().map(|p| self.lower_type(p)).collect(),
                Box::new(self.lower_type(result)),
            )
        }
        TypeKind::Tuple(_) => {
            // No tuple type in Plan 4's Ty model. Treat as Unresolved.
            self.errors.push(Error {
                span: t.span,
                kind: crate::error::ErrorKind::TuplesNotYetImplemented,
            });
            Ty::Var(self.fresh())
        }
    }
}
```

Add `EffectsNotYetImplemented` and `TuplesNotYetImplemented` to `ErrorKind`.

Wire annotation pinning: in the top-level phase-1 loop, if `decl.node.ty` is `Some(t)`, lower it and unify against the binding's `Ty::Var(v)` *before* phase 2 infers the body. That way the body's inferred type unifies through the annotation.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS — three new tests plus all earlier tests.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/error.rs tests/check_bindings.rs
git commit -m "Plan 4 Task 11: generalisation and type annotations"
```

---

## Task 12: Type registry and newtype declarations

**Files:**
- Create: `src/check/registry.rs`
- Modify: `src/check/mod.rs` (build registry before inference)
- Test: `tests/check_records.rs`

- [ ] **Step 1: Write the failing test**

`tests/check_records.rs`:

```rust
use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn newtype_declaration_is_registered() {
    let src = "\
type UserId = Int
firstUser : UserId
firstUser = 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res);
    // Plan 4 decision: a newtype is a distinct nominal type. `firstUser : UserId`
    // is annotated, and the literal `1 : Int` should NOT unify with `UserId` —
    // this should be a TypeMismatch.
    let errs = errs.unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}

#[test]
fn newtype_value_with_wrapped_construct_passes() {
    let src = "\
type UserId
    value : Int

firstUser : UserId
firstUser = UserId(value = 1)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let f = res.defs.iter().find(|d| d.name == "firstUser").unwrap();
    let user_id_def = res.defs.iter().find(|d| d.name == "UserId").unwrap();
    assert_eq!(typing.schemes[&f.id].ty, Ty::Con(user_id_def.id, vec![]));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_records -- newtype_`
Expected: FAIL — no `TypeRegistry`, no construction handling.

- [ ] **Step 3: Write minimal implementation**

`src/check/registry.rs`:

```rust
use crate::ast::{Decl, DeclKind, File, TypeBody, TypeMember, VariantBody};
use crate::check::types::Ty;
use crate::resolve::{DefId, Resolution};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub enum PayloadShape {
    Bare,
    Single(Ty),
    Record(Vec<FieldInfo>),
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub ctor_def_id: DefId,
    pub payload: PayloadShape,
    pub parent: DefId,
}

#[derive(Debug, Clone)]
pub enum TypeDeclBody {
    Newtype(Ty),
    Record { fields: Vec<FieldInfo> },
    Sum { variants: Vec<VariantInfo> },
}

#[derive(Debug, Clone)]
pub struct TypeDeclInfo {
    pub def_id: DefId,
    pub name: String,
    pub params: Vec<crate::check::types::TyVarId>,
    pub body: TypeDeclBody,
}

#[derive(Debug, Default, Clone)]
pub struct TypeRegistry {
    pub types: HashMap<DefId, TypeDeclInfo>,
    pub ctor_to_type: HashMap<DefId, DefId>,
}
```

Populating the registry is incremental: this task only handles `TypeBody::Newtype` (the single-line form). Subsequent tasks add the block form.

Hook `build_registry(file, res, infer)` into `check_file` before any expression inference. The registry construction calls `infer.lower_type(...)` to translate AST `Type` nodes into `Ty`, so it can allocate fresh tyvars for type parameters.

For the single-line newtype `type UserId = Int`:
- Look up `UserId` in `res.defs` → get the `DefId`.
- Lower the RHS type (`Int` → `Ty::Prim(PrimTy::Int)`).
- Insert `TypeDeclInfo { def_id, name: "UserId", params: vec![], body: TypeDeclBody::Newtype(Ty::Prim(Int)) }`.

The annotation `firstUser : UserId` lowers via `lower_type`. Make `lower_type` consult the registry for nominal types: if the name matches a registered type, build `Ty::Con(def_id, args)`. Newtypes are still represented as `Ty::Con(def_id, vec![])` — the wrapped type isn't unwrapped during unification, which is exactly what makes them nominally distinct.

For the second test (block-form newtype with `value : Int`), this needs the record-fields path from Task 13. Mark the test `#[ignore]` for Task 12 if needed and un-ignore in Task 13. (Plan note: deciding which test reaches green when is fine — the goal is forward progress.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_records -- newtype_declaration_is_registered`
Expected: PASS (first test). Second test ignored until Task 13.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_records.rs
git commit -m "Plan 4 Task 12: type registry and newtype declarations"
```

---

## Task 13: Record and sum type declarations

**Files:**
- Modify: `src/check/registry.rs` (handle `TypeBody::Block` — fields, variants)
- Modify: `src/check/mod.rs` (registry build pass walks block-form types)
- Test: `tests/check_sums.rs`, expand `tests/check_records.rs`

- [ ] **Step 1: Write the failing test**

`tests/check_sums.rs`:

```rust
use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn sum_type_with_variants_is_registered() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected check to succeed");
    // Schemes empty — no value bindings — but Circle and Rect should have
    // constructor schemes registered.
    let circle = res.defs.iter().find(|d| d.name == "Circle").unwrap();
    let rect = res.defs.iter().find(|d| d.name == "Rect").unwrap();
    assert!(typing.schemes.contains_key(&circle.id));
    assert!(typing.schemes.contains_key(&rect.id));
}
```

And un-ignore `newtype_value_with_wrapped_construct_passes` in `tests/check_records.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums -- sum_type_with_variants_is_registered`
Expected: FAIL — block-form types not walked, no ctor schemes.

- [ ] **Step 3: Write minimal implementation**

In `build_registry`, walk `TypeBody::Block(members)` and split members:
- `TypeMember::Field { name, ty }` → push into `fields` (record).
- `TypeMember::Variant { name, body }` → push into `variants` (sum). Bodies:
  - `VariantBody::Bare` → `PayloadShape::Bare`.
  - `VariantBody::Single(ty)` → `PayloadShape::Single(lower_type(ty))`.
  - `VariantBody::Fields(members)` → recurse to collect field names/types; produce `PayloadShape::Record(...)`.
- `TypeMember::Method(decl)` → defer to Task 14 (record/method registration).

If a block has both fields *and* variants in v1, it's a sum with method-like state — but the spec doesn't (yet) describe that combination. For Plan 4, if a block has variants present, it's a `Sum`; otherwise it's a `Record`. Field-and-variant mixing produces an error: `MixedFieldsAndVariants { type_name }` (add to `ErrorKind`).

For each variant collected, look up its constructor `DefId` from the resolver and register an entry into `Typing.schemes` with the constructor scheme:
- `Bare` → `Scheme { vars: type_params, ty: Ty::Con(parent_def_id, params_as_vars) }`.
- `Single(ty)` → `Scheme { vars: type_params, ty: Ty::Fun(vec![ty], Box::new(Ty::Con(parent_def_id, params_as_vars))) }`.
- `Record(fields)` → constructor is conceptually `(field1, field2, ...) -> Parent`. Encode as `Ty::Fun(fields_in_order, Box::new(Ty::Con(parent_def_id, params_as_vars)))`; the record-pattern Task 18 reads payload field names from the registry. (For Plan 4 we use positional ctor application; named-arg ctor calls land in Task 14's construction logic.)

Build `ctor_to_type[ctor_def_id] = parent_def_id` for fast lookup.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_sums`, `cargo test --test check_records`
Expected: PASS — sum-type test green, newtype-wrapped-construct test green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/error.rs tests/
git commit -m "Plan 4 Task 13: record and sum type declarations"
```

---

## Task 14: Record construction and update

**Files:**
- Modify: `src/check/infer.rs` (Construct, Update arms)
- Modify: `src/error.rs` (UnknownField, MissingField, UnknownType)
- Test: `tests/check_records.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_records.rs`:

```rust
#[test]
fn record_construction_with_all_fields_succeeds() {
    let src = "\
type Point
    x : Float
    y : Float

origin = Point(x = 0.0, y = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let origin = res.defs.iter().find(|d| d.name == "origin").unwrap();
    let point = res.defs.iter().find(|d| d.name == "Point").unwrap();
    assert_eq!(typing.schemes[&origin.id].ty, Ty::Con(point.id, vec![]));
}

#[test]
fn record_construction_missing_field_errors() {
    let src = "\
type Point
    x : Float
    y : Float

bad = Point(x = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::MissingField { .. })));
}

#[test]
fn record_construction_unknown_field_errors() {
    let src = "\
type Point
    x : Float
    y : Float

bad = Point(x = 0.0, y = 0.0, z = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::UnknownField { .. })));
}

#[test]
fn record_update_keeps_type() {
    let src = "\
type Point
    x : Float
    y : Float

p1 = Point(x = 0.0, y = 0.0)
p2 = p1(x = 5.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let p1 = res.defs.iter().find(|d| d.name == "p1").unwrap();
    let p2 = res.defs.iter().find(|d| d.name == "p2").unwrap();
    assert_eq!(typing.schemes[&p1.id].ty, typing.schemes[&p2.id].ty);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_records -- record_construction record_update`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

Add `MissingField { type_name, field }`, `UnknownField { type_name, field }`, `UnknownType { name }` to `ErrorKind`.

Add to `infer_expr`:

```rust
ExprKind::Construct { type_name, fields } => {
    let def = self.res.defs.iter().find(|d| &d.name == type_name).cloned();
    let Some(def) = def else {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownType { name: type_name.clone() },
        });
        return Ty::Var(self.fresh());
    };
    let entry = self.registry.types.get(&def.id).cloned();
    let Some(info) = entry else {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownType { name: type_name.clone() },
        });
        return Ty::Var(self.fresh());
    };
    let TypeDeclBody::Record { fields: decl_fields } = &info.body else {
        // Newtype with single value field, or sum — for now treat newtype-block
        // as record with one field. Sum construction goes through Ctor, not here.
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownType { name: type_name.clone() },
        });
        return Ty::Var(self.fresh());
    };
    // Check every declared field is supplied exactly once; check no extras.
    use std::collections::HashSet;
    let declared: HashSet<&str> = decl_fields.iter().map(|f| f.name.as_str()).collect();
    let provided: HashSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    for missing in declared.difference(&provided) {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::MissingField {
                type_name: type_name.clone(),
                field: missing.to_string(),
            },
        });
    }
    for extra in provided.difference(&declared) {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownField {
                type_name: type_name.clone(),
                field: extra.to_string(),
            },
        });
    }
    // For each supplied field, unify provided expr type with declared.
    for kw in fields {
        let provided_ty = self.infer_expr(&kw.value);
        if let Some(decl) = decl_fields.iter().find(|f| f.name == kw.name) {
            if let Err(ue) = crate::check::unify::unify(
                &mut self.subst, &decl.ty, &provided_ty,
            ) {
                self.errors.push(unify_to_error(kw.value.span, ue));
            }
        }
    }
    Ty::Con(def.id, vec![])
}
```

For `Update { value, fields }`: infer `value`, expect `Ty::Con(def_id, args)`, look up the type's fields, unify each provided value against the declared field type, return the same `Ty::Con(...)`. UnknownField is the relevant error here too; MissingField doesn't apply to update.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_records`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/error.rs tests/check_records.rs
git commit -m "Plan 4 Task 14: record construction and update"
```

---

## Task 15: Field access and method calls

**Files:**
- Modify: `src/check/infer.rs` (FieldAccess, MethodCall arms)
- Modify: `src/check/registry.rs` (track methods on types)
- Test: `tests/check_records.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_records.rs`:

```rust
#[test]
fn field_access_returns_field_type() {
    let src = "\
type Point
    x : Float
    y : Float

xCoord : Float
xCoord = (Point(x = 1.0, y = 2.0)).x
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let x = res.defs.iter().find(|d| d.name == "xCoord").unwrap();
    assert_eq!(typing.schemes[&x.id].ty, Ty::Prim(PrimTy::Float));
}

#[test]
fn method_call_resolves_to_method_scheme() {
    let src = "\
type Point
    x : Float
    y : Float
    magnitude = -> (self.x * self.x + self.y * self.y) ^ 0.5

result : Float
result = (Point(x = 3.0, y = 4.0)).magnitude
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let r = res.defs.iter().find(|d| d.name == "result").unwrap();
    assert_eq!(typing.schemes[&r.id].ty, Ty::Prim(PrimTy::Float));
}
```

Note `magnitude = -> ...` is a zero-arg method; on access it's already applied (no extra args needed). Field-vs-method dispatch needs to know whether `.magnitude` is a no-arg method call or a method-as-value reference. For Plan 4: a `MethodCall` AST node means "method-call shape" — apply if the method is a zero-arg lambda, otherwise return the method's type. Calls with args (`p.distance other`) parse as `Call(MethodCall(p, distance), [other])` — the outer Call then unifies as in Task 9.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_records -- field_access method_call`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

Add method registration to the registry: when walking `TypeBody::Block`, collect `TypeMember::Method(decl)` items into a per-type `methods: Vec<(String, DefId)>` list. The method's `DefId` is allocated by the resolver (the resolver registers each method-binding as a top-level `Value` def — verify this in `src/resolve/scope.rs::collect_top_level`; if not, add it).

Then `infer.schemes[method_def_id]` gets the method's typed scheme just like any top-level value (Tasks 6 + 11 already cover this — methods are walked as part of phase-1 and phase-2 if we add their bindings to `pending`).

`infer_expr` for `MethodCall { receiver, method }`:

```rust
ExprKind::MethodCall { receiver, method } => {
    let recv_ty = self.infer_expr(receiver);
    let recv_resolved = crate::check::unify::apply_subst(&recv_ty, &self.subst);
    let parent_def = match recv_resolved {
        Ty::Con(id, _) => id,
        _ => {
            self.errors.push(Error {
                span: receiver.span,
                kind: crate::error::ErrorKind::CannotAccessMember {
                    ty: format!("{:?}", recv_resolved),
                    member: method.clone(),
                },
            });
            return Ty::Var(self.fresh());
        }
    };
    let info = self.registry.types.get(&parent_def).cloned();
    let Some(info) = info else {
        // self.errors.push(...);
        return Ty::Var(self.fresh());
    };
    // Field lookup first.
    if let TypeDeclBody::Record { fields } = &info.body {
        if let Some(f) = fields.iter().find(|f| &f.name == method) {
            return f.ty.clone();
        }
    }
    // Method lookup.
    if let Some(method_def_id) = info.methods.iter()
        .find(|(name, _)| name == method)
        .map(|(_, id)| *id)
    {
        let scheme = self.schemes.get(&method_def_id).cloned()
            .unwrap_or(Scheme { vars: Vec::new(), ty: Ty::Var(self.fresh()) });
        let inst = self.instantiate(scheme);
        // If the method's scheme is a Fun whose first param is `self`, it's
        // already (Point, ...) -> R. Unify recv_ty with first param and
        // return Fun(remaining_params) -> R. For a zero-arg method with
        // signature Fun(vec![Point], Box::new(R)), the result is R.
        if let Ty::Fun(params, ret) = inst {
            if params.is_empty() {
                return *ret;
            }
            let first = params[0].clone();
            if let Err(ue) = crate::check::unify::unify(&mut self.subst, &first, &recv_ty) {
                self.errors.push(unify_to_error(receiver.span, ue));
            }
            let rest = params[1..].to_vec();
            if rest.is_empty() {
                return *ret;
            }
            return Ty::Fun(rest, ret);
        }
        return inst;
    }
    self.errors.push(Error {
        span: e.span,
        kind: crate::error::ErrorKind::UnknownField {
            type_name: info.name.clone(),
            field: method.clone(),
        },
    });
    Ty::Var(self.fresh())
}
```

Same path for `FieldAccess { receiver, field }`: look up field on the parent type's record body, or fall through to method lookup. Plan note: for sum types, `instance.field` could mean accessing a variant payload field — that's a Plan 4.5 or later concern. For now, error.

Add `CannotAccessMember { ty, member }` to `ErrorKind`.

Methods need to be picked up by phase-1: add a recursive walk in `check_file`'s phase-1 collection that descends into `TypeMember::Method(decl)` inside `TypeDecl` bodies and adds each method's binding to `pending`. The method body's `self` is already a local (the resolver injects it); the phase-1 fresh-tyvar slot should be `Ty::Fun(vec![Ty::Con(parent_def_id, ...)], result_v)` so the body's `self` lookups resolve correctly.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_records`
Expected: PASS — field-access and method-call tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/error.rs tests/check_records.rs
git commit -m "Plan 4 Task 15: field access and method calls"
```

---

## Task 16: Constructor application

**Files:**
- Modify: `src/check/infer.rs` (Ctor arm — real impl, replacing the Task 7 stub)
- Test: `tests/check_sums.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_sums.rs`:

```rust
use i_lang::check::types::{PrimTy, Ty};

#[test]
fn bare_variant_has_parent_type() {
    let src = "\
type Maybe a
    None
    Some : a

empty : Maybe Int
empty = None
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let e = res.defs.iter().find(|d| d.name == "empty").unwrap();
    let maybe = res.defs.iter().find(|d| d.name == "Maybe").unwrap();
    assert_eq!(typing.schemes[&e.id].ty, Ty::Con(maybe.id, vec![Ty::Prim(PrimTy::Int)]));
}

#[test]
fn single_payload_ctor_takes_payload_type() {
    let src = "\
type Maybe a
    None
    Some : a

three : Maybe Int
three = Some 3
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let t = res.defs.iter().find(|d| d.name == "three").unwrap();
    let maybe = res.defs.iter().find(|d| d.name == "Maybe").unwrap();
    assert_eq!(typing.schemes[&t.id].ty, Ty::Con(maybe.id, vec![Ty::Prim(PrimTy::Int)]));
}

#[test]
fn ctor_payload_type_mismatch_errors() {
    let src = "\
type Maybe a
    None
    Some : a

bad : Maybe Int
bad = Some \"hi\"
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums -- bare_variant single_payload_ctor`
Expected: FAIL — `Ctor` arm in `infer_expr` returns a fresh tyvar.

- [ ] **Step 3: Write minimal implementation**

Replace the `Ctor` stub:

```rust
ExprKind::Ctor(name) => {
    let resolved = self.res.refs.get(&e.span).cloned();
    let Some(crate::resolve::ResolvedName::Ctor(ctor_id)) = resolved else {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::Unresolved { name: name.clone() },
        });
        return Ty::Var(self.fresh());
    };
    let scheme = self.schemes.get(&ctor_id).cloned()
        .unwrap_or(Scheme { vars: Vec::new(), ty: Ty::Var(self.fresh()) });
    self.instantiate(scheme)
}
```

That gives bare-variant `None` the type `Maybe a` directly (after instantiation), and gives `Some` the type `a -> Maybe a` (a function), so calling `Some 3` resolves through the normal `Call` arm.

For type-parameter handling: phase-1 needs to use the type's params in the ctor's scheme. When building `Some`'s scheme, the scheme is `Scheme { vars: [α], ty: Fun([α], Box(Con(maybe_id, [α]))) }`. Each instantiation gives fresh α.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_sums`
Expected: PASS — three new tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_sums.rs
git commit -m "Plan 4 Task 16: constructor application"
```

---

## Task 17: Pattern typing — wildcards, vars, literals

**Files:**
- Modify: `src/check/infer.rs` (extend `infer_pattern` for `Wildcard`, `Lit`)
- Test: `tests/check_sums.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_sums.rs`:

```rust
#[test]
fn match_with_wildcard_on_int_type_checks() {
    let src = "\
classify : Int -> Int
classify = n -> n match
    0 -> 0
    _ -> 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let c = res.defs.iter().find(|d| d.name == "classify").unwrap();
    let expected = Ty::Fun(vec![Ty::Prim(PrimTy::Int)], Box::new(Ty::Prim(PrimTy::Int)));
    assert_eq!(typing.schemes[&c.id].ty, expected);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums -- match_with_wildcard`
Expected: FAIL — Match arm isn't handled yet (arrives in Task 20), but the pattern-typing primitives should compile cleanly.

- [ ] **Step 3: Write minimal implementation**

Extend `infer_pattern`:

```rust
match &p.node {
    PatternKind::Wildcard => {
        let v = self.fresh();
        PatternResult { ty: Ty::Var(v), bindings: vec![] }
    }
    PatternKind::Lit(lit) => {
        let ty = match lit {
            LitPat::Int(_) => Ty::Prim(PrimTy::Int),
            LitPat::Float(_) => Ty::Prim(PrimTy::Float),
            LitPat::Str(_) => Ty::Prim(PrimTy::String),
        };
        PatternResult { ty, bindings: vec![] }
    }
    PatternKind::Var(_) => { /* Task 8 stub */ }
    _ => { /* later tasks */ }
}
```

The Match arm in Task 20 will use these. For Task 17, the test won't fully pass until Task 20 — mark it `#[ignore]` and un-ignore in Task 20.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib check::infer` and confirm the pattern unit tests are green; the integration test for Match stays ignored.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_sums.rs
git commit -m "Plan 4 Task 17: pattern typing for wildcards, vars, literals"
```

---

## Task 18: Pattern typing — constructors and records

**Files:**
- Modify: `src/check/infer.rs` (extend `infer_pattern` for `Ctor`, `Record`)
- Test: inline unit tests in `src/check/infer.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/check/infer.rs#[cfg(test)] mod tests`:

```rust
#[test]
fn pattern_some_x_binds_payload() {
    use crate::ast::{Pattern, PatternKind};
    use crate::span::Spanned;
    // Build a Some pattern manually with a fresh resolver setup. Easier: build
    // the whole test through parsing and resolving a small source fragment, e.g.
    //   type Maybe a
    //       None
    //       Some : a
    //   f = x -> x match
    //       Some n -> n
    //       None -> 0
    // and assert `f : Maybe Int -> Int` after Match arrives in Task 20.
    // For Task 18, the unit test verifies infer_pattern on a hand-built Ctor pattern.
}
```

Rather than a brittle hand-built AST test, write the higher-level integration test in `tests/check_sums.rs` and mark it `#[ignore]` until Task 20:

```rust
#[test]
#[ignore = "needs Match (Task 20)"]
fn match_unwraps_maybe() {
    let src = "\
type Maybe a
    None
    Some : a

unwrapOr : Maybe Int, Int -> Int
unwrapOr = m d -> m match
    Some n -> n
    None -> d
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    assert!(typing.schemes.values().any(|s| matches!(&s.ty, Ty::Fun(_, _))));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums`
Expected: Build clean (the ignored test doesn't run; passes still pass).

- [ ] **Step 3: Write minimal implementation**

In `infer_pattern`:

```rust
PatternKind::Ctor { name, args } => {
    let resolved = self.res.refs.get(&p.span).cloned();
    let ctor_id = match resolved {
        Some(crate::resolve::ResolvedName::Ctor(id)) => id,
        _ => {
            self.errors.push(Error {
                span: p.span,
                kind: crate::error::ErrorKind::Unresolved { name: name.clone() },
            });
            return PatternResult { ty: Ty::Var(self.fresh()), bindings: vec![] };
        }
    };
    let scheme = self.schemes.get(&ctor_id).cloned()
        .unwrap_or(Scheme { vars: Vec::new(), ty: Ty::Var(self.fresh()) });
    let inst = self.instantiate(scheme);
    // Bare: inst is Con(...); Single/Record: inst is Fun(payloads, Con(...)).
    let (payload_tys, result_ty) = match inst {
        Ty::Fun(ps, r) => (ps, *r),
        other => (Vec::new(), other),
    };
    if payload_tys.len() != args.len() {
        self.errors.push(Error {
            span: p.span,
            kind: crate::error::ErrorKind::ArityMismatch {
                expected: payload_tys.len(),
                found: args.len(),
            },
        });
    }
    let mut bindings = Vec::new();
    for (pt, sub_pat) in payload_tys.iter().zip(args.iter()) {
        let sub = self.infer_pattern(sub_pat);
        if let Err(ue) = crate::check::unify::unify(&mut self.subst, pt, &sub.ty) {
            self.errors.push(unify_to_error(sub_pat.span, ue));
        }
        bindings.extend(sub.bindings);
    }
    PatternResult { ty: result_ty, bindings }
}
```

Record pattern uses `registry.types[parent].body` (must be `Record { fields }`) to map field names to declared types; unify each field's sub-pattern.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS — no regressions; the ignored Match test stays ignored.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_sums.rs
git commit -m "Plan 4 Task 18: pattern typing for constructors and records"
```

---

## Task 19: Pattern typing — lists

**Files:**
- Modify: `src/check/infer.rs` (extend `infer_pattern` for `List`)
- Test: inline (or fold into Task 23's list-literal task if simpler)

- [ ] **Step 1: Write the failing test**

For a `PatternKind::List(items)` whose items each have type `α`, the pattern has type `List α`. Requires `List` to be in scope (corpus fixture provides `type List a` stub). For Task 19, write the unit test in `infer.rs` checking the pattern's resulting type.

If the test setup gets bulky, defer the list pattern test to Task 23 (which builds the list literal infrastructure) and just stub `PatternKind::List(_) => PatternResult { ty: Ty::Var(self.fresh()), bindings: vec![] }` here, with a `// TODO Task 23` note (and a passing test that doesn't exercise lists).

- [ ] **Step 2: Run test to verify it fails / step 3: implement / step 4: pass**

Standard. The minimal impl is:

```rust
PatternKind::List(items) => {
    // Look up `List` in resolver defs; if missing, emit UnknownType.
    let list_def = self.res.defs.iter().find(|d| d.name == "List").cloned();
    let Some(list_def) = list_def else {
        self.errors.push(Error {
            span: p.span,
            kind: crate::error::ErrorKind::UnknownType { name: "List".into() },
        });
        return PatternResult { ty: Ty::Var(self.fresh()), bindings: vec![] };
    };
    let elem_var = self.fresh();
    let mut bindings = Vec::new();
    for item in items {
        let sub = self.infer_pattern(item);
        if let Err(ue) = crate::check::unify::unify(
            &mut self.subst, &Ty::Var(elem_var), &sub.ty,
        ) {
            self.errors.push(unify_to_error(item.span, ue));
        }
        bindings.extend(sub.bindings);
    }
    PatternResult {
        ty: Ty::Con(list_def.id, vec![Ty::Var(elem_var)]),
        bindings,
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add src/check/
git commit -m "Plan 4 Task 19: pattern typing for lists"
```

---

## Task 20: Match expression typing

**Files:**
- Modify: `src/check/infer.rs` (Match arm)
- Test: `tests/check_sums.rs` (un-ignore prior tests)

- [ ] **Step 1: Write the failing test**

Un-ignore the previously-marked tests in `tests/check_sums.rs`: `match_with_wildcard_on_int_type_checks`, `match_unwraps_maybe`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums`
Expected: FAIL — Match arm isn't implemented.

- [ ] **Step 3: Write minimal implementation**

```rust
ExprKind::Match { scrutinee, arms } => {
    let scrutinee_ty = self.infer_expr(scrutinee);
    let result_v = self.fresh();
    for arm in arms {
        let pat = self.infer_pattern(&arm.pattern);
        if let Err(ue) = crate::check::unify::unify(
            &mut self.subst, &scrutinee_ty, &pat.ty,
        ) {
            self.errors.push(unify_to_error(arm.pattern.span, ue));
        }
        let body_ty = self.infer_expr(&arm.body);
        if let Err(ue) = crate::check::unify::unify(
            &mut self.subst, &Ty::Var(result_v), &body_ty,
        ) {
            self.errors.push(unify_to_error(arm.body.span, ue));
        }
    }
    Ty::Var(result_v)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_sums`
Expected: PASS — Match typing works; un-ignored tests green.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_sums.rs
git commit -m "Plan 4 Task 20: match expression typing"
```

---

## Task 21: Exhaustiveness checking for sum types

**Files:**
- Create: `src/check/exhaust.rs`
- Modify: `src/check/mod.rs` (wire exhaust into `mod`)
- Modify: `src/check/infer.rs` (after Match arm types successfully, run exhaustiveness)
- Modify: `src/error.rs` (`NonExhaustiveMatch { missing: Vec<String> }`)
- Test: `tests/check_sums.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_sums.rs`:

```rust
#[test]
fn non_exhaustive_match_errors() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

f : Shape -> Float
f = s -> s match
    Circle r -> r
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        i_lang::error::ErrorKind::NonExhaustiveMatch { missing }
            if missing.iter().any(|m| m == "Rect")
    )));
}

#[test]
fn exhaustive_match_with_wildcard_is_ok() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

f : Shape -> Float
f = s -> s match
    Circle r -> r
    _ -> 0.0
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    check_file(&file, &res).unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_sums -- non_exhaustive_match`
Expected: FAIL — no exhaustiveness check.

- [ ] **Step 3: Write minimal implementation**

`src/check/exhaust.rs`:

```rust
use crate::ast::{MatchArm, PatternKind};
use crate::check::registry::{TypeDeclBody, TypeRegistry};
use crate::check::types::Ty;
use crate::resolve::Resolution;
use std::collections::HashSet;

pub enum Coverage {
    Exhaustive,
    Missing(Vec<String>),
}

pub fn check_arms(
    scrutinee: &Ty,
    arms: &[MatchArm],
    registry: &TypeRegistry,
    res: &Resolution,
) -> Coverage {
    if arms.iter().any(|a| matches!(a.pattern.node, PatternKind::Wildcard | PatternKind::Var(_))) {
        return Coverage::Exhaustive;
    }
    let Ty::Con(parent, _) = scrutinee else {
        return Coverage::Exhaustive; // primitives need a wildcard; if none, the caller errors.
    };
    let Some(info) = registry.types.get(parent) else { return Coverage::Exhaustive; };
    let TypeDeclBody::Sum { variants } = &info.body else {
        return Coverage::Exhaustive; // records don't have variants to cover.
    };
    let covered: HashSet<String> = arms.iter().filter_map(|a| match &a.pattern.node {
        PatternKind::Ctor { name, .. } => Some(name.clone()),
        _ => None,
    }).collect();
    let missing: Vec<String> = variants
        .iter()
        .map(|v| v.name.clone())
        .filter(|n| !covered.contains(n))
        .collect();
    if missing.is_empty() {
        Coverage::Exhaustive
    } else {
        Coverage::Missing(missing)
    }
}
```

Wire `check_arms` into the Match arm in `infer_expr`. After typing the scrutinee and arms, run `check_arms` against the substituted scrutinee type:

```rust
let scrutinee_resolved = crate::check::unify::apply_subst(&scrutinee_ty, &self.subst);
match crate::check::exhaust::check_arms(&scrutinee_resolved, arms, &self.registry, self.res) {
    crate::check::exhaust::Coverage::Exhaustive => {}
    crate::check::exhaust::Coverage::Missing(missing) => {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::NonExhaustiveMatch { missing },
        });
    }
}
```

Add `NonExhaustiveMatch { missing: Vec<String> }` to `ErrorKind`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_sums`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/check/ src/error.rs tests/check_sums.rs
git commit -m "Plan 4 Task 21: exhaustiveness checking for sum types"
```

---

## Task 22: Primitive binary and unary operators

**Files:**
- Modify: `src/check/infer.rs` (BinOp, UnaryOp arms)
- Test: `tests/check_literals.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_literals.rs`:

```rust
#[test]
fn int_addition_is_int() {
    let src = "n = 1 + 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn mixed_int_float_addition_errors() {
    let src = "n = 1 + 1.0\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}

#[test]
fn comparison_returns_bool() {
    let src = "b = 1 == 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let b = res.defs.iter().find(|d| d.name == "b").unwrap();
    assert_eq!(typing.schemes[&b.id].ty, Ty::Prim(PrimTy::Bool));
}

#[test]
fn logical_and_requires_bool() {
    let src = "b = 1 and 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals -- int_addition mixed_int comparison logical`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

```rust
ExprKind::BinOp { op, lhs, rhs } => {
    let lhs_ty = self.infer_expr(lhs);
    let rhs_ty = self.infer_expr(rhs);
    let (expect, result) = match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
            // Both sides must unify; restrict to Int or Float by unifying lhs with a
            // primitive *after* the fact via a custom predicate. Simpler: unify
            // lhs with rhs (homogeneous), then assert that the resolved type is
            // Int or Float.
            if let Err(ue) = crate::check::unify::unify(&mut self.subst, &lhs_ty, &rhs_ty) {
                self.errors.push(unify_to_error(e.span, ue));
            }
            let resolved = crate::check::unify::apply_subst(&lhs_ty, &self.subst);
            match resolved {
                Ty::Prim(PrimTy::Int) | Ty::Prim(PrimTy::Float) => {}
                _ => self.errors.push(Error {
                    span: e.span,
                    kind: crate::error::ErrorKind::TypeMismatch {
                        expected: "Int or Float".into(),
                        found: format!("{:?}", resolved),
                    },
                }),
            }
            return resolved;
        }
        BinOp::Eq | BinOp::Ne => {
            if let Err(ue) = crate::check::unify::unify(&mut self.subst, &lhs_ty, &rhs_ty) {
                self.errors.push(unify_to_error(e.span, ue));
            }
            return Ty::Prim(PrimTy::Bool);
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            if let Err(ue) = crate::check::unify::unify(&mut self.subst, &lhs_ty, &rhs_ty) {
                self.errors.push(unify_to_error(e.span, ue));
            }
            let resolved = crate::check::unify::apply_subst(&lhs_ty, &self.subst);
            match resolved {
                Ty::Prim(PrimTy::Int) | Ty::Prim(PrimTy::Float) => {}
                _ => self.errors.push(Error {
                    span: e.span,
                    kind: crate::error::ErrorKind::TypeMismatch {
                        expected: "Int or Float".into(),
                        found: format!("{:?}", resolved),
                    },
                }),
            }
            return Ty::Prim(PrimTy::Bool);
        }
        BinOp::And | BinOp::Or | BinOp::Xor => (Ty::Prim(PrimTy::Bool), Ty::Prim(PrimTy::Bool)),
        BinOp::Concat => (Ty::Prim(PrimTy::String), Ty::Prim(PrimTy::String)),
    };
    if let Err(ue) = crate::check::unify::unify(&mut self.subst, &lhs_ty, &expect) {
        self.errors.push(unify_to_error(lhs.span, ue));
    }
    if let Err(ue) = crate::check::unify::unify(&mut self.subst, &rhs_ty, &expect) {
        self.errors.push(unify_to_error(rhs.span, ue));
    }
    result
}

ExprKind::UnaryOp { op, expr } => {
    let inner = self.infer_expr(expr);
    match op {
        crate::ast::UnaryOp::Neg => {
            let resolved = crate::check::unify::apply_subst(&inner, &self.subst);
            match resolved {
                Ty::Prim(PrimTy::Int) | Ty::Prim(PrimTy::Float) => resolved,
                _ => {
                    self.errors.push(Error {
                        span: e.span,
                        kind: crate::error::ErrorKind::TypeMismatch {
                            expected: "Int or Float".into(),
                            found: format!("{:?}", resolved),
                        },
                    });
                    Ty::Var(self.fresh())
                }
            }
        }
        crate::ast::UnaryOp::Not => {
            if let Err(ue) = crate::check::unify::unify(
                &mut self.subst, &inner, &Ty::Prim(PrimTy::Bool),
            ) {
                self.errors.push(unify_to_error(e.span, ue));
            }
            Ty::Prim(PrimTy::Bool)
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_literals`
Expected: PASS — four new tests plus existing ones.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_literals.rs
git commit -m "Plan 4 Task 22: primitive binary and unary operators"
```

---

## Task 23: List literal typing

**Files:**
- Modify: `src/check/infer.rs` (List arm)
- Test: `tests/check_literals.rs` (add)

- [ ] **Step 1: Write the failing test**

Add to `tests/check_literals.rs`:

```rust
#[test]
fn empty_list_is_list_of_var() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs : List Int
xs = []
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let xs = res.defs.iter().find(|d| d.name == "xs").unwrap();
    let list_def = res.defs.iter().find(|d| d.name == "List").unwrap();
    assert_eq!(typing.schemes[&xs.id].ty, Ty::Con(list_def.id, vec![Ty::Prim(PrimTy::Int)]));
}

#[test]
fn homogeneous_list_takes_element_type() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs = [1, 2, 3]
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let xs = res.defs.iter().find(|d| d.name == "xs").unwrap();
    let list_def = res.defs.iter().find(|d| d.name == "List").unwrap();
    assert_eq!(typing.schemes[&xs.id].ty, Ty::Con(list_def.id, vec![Ty::Prim(PrimTy::Int)]));
}

#[test]
fn heterogeneous_list_errors() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs = [1, \"hi\"]
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs
        .iter()
        .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_literals -- empty_list homogeneous_list heterogeneous_list`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

```rust
ExprKind::List(items) => {
    let list_def = self.res.defs.iter().find(|d| d.name == "List").cloned();
    let Some(list_def) = list_def else {
        self.errors.push(Error {
            span: e.span,
            kind: crate::error::ErrorKind::UnknownType { name: "List".into() },
        });
        return Ty::Var(self.fresh());
    };
    let elem = Ty::Var(self.fresh());
    for item in items {
        let it = self.infer_expr(item);
        if let Err(ue) = crate::check::unify::unify(&mut self.subst, &elem, &it) {
            self.errors.push(unify_to_error(item.span, ue));
        }
    }
    Ty::Con(list_def.id, vec![elem])
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_literals`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/check/ tests/check_literals.rs
git commit -m "Plan 4 Task 23: list literal typing"
```

---

## Task 24: Pretty-print `Ty` and tidy error messages

**Files:**
- Modify: `src/check/types.rs` (add `Display for Ty`, `Display for Scheme`)
- Modify: `src/check/infer.rs` (replace `format!("{:?}", ty)` with `format!("{}", ty)` in error construction)
- Test: inline `#[cfg(test)] mod tests` in `types.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::DefId;

    #[test]
    fn display_primitive() {
        assert_eq!(format!("{}", Ty::Prim(PrimTy::Int)), "Int");
        assert_eq!(format!("{}", Ty::Prim(PrimTy::Float)), "Float");
    }

    #[test]
    fn display_function() {
        let ty = Ty::Fun(
            vec![Ty::Prim(PrimTy::Int), Ty::Prim(PrimTy::Int)],
            Box::new(Ty::Prim(PrimTy::Bool)),
        );
        assert_eq!(format!("{}", ty), "Int, Int -> Bool");
    }

    #[test]
    fn display_var_uses_letter_alphabet() {
        // Var(0) → "a", Var(1) → "b", ... after pretty-printing pass.
        // For this task, just verify Vars don't print as "TyVarId(0)".
        let s = format!("{}", Ty::Var(TyVarId(0)));
        assert!(!s.contains("TyVarId"));
    }

    #[test]
    fn display_scheme_includes_forall_when_quantified() {
        let scheme = Scheme {
            vars: vec![TyVarId(0)],
            ty: Ty::Fun(
                vec![Ty::Var(TyVarId(0))],
                Box::new(Ty::Var(TyVarId(0))),
            ),
        };
        let s = format!("{}", scheme);
        assert!(s.starts_with("forall"));
    }
}
```

- [ ] **Step 2 / 3 / 4: implement and verify**

A minimal `Display`:

```rust
impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Var(v) => write!(f, "t{}", v.0),
            Ty::Prim(p) => write!(f, "{}", match p {
                PrimTy::Int => "Int",
                PrimTy::Float => "Float",
                PrimTy::String => "String",
                PrimTy::Bool => "Bool",
                PrimTy::Unit => "Unit",
            }),
            Ty::Con(id, args) => {
                write!(f, "#{}", id.0)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", a)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Ty::Fun(ps, r) => {
                for (i, p) in ps.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, " -> {}", r)
            }
        }
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.vars.is_empty() {
            write!(f, "forall")?;
            for v in &self.vars {
                write!(f, " t{}", v.0)?;
            }
            write!(f, " . ")?;
        }
        write!(f, "{}", self.ty)
    }
}
```

Replace `format!("{:?}", ty)` call sites in `infer.rs` error construction with `format!("{}", ty)`.

`Ty::Con(id, ...)` currently prints `#<DefId>` — Plan 4 doesn't store nominal type names in the `Ty` itself, so the printer needs an optional name-resolution callback. For Plan 4, leaving `#<DefId>` is acceptable (the corpus snapshot is reviewed alongside the source; the user knows what `#5` refers to). Plan 5 may add `Display` taking a `&Resolution` to print friendly names.

- [ ] **Step 5: Commit**

```bash
git add src/check/
git commit -m "Plan 4 Task 24: pretty-print Ty and Scheme"
```

---

## Task 25: Corpus snapshots

**Files:**
- Create: `tests/check_corpus.rs`
- Create: `tests/corpus/check/*.i` (small fixtures, one per feature)
- Modify: `src/check/types.rs` (add `Display for Typing`)

- [ ] **Step 1: Write the failing test**

`tests/check_corpus.rs`:

```rust
use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn snapshot_check_corpus() {
    insta::glob!(env!("CARGO_MANIFEST_DIR"), "tests/corpus/check/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).expect("lex");
        let file = parse(&toks).expect("parse");
        let res = resolve_file(&file).expect("resolve");
        let typing = check_file(&file, &res).expect("check");
        insta::assert_snapshot!(format!("{}", typing));
    });
}
```

Create six minimal fixtures (each ~5–10 lines) under `tests/corpus/check/`:
- `identity.i` — `id = x -> x`
- `lambda-app.i` — `id = x -> x` then `n = id 42`
- `mutual-rec.i` — two top-level bindings referring to each other
- `record-build-update.i` — a record with construction and update
- `sum-match.i` — a sum type with a complete match
- `method-self.i` — a type with a method using `self`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_corpus`
Expected: FAIL — `Display for Typing` not implemented; or insta has no accepted snapshots yet.

- [ ] **Step 3: Write minimal implementation**

Add `Display for Typing` (prints each top-level scheme, sorted by DefId for determinism):

```rust
impl std::fmt::Display for Typing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "schemes:")?;
        let mut entries: Vec<_> = self.schemes.iter().collect();
        entries.sort_by_key(|(id, _)| id.0);
        for (id, scheme) in entries {
            writeln!(f, "  #{} : {}", id.0, scheme)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run snapshots and accept**

Run: `cargo test --test check_corpus`
Then: `cargo insta review` and accept each snapshot (this is the user's pause — hand off, then resume).

- [ ] **Step 5: Commit**

```bash
git add src/check/types.rs tests/check_corpus.rs tests/corpus/check/ tests/snapshots/check_corpus__*.snap
git commit -m "Plan 4 Task 25: type-checker corpus snapshots"
```

---

## Task 26: End-to-end test against a small program

**Files:**
- Create: `tests/check_end_to_end.rs`

- [ ] **Step 1: Write the failing test**

```rust
use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn small_program_with_records_methods_and_match_type_checks() {
    let src = "\
type Maybe a
    None
    Some : a

type Point
    x : Float
    y : Float
    magnitude = -> (self.x * self.x + self.y * self.y) ^ 0.5

origin : Point
origin = Point(x = 0.0, y = 0.0)

magOrZero : Maybe Point -> Float
magOrZero = mp -> mp match
    Some p -> p.magnitude
    None -> 0.0
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected end-to-end check to pass");
    let mag = res.defs.iter().find(|d| d.name == "magOrZero").unwrap();
    let result = &typing.schemes[&mag.id].ty;
    match result {
        Ty::Fun(params, ret) => {
            assert_eq!(params.len(), 1);
            assert_eq!(ret.as_ref(), &Ty::Prim(PrimTy::Float));
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test check_end_to_end`
Expected: PASS (everything should be in place from Tasks 1–25).

If it fails — diagnose, fix the root cause (don't soften the test), and rerun. Common surprises: method's first `self` parameter not unified with the receiver, exhaustiveness check seeing only one of two variants, generalisation tripping over `self` types.

- [ ] **Step 5: Commit**

```bash
git add tests/check_end_to_end.rs
git commit -m "Plan 4 Task 26: end-to-end small-program typing test"
```

---

## Task 27: Document the type checker

**Files:**
- Create: `docs/checker.md`
- Modify: `docs/README.md` (add link)
- Modify: `README.md` (Status line: add type checker to completed list)
- Modify: `docs/superpowers/plans/PROGRESS.md` (add Phase 4 section, remove `Type checker — Plan 4 (TBD)` from Later v1 phases)

- [ ] **Step 1: Draft the doc**

`docs/checker.md` covers: what it does, the `Ty`/`Scheme` model, the two-phase top-level inference, generalisation rules, record/sum/method handling, the registry, exhaustiveness, errors, and what's deferred (traits → Plan 5; effects → Plan 6; totality → Plan 7). Mirror the structure of `docs/resolution.md`.

- [ ] **Step 2: Link from `docs/README.md`**

Add under Reference (random access):
```
- [Type checking](checker.md) — Hindley-Milner inference and exhaustiveness
```

- [ ] **Step 3: Refresh the top-level Status line**

In `README.md`:
> Docs are complete; lexer, parser, name resolution, and the HM core of the type checker are complete and tested. Traits, effects, and totality are next.

- [ ] **Step 4: Add Phase 4 section to `PROGRESS.md`**

```markdown
## Phase 4: Implementation — Plan 4 (type checker, HM core) DONE
- [x] Scaffold + Ty/Scheme/Subst + unification (Tasks 1-3)
- [x] Inference context, literals, variables, lambdas, applications (Tasks 4-9)
- [x] Blocks, generalisation, annotations (Tasks 10-11)
- [x] Type registry, newtypes, records, sums (Tasks 12-13)
- [x] Construction, update, field access, methods, constructors (Tasks 14-16)
- [x] Patterns and match with exhaustiveness (Tasks 17-21)
- [x] Primitive operators and list literals (Tasks 22-23)
- [x] Pretty-printing, corpus snapshots, end-to-end test (Tasks 24-26)
- [x] Documentation (Task 27)
```

Remove `- [ ] Type checker — Plan 4 (TBD)` from Later v1 phases.

- [ ] **Step 5: Commit**

```bash
git add docs/checker.md docs/README.md README.md docs/superpowers/plans/PROGRESS.md
git commit -m "Plan 4 Task 27: document the type checker"
```

---

## Self-review checklist (done before handing off)

- Each task starts with a failing test, ends with a commit. ✓
- File paths are concrete (no `<TBD>` slots). ✓
- Errors are added in the task that first needs them, not in a batch at the end. ✓
- Decisions baked in match what the spec promises (`types.md § 2`, § 5, § 7). ✓
- Deferred work is explicitly attributed to the right downstream plan. ✓
- Method-vs-field decision (decision 8) is implemented in Task 15 and tested. ✓
- Generalisation lives at top level only (decision 3) — block-locals are monomorphic, tested in Task 10. ✓
- Operator dispatch is a placeholder, not a real abstraction — Plan 5 will rip it out. ✓
