# Plan 5: Traits + operator desugaring — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Plan 4's hardcoded operator typing with trait-based ad-hoc polymorphism: operators dispatch through built-in traits, user types become operator-usable via `impl`, and generic functions infer trait-constrained types like `Eq a => List a -> Bool`.

**Architecture:** Type-checking only (no runtime — Plan 8). The operator traits are a built-in `TraitId` enum (no prelude exists yet); a `Constraint` rides in each `Scheme`; inference collects constraints, a per-SCC solver discharges concrete ones against an impl table and retains generalisable ones in the scheme. Operators stay as `BinOp`/`UnaryOp` AST nodes — dispatch happens in the checker, not by rewriting the tree.

**Tech Stack:** Rust, hand-written compiler. Tests: `cargo test`, `insta` snapshots. Source under `src/check/`.

**Spec:** `docs/superpowers/specs/2026-05-26-plan-5-traits-design.md`. Read it before starting.

---

## File structure

- **`src/check/traits.rs`** (NEW) — the built-in trait knowledge: `TraitId` enum, operator→trait mapping, per-trait method-name set, per-trait result-shape, and the synthetic primitive impl seeds. Pure data + pure functions; no inference state. Keeps trait *facts* out of the inference engine.
- **`src/check/types.rs`** (MODIFY) — add `Constraint`; add `constraints` field to `Scheme`; add the registry-aware `ty_to_string` / `scheme_to_string` / typing renderer used by errors and snapshots.
- **`src/check/registry.rs`** (MODIFY) — add `TypeHead`, `ImplInfo`, the `impls` map on `TypeRegistry`, and `head_of`.
- **`src/check/infer.rs`** (MODIFY) — `Infer` gains an ambient `constraints` list; `require_trait` helper; `instantiate` carries scheme constraints to the ambient set; `infer_binop`/`infer_unaryop` dispatch through traits; error messages use friendly names.
- **`src/check/mod.rs`** (MODIFY) — build the impl table from `ImplDecl` + coherence checks; per-SCC constraint solver; attach retained constraints during generalisation.
- **`src/error.rs`** (MODIFY) — new `ErrorKind` variants.
- **`tests/check_traits.rs`** (NEW) — integration tests through `check_file`.
- **`tests/check_corpus.rs`** (MODIFY) + **`tests/corpus/check/*.i`** (NEW) — trait/impl/constrained fixtures; snapshots rendered with friendly names.
- **`tests/check_end_to_end.rs`** (MODIFY) — a program exercising an `impl` through an operator and a generic helper.
- **`docs/checker.md`**, **`docs/superpowers/plans/PROGRESS.md`**, **`README.md`** (MODIFY) — documentation.

## Testing strategy

TDD per task: failing test first, confirm it fails, minimal implementation, confirm green. Unit tests inline in the module they exercise (`#[cfg(test)] mod tests`); integration tests through the public `check_file` API in `tests/`; snapshot tests in `tests/`. After green + `make ci`, run the per-task code review (CLAUDE.md) before committing. Snapshot acceptance (`cargo insta review`) is the user's interactive checkpoint — generate and hand off, don't accept on their behalf.

Per CLAUDE.md, commit only after the user has seen the review and said go. Commit headline `Plan 5 Task N: <verb-led description>`; trailer `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.

---

### Task 1: Built-in trait knowledge (`TraitId`)

**Files:**
- Create: `src/check/traits.rs`
- Modify: `src/check/mod.rs:1-7` (add `pub mod traits;`)

- [ ] **Step 1: Write the failing test**

In `src/check/traits.rs`:

```rust
use crate::ast::{BinOp, UnaryOp};

/// The built-in operator traits. Intrinsic in Plan 5 — there is no prelude
/// declaring them yet (Plan 9). Operators are the only thing that names a
/// trait in this plan, so this small closed set is the whole trait universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitId {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Neg,
    Eq,
    Ord,
    Concat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binop_maps_to_trait() {
        assert_eq!(TraitId::of_binop(&BinOp::Add), Some(TraitId::Add));
        assert_eq!(TraitId::of_binop(&BinOp::Lt), Some(TraitId::Ord));
        assert_eq!(TraitId::of_binop(&BinOp::Eq), Some(TraitId::Eq));
        assert_eq!(TraitId::of_binop(&BinOp::Concat), Some(TraitId::Concat));
        // and/or/xor are Std.Bool functions, not trait operators (spec §11).
        assert_eq!(TraitId::of_binop(&BinOp::And), None);
    }

    #[test]
    fn result_is_bool_for_comparisons_else_operand() {
        assert!(TraitId::Eq.result_is_bool());
        assert!(TraitId::Ord.result_is_bool());
        assert!(!TraitId::Add.result_is_bool());
    }

    #[test]
    fn name_and_methods_are_known() {
        assert_eq!(TraitId::Eq.name(), "Eq");
        assert_eq!(TraitId::Eq.method_names(), &["eq", "ne"]);
        assert_eq!(TraitId::Add.method_names(), &["add"]);
    }

    #[test]
    fn trait_id_parses_from_name() {
        assert_eq!(TraitId::from_name("Ord"), Some(TraitId::Ord));
        assert_eq!(TraitId::from_name("Nope"), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Add `pub mod traits;` after `pub mod registry;` in `src/check/mod.rs` first.
Run: `cargo test -p i-lang --lib check::traits`
Expected: FAIL — `of_binop`, `result_is_bool`, `name`, `method_names`, `from_name` not found.

- [ ] **Step 3: Write minimal implementation**

Append to `src/check/traits.rs` (above the test module):

```rust
impl TraitId {
    /// The trait an infix operator dispatches to. `and`/`or`/`xor` are Bool
    /// functions, not trait operators, so they return `None`.
    pub fn of_binop(op: &BinOp) -> Option<TraitId> {
        Some(match op {
            BinOp::Add => TraitId::Add,
            BinOp::Sub => TraitId::Sub,
            BinOp::Mul => TraitId::Mul,
            BinOp::Div => TraitId::Div,
            BinOp::Pow => TraitId::Pow,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => TraitId::Ord,
            BinOp::Eq | BinOp::Ne => TraitId::Eq,
            BinOp::Concat => TraitId::Concat,
            BinOp::And | BinOp::Or | BinOp::Xor => return None,
        })
    }

    /// The trait a prefix operator dispatches to. `not` is a Bool function.
    pub fn of_unaryop(op: &UnaryOp) -> Option<TraitId> {
        match op {
            UnaryOp::Neg => Some(TraitId::Neg),
            UnaryOp::Not => None,
        }
    }

    /// Comparisons return `Bool`; every other operator trait returns its
    /// operand type.
    pub fn result_is_bool(&self) -> bool {
        matches!(self, TraitId::Eq | TraitId::Ord)
    }

    pub fn name(&self) -> &'static str {
        match self {
            TraitId::Add => "Add",
            TraitId::Sub => "Sub",
            TraitId::Mul => "Mul",
            TraitId::Div => "Div",
            TraitId::Pow => "Pow",
            TraitId::Neg => "Neg",
            TraitId::Eq => "Eq",
            TraitId::Ord => "Ord",
            TraitId::Concat => "Concat",
        }
    }

    pub fn from_name(name: &str) -> Option<TraitId> {
        Some(match name {
            "Add" => TraitId::Add,
            "Sub" => TraitId::Sub,
            "Mul" => TraitId::Mul,
            "Div" => TraitId::Div,
            "Pow" => TraitId::Pow,
            "Neg" => TraitId::Neg,
            "Eq" => TraitId::Eq,
            "Ord" => TraitId::Ord,
            "Concat" => TraitId::Concat,
            _ => return None,
        })
    }

    /// The method names a complete `impl` of this trait must define — used for
    /// the exact-method-set coherence check.
    pub fn method_names(&self) -> &'static [&'static str] {
        match self {
            TraitId::Add => &["add"],
            TraitId::Sub => &["sub"],
            TraitId::Mul => &["mul"],
            TraitId::Div => &["div"],
            TraitId::Pow => &["pow"],
            TraitId::Neg => &["neg"],
            TraitId::Eq => &["eq", "ne"],
            TraitId::Ord => &["lt", "le", "gt", "ge"],
            TraitId::Concat => &["concat"],
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang --lib check::traits`
Expected: PASS (4 tests).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/traits.rs src/check/mod.rs
git commit   # headline: "Plan 5 Task 1: built-in operator-trait knowledge"
```

---

### Task 2: `Constraint` and `Scheme.constraints`

**Files:**
- Modify: `src/check/types.rs:25-29` (Scheme), add `Constraint`
- Modify: `src/check/mod.rs` (every `Scheme { vars, ty }` literal — lines ~99-105, ~145-151, ~494-500, ~580-583)
- Modify: `src/check/infer.rs` (test-module `Scheme { .. }` literals ~932, ~972)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/check/types.rs`:

```rust
#[test]
fn scheme_carries_constraints() {
    let s = Scheme {
        vars: vec![TyVarId(0)],
        constraints: vec![Constraint {
            trait_: crate::check::traits::TraitId::Eq,
            ty: Ty::Var(TyVarId(0)),
        }],
        ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Bool))),
    };
    assert_eq!(s.constraints.len(), 1);
    assert_eq!(s.constraints[0].trait_, crate::check::traits::TraitId::Eq);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib check::types`
Expected: FAIL — no field `constraints` on `Scheme`; no `Constraint`.

- [ ] **Step 3: Write minimal implementation**

In `src/check/types.rs`, add after the `Scheme` definition and update the struct:

```rust
use crate::check::traits::TraitId;

/// "`ty` must implement `trait_`." The sole obligation kind in Plan 5.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub trait_: TraitId,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: Vec<TyVarId>,
    pub constraints: Vec<Constraint>,
    pub ty: Ty,
}
```

Then add `constraints: Vec::new(),` to every `Scheme { .. }` constructor in `src/check/mod.rs` and `src/check/infer.rs`. Search: `rg "Scheme \{" src/check`. Each existing literal becomes e.g.:

```rust
Scheme {
    vars: Vec::new(),
    constraints: Vec::new(),
    ty: Ty::Var(v),
}
```

(Display for `Scheme` is left unchanged here — constraint rendering arrives in Task 10.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang`
Expected: PASS — the new test plus the entire existing suite (the field is additive, all schemes default to no constraints).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/types.rs src/check/mod.rs src/check/infer.rs
git commit   # "Plan 5 Task 2: add Constraint and Scheme.constraints"
```

---

### Task 3: `TypeHead`, `ImplInfo`, impl table, `head_of`

**Files:**
- Modify: `src/check/registry.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` to `src/check/registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::traits::TraitId;
    use crate::check::types::{PrimTy, Ty};
    use crate::resolve::DefId;

    #[test]
    fn head_of_classifies_types() {
        assert_eq!(head_of(&Ty::Prim(PrimTy::Int)), Some(TypeHead::Prim(PrimTy::Int)));
        assert_eq!(head_of(&Ty::Con(DefId(3), vec![])), Some(TypeHead::Con(DefId(3))));
        assert_eq!(head_of(&Ty::Var(crate::check::types::TyVarId(0))), None);
    }

    #[test]
    fn impl_table_keys_on_trait_and_head() {
        let mut reg = TypeRegistry::default();
        reg.impls.insert(
            (TraitId::Eq, TypeHead::Con(DefId(3))),
            ImplInfo { trait_: TraitId::Eq, head: TypeHead::Con(DefId(3)) },
        );
        assert!(reg.impls.contains_key(&(TraitId::Eq, TypeHead::Con(DefId(3)))));
        assert!(!reg.impls.contains_key(&(TraitId::Eq, TypeHead::Prim(PrimTy::Int))));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib check::registry`
Expected: FAIL — `TypeHead`, `ImplInfo`, `head_of`, `impls` not found.

- [ ] **Step 3: Write minimal implementation**

In `src/check/registry.rs`, add the imports and types, and the `impls` field:

```rust
use crate::check::traits::TraitId;
use crate::check::types::PrimTy;

/// The "head" of a type — what an impl matches on. Primitives have no DefId,
/// so the head unifies the primitive and nominal cases under one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeHead {
    Prim(PrimTy),
    Con(DefId),
}

#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub trait_: TraitId,
    pub head: TypeHead,
}

/// The matchable head of a resolved type, or `None` for a type variable /
/// function type (neither can have an impl in Plan 5).
pub fn head_of(ty: &Ty) -> Option<TypeHead> {
    match ty {
        Ty::Prim(p) => Some(TypeHead::Prim(*p)),
        Ty::Con(id, _) => Some(TypeHead::Con(*id)),
        Ty::Var(_) | Ty::Fun(..) => None,
    }
}
```

Add to `TypeRegistry` (and it stays `Default`):

```rust
#[derive(Debug, Default, Clone)]
pub struct TypeRegistry {
    pub types: HashMap<DefId, TypeDeclInfo>,
    pub ctor_to_type: HashMap<DefId, DefId>,
    pub impls: HashMap<(TraitId, TypeHead), ImplInfo>,
}
```

`PrimTy` already derives `Hash, Eq` (`src/check/types.rs:8`), so `TypeHead` deriving them compiles.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang --lib check::registry`
Expected: PASS (2 tests).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/registry.rs
git commit   # "Plan 5 Task 3: impl table, TypeHead, head_of"
```

---

### Task 4: Synthetic primitive impls

**Files:**
- Modify: `src/check/traits.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/check/traits.rs` tests:

```rust
#[test]
fn builtin_impls_cover_primitive_arithmetic_and_eq() {
    use crate::check::registry::{TypeHead, TypeRegistry};
    use crate::check::types::PrimTy;
    let mut reg = TypeRegistry::default();
    seed_builtin_impls(&mut reg);
    assert!(reg.impls.contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::Int))));
    assert!(reg.impls.contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::Float))));
    assert!(reg.impls.contains_key(&(TraitId::Eq, TypeHead::Prim(PrimTy::Int))));
    assert!(reg.impls.contains_key(&(TraitId::Ord, TypeHead::Prim(PrimTy::Float))));
    assert!(reg.impls.contains_key(&(TraitId::Concat, TypeHead::Prim(PrimTy::String))));
    // No arithmetic on String.
    assert!(!reg.impls.contains_key(&(TraitId::Add, TypeHead::Prim(PrimTy::String))));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib check::traits`
Expected: FAIL — `seed_builtin_impls` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `src/check/traits.rs`:

```rust
use crate::check::registry::{ImplInfo, TypeHead, TypeRegistry};
use crate::check::types::PrimTy;

/// Seed the impl table with the prelude impls on primitive types. Built-in
/// until Plan 9's `prelude.i` supplies them in source. `Eq`/`Ord` on every
/// primitive; numeric traits on Int and Float; `Concat` on String.
pub fn seed_builtin_impls(reg: &mut TypeRegistry) {
    use PrimTy::*;
    use TraitId::*;
    let eq_ord: &[PrimTy] = &[Int, Float, String, Bool, Unit];
    let numeric: &[PrimTy] = &[Int, Float];

    let mut add = |t: TraitId, p: PrimTy, reg: &mut TypeRegistry| {
        let head = TypeHead::Prim(p);
        reg.impls.insert((t, head), ImplInfo { trait_: t, head });
    };
    for &p in eq_ord {
        add(Eq, p, reg);
        add(Ord, p, reg);
    }
    for &p in numeric {
        for t in [Add, Sub, Mul, Div, Pow, Neg] {
            add(t, p, reg);
        }
    }
    add(Concat, String, reg);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang --lib check::traits`
Expected: PASS.

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/traits.rs
git commit   # "Plan 5 Task 4: synthetic primitive impls"
```

---

### Task 5: New error variants

**Files:**
- Modify: `src/error.rs:76-80` (append to `ErrorKind`)

- [ ] **Step 1: Write the failing test**

Add to `src/error.rs` (new `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trait_error_variants_exist() {
        let _ = ErrorKind::MissingImpl { trait_name: "Eq".into(), ty: "Point".into() };
        let _ = ErrorKind::DuplicateImpl { trait_name: "Eq".into(), ty: "Point".into() };
        let _ = ErrorKind::UnknownTrait { name: "Nope".into() };
        let _ = ErrorKind::MissingMethod { trait_name: "Eq".into(), method: "ne".into() };
        let _ = ErrorKind::UnknownMethod { trait_name: "Eq".into(), method: "zz".into() };
        let _ = ErrorKind::AmbiguousConstraint { trait_name: "Eq".into() };
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib error`
Expected: FAIL — variants not found.

- [ ] **Step 3: Write minimal implementation**

Append to the `ErrorKind` enum in `src/error.rs`, before the closing `}`:

```rust
    MissingImpl {
        trait_name: String,
        ty: String,
    },
    DuplicateImpl {
        trait_name: String,
        ty: String,
    },
    UnknownTrait {
        name: String,
    },
    MissingMethod {
        trait_name: String,
        method: String,
    },
    UnknownMethod {
        trait_name: String,
        method: String,
    },
    AmbiguousConstraint {
        trait_name: String,
    },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang --lib error`
Expected: PASS.

- [ ] **Step 5: Code review, then commit**

```bash
git add src/error.rs
git commit   # "Plan 5 Task 5: error variants for traits and coherence"
```

---

### Task 6: Build the impl table from `ImplDecl` + coherence

**Files:**
- Modify: `src/check/mod.rs` (`build_registry`, after the type/method passes near `:592`; add imports for `TraitId`, `head_of`, `TypeHead`)
- Test: `tests/check_traits.rs` (NEW)

- [ ] **Step 1: Write the failing test**

Create `tests/check_traits.rs`:

```rust
use i_lang::check::check_file;
use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

fn errors(src: &str) -> Vec<ErrorKind> {
    let toks = lex(src).expect("lex");
    let file = parse(&toks).expect("parse");
    let res = resolve_file(&file).expect("resolve");
    match check_file(&file, &res) {
        Ok(_) => vec![],
        Err(es) => es.into_iter().map(|e| e.kind).collect(),
    }
}

#[test]
fn impl_of_unknown_trait_errors() {
    let src = "type Point\n    x : Int\nimpl Bogus Point\n    eq = a b -> a\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::UnknownTrait { name } if name == "Bogus")));
}

#[test]
fn duplicate_impl_errors() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::DuplicateImpl { trait_name, .. } if trait_name == "Eq")));
}

#[test]
fn impl_missing_method_errors() {
    // Eq requires eq AND ne; provide only eq.
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::MissingMethod { method, .. } if method == "ne")));
}

#[test]
fn impl_unknown_method_errors() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\n    zz = a b -> a\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::UnknownMethod { method, .. } if method == "zz")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_traits`
Expected: FAIL — no impl processing yet, so none of these errors are produced.

- [ ] **Step 3: Write minimal implementation**

Add imports near the top of `src/check/mod.rs`:

```rust
use crate::check::registry::{head_of, ImplInfo, TypeHead};
use crate::check::traits::TraitId;
```

Add a new pass at the end of `build_registry` (after the method second-pass loop, before the closing `}`), then seed the builtins:

```rust
    // Impl declarations. Each `impl Trait Type` registers (TraitId, head) in
    // the impl table after coherence checks. User impls of built-in traits are
    // how a user type (e.g. Point) becomes usable with operators.
    for decl in &file.decls {
        let DeclKind::ImplDecl { trait_name, target, methods } = &decl.node else {
            continue;
        };
        let Some(trait_) = TraitId::from_name(trait_name) else {
            infer.errors.push(Error {
                span: decl.span,
                kind: ErrorKind::UnknownTrait { name: trait_name.clone() },
            });
            continue;
        };
        let target_ty = infer.lower_type(target);
        let Some(head) = head_of(&target_ty) else {
            // A type variable or function type can't be an impl target.
            infer.errors.push(Error {
                span: decl.span,
                kind: ErrorKind::UnknownType { name: trait_name.clone() },
            });
            continue;
        };

        // Coherence: at most one impl per (trait, type).
        if infer.registry.impls.contains_key(&(trait_, head)) {
            infer.errors.push(Error {
                span: decl.span,
                kind: ErrorKind::DuplicateImpl {
                    trait_name: trait_.name().to_string(),
                    ty: format!("{target_ty}"),
                },
            });
            continue;
        }

        // Exact method set: every required method present, no extras.
        let required = trait_.method_names();
        let provided: Vec<&str> = methods
            .iter()
            .filter_map(|m| match &m.node {
                DeclKind::Binding { name, value: Some(_), .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        for req in required {
            if !provided.contains(req) {
                infer.errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::MissingMethod {
                        trait_name: trait_.name().to_string(),
                        method: req.to_string(),
                    },
                });
            }
        }
        for p in &provided {
            if !required.contains(p) {
                infer.errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::UnknownMethod {
                        trait_name: trait_.name().to_string(),
                        method: p.to_string(),
                    },
                });
            }
        }

        infer.registry.impls.insert((trait_, head), ImplInfo { trait_, head });
    }

    crate::check::traits::seed_builtin_impls(&mut infer.registry);
```

> Note: impl method *bodies* are walked by the resolver but are not type-checked against the trait signature in Plan 5 — the spec defers method-body/signature checking (no `Trait.method` machinery yet). This pass checks the method *set* only.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test check_traits && cargo test -p i-lang`
Expected: PASS — the four new tests, and the existing suite (seeding builtins doesn't affect programs with no operators-on-unimpl'd-types yet, because operator dispatch isn't wired until Task 8).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/mod.rs tests/check_traits.rs
git commit   # "Plan 5 Task 6: build impl table with coherence checks"
```

---

### Task 7: Ambient constraints + constrained instantiation

**Files:**
- Modify: `src/check/infer.rs` (`Infer` struct + `new`; `instantiate`; its callers at `:142`, `:154`, `:613`)

- [ ] **Step 1: Write the failing test**

Add to `src/check/infer.rs` tests:

```rust
#[test]
fn instantiating_constrained_scheme_records_ambient_constraint() {
    use crate::check::traits::TraitId;
    let res = Resolution::default();
    let mut infer = Infer::new(&res);
    let a = infer.fresh();
    let scheme = Scheme {
        vars: vec![a],
        constraints: vec![Constraint { trait_: TraitId::Eq, ty: Ty::Var(a) }],
        ty: Ty::Fun(vec![Ty::Var(a)], Box::new(Ty::Prim(PrimTy::Bool))),
    };
    let span = Span::new(0, 1);
    let _ = infer.instantiate(scheme, span);
    // The Eq constraint is carried into the ambient set, with `a` renamed to a
    // fresh var (so it is no longer the original `a`).
    assert_eq!(infer.constraints.len(), 1);
    assert_eq!(infer.constraints[0].0.trait_, TraitId::Eq);
    assert_ne!(infer.constraints[0].0.ty, Ty::Var(a));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib check::infer::tests::instantiating_constrained`
Expected: FAIL — `infer.constraints` field absent; `instantiate` takes one arg.

- [ ] **Step 3: Write minimal implementation**

Add the field to `Infer` (`src/check/infer.rs:17-27`) and initialise it in `new`:

```rust
    pub constraints: Vec<(Constraint, Span)>,
```

```rust
            constraints: Vec::new(),
```

Import `Constraint` (extend the `types` use at `:3`): `use crate::check::types::{Constraint, PrimTy, Scheme, Subst, Ty, TyVarId, Typing};`

Change `instantiate` to take a span and carry constraints:

```rust
    pub fn instantiate(&mut self, scheme: Scheme, span: Span) -> Ty {
        let mut s: Subst = HashMap::new();
        for v in &scheme.vars {
            let fresh = self.fresh();
            s.insert(*v, Ty::Var(fresh));
        }
        for c in &scheme.constraints {
            self.constraints.push((
                Constraint { trait_: c.trait_, ty: apply_subst(&c.ty, &s) },
                span,
            ));
        }
        apply_subst(&scheme.ty, &s)
    }
```

Update the three call sites to pass a span:
- `:142` (Var): `self.instantiate(scheme, e.span)`
- `:154` (Ctor): `self.instantiate(scheme, e.span)`
- `:613` (`infer_ctor_pattern`): `self.instantiate(scheme, p.span)`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang`
Expected: PASS — new test plus existing suite (ctor/var schemes have no constraints today, so the ambient set stays empty for them).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/infer.rs
git commit   # "Plan 5 Task 7: ambient constraints, constrained instantiation"
```

---

### Task 8: Operator dispatch through traits

**Files:**
- Modify: `src/check/infer.rs` (`infer_binop:279`, `infer_unaryop:315`; remove now-unused `require_numeric` if nothing else uses it — keep `unify_operands`/`expect_operand`)
- Test: `tests/check_traits.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/check_traits.rs` a helper and tests asserting the recorded constraints. Add at top:

```rust
use i_lang::check::traits::TraitId;
use i_lang::check::types::{PrimTy, Ty};

fn check_ok(src: &str) -> i_lang::check::Typing {
    let toks = lex(src).expect("lex");
    let file = parse(&toks).expect("parse");
    let res = resolve_file(&file).expect("resolve");
    check_file(&file, &res).expect("check")
}
```

```rust
#[test]
fn int_addition_still_types_as_int() {
    // Operator now dispatches via Add, but 3 + 4 must still come out Int and
    // type-check (Add Int is a built-in impl).
    let t = check_ok("main = 3 + 4\n");
    let scheme = t.schemes.values().next().expect("a scheme");
    assert_eq!(scheme.ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn comparison_still_types_as_bool() {
    let t = check_ok("main = 3 < 4\n");
    let scheme = t.schemes.values().next().unwrap();
    assert_eq!(scheme.ty, Ty::Prim(PrimTy::Bool));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_traits`
Expected: these two may already pass for `+`/`<` via the OLD hardcoded path. To force the new path, first delete the `BinOp::Add | ...` numeric arms' bodies in Step 3, then they exercise dispatch. Run after Step 3. (If they pass pre-change, that is fine — Step 3 must keep them green while routing through traits, verified by Task 9's discharge tests.)

- [ ] **Step 3: Write minimal implementation**

Replace `infer_binop` (`src/check/infer.rs:279-313`) with trait dispatch:

```rust
    fn infer_binop(&mut self, e: &Expr, op: &BinOp, lhs: &Expr, rhs: &Expr) -> Ty {
        let lhs_ty = self.infer_expr(lhs);
        let rhs_ty = self.infer_expr(rhs);
        // Operands must agree; unify the two sides first for every operator.
        self.unify_operands(&lhs_ty, &rhs_ty, e.span);

        match crate::check::traits::TraitId::of_binop(op) {
            Some(trait_) => {
                let operand = apply_subst(&lhs_ty, &self.subst);
                // Record the obligation; the per-SCC solver discharges it
                // against the impl table (or retains it on the scheme).
                self.constraints.push((
                    crate::check::types::Constraint { trait_, ty: operand.clone() },
                    e.span,
                ));
                if trait_.result_is_bool() {
                    Ty::Prim(PrimTy::Bool)
                } else {
                    operand
                }
            }
            // and / or / xor: Bool functions, not trait operators.
            None => {
                self.expect_operand(&lhs_ty, &Ty::Prim(PrimTy::Bool), lhs.span);
                self.expect_operand(&rhs_ty, &Ty::Prim(PrimTy::Bool), rhs.span);
                Ty::Prim(PrimTy::Bool)
            }
        }
    }
```

Replace `infer_unaryop` (`:315-331`):

```rust
    fn infer_unaryop(&mut self, e: &Expr, op: &UnaryOp, inner: &Expr) -> Ty {
        let inner_ty = self.infer_expr(inner);
        match crate::check::traits::TraitId::of_unaryop(op) {
            Some(trait_) => {
                let operand = apply_subst(&inner_ty, &self.subst);
                self.constraints.push((
                    crate::check::types::Constraint { trait_, ty: operand.clone() },
                    e.span,
                ));
                operand
            }
            None => {
                // `not`: Bool function.
                self.expect_operand(&inner_ty, &Ty::Prim(PrimTy::Bool), e.span);
                Ty::Prim(PrimTy::Bool)
            }
        }
    }
```

Delete `require_numeric` (`:377-394`) — no longer referenced. `clippy -D warnings` will flag it if left.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang`
Expected: PASS for the two new tests. **Note:** `tests/check_literals.rs::mixed_int_float_addition_errors` still passes (unifying `Int` with `Float` fails in `unify_operands`). `heterogeneous_list_errors` unaffected. The Plan-4 test `mixed_int_float_addition_errors` expecting a `TypeMismatch` still gets one from the operand unify. If any literal test asserted the *old* "Int or Float" message text, update it to the unification mismatch — check `tests/check_literals.rs` and adjust assertions that match on the removed `require_numeric` message.

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/infer.rs tests/check_traits.rs
git commit   # "Plan 5 Task 8: dispatch operators through traits"
```

---

### Task 9: Per-SCC constraint solving

**Files:**
- Modify: `src/check/mod.rs` (SCC loop, after generalisation block `:176-192`; add a solve step)
- Test: `tests/check_traits.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/check_traits.rs`:

```rust
#[test]
fn operator_on_type_without_impl_errors() {
    // Point has no Eq impl; pt == pt must fail with MissingImpl.
    let src = "type Point\n    x : Int\np : Point\np = Point(x = 1)\nb = p == p\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::MissingImpl { trait_name, .. } if trait_name == "Eq")));
}

#[test]
fn operator_on_type_with_impl_ok() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\np : Point\np = Point(x = 1)\nb = p == p\n";
    assert!(errors(src).is_empty(), "expected clean, got {:?}", errors(src));
}

#[test]
fn generic_equality_helper_infers_eq_constraint() {
    // bothEq compares its two args; its scheme should carry Eq on the param var.
    let src = "bothEq = a b -> a == b\n";
    let t = check_ok(src);
    let scheme = t.schemes.values().find(|s| matches!(s.ty, Ty::Fun(..))).unwrap();
    assert_eq!(scheme.constraints.len(), 1, "scheme: {:?}", scheme);
    assert_eq!(scheme.constraints[0].trait_, TraitId::Eq);
    // The constrained var is one of the generalised vars.
    if let Ty::Var(v) = scheme.constraints[0].ty {
        assert!(scheme.vars.contains(&v));
    } else {
        panic!("expected constraint on a type var, got {:?}", scheme.constraints[0].ty);
    }
}

#[test]
fn unsatisfiable_monomorphic_constraint_is_ambiguous() {
    // A block-local lambda is monomorphic; its Eq'd param var never resolves
    // and never generalises, so the constraint is ambiguous.
    let src = "main =\n    f = x -> x == x\n    0\n";
    assert!(errors(src)
        .iter()
        .any(|k| matches!(k, ErrorKind::AmbiguousConstraint { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_traits`
Expected: FAIL — no solver yet, so MissingImpl/AmbiguousConstraint aren't raised and the generic helper's scheme has no constraints.

- [ ] **Step 3: Write minimal implementation**

In `src/check/mod.rs`, record the ambient-constraint watermark *before* the SCC's bodies are inferred. At the start of the per-SCC body-inference (just before the `for (idx, &i) in scc.iter().enumerate()` loop at `:115`):

```rust
        let constraints_start = infer.constraints.len();
```

After the generalisation block (after `:192`, still inside `for scc in &sccs`), add the solver:

```rust
        // Solve the constraints emitted while inferring this SCC's bodies.
        // Concrete-headed constraints discharge against the impl table now;
        // constraints on a generalised var attach to that var's scheme;
        // anything else is ambiguous.
        let mine_by_scheme: Vec<(DefId, Vec<TyVarId>)> = scc
            .iter()
            .map(|&i| {
                let id = bindings[i].0;
                (id, infer.schemes[&id].vars.clone())
            })
            .collect();

        let pending: Vec<(Constraint, Span)> = infer.constraints.split_off(constraints_start);
        for (c, span) in pending {
            let resolved = apply_subst(&c.ty, &infer.subst);
            match crate::check::registry::head_of(&resolved) {
                Some(head) => {
                    if !infer.registry.impls.contains_key(&(c.trait_, head)) {
                        infer.errors.push(Error {
                            span,
                            kind: ErrorKind::MissingImpl {
                                trait_name: c.trait_.name().to_string(),
                                ty: format!("{resolved}"),
                            },
                        });
                    }
                }
                None => {
                    // A type variable. Attach to whichever SCC scheme generalises it.
                    let var = match resolved {
                        Ty::Var(v) => Some(v),
                        _ => None,
                    };
                    let mut attached = false;
                    if let Some(v) = var {
                        for (id, mine) in &mine_by_scheme {
                            if mine.contains(&v) {
                                if let Some(s) = infer.schemes.get_mut(id) {
                                    s.constraints.push(Constraint { trait_: c.trait_, ty: Ty::Var(v) });
                                }
                                attached = true;
                            }
                        }
                    }
                    if !attached {
                        infer.errors.push(Error {
                            span,
                            kind: ErrorKind::AmbiguousConstraint {
                                trait_name: c.trait_.name().to_string(),
                            },
                        });
                    }
                }
            }
        }
```

Add `Constraint` to the `types::*` glob already imported via `pub use types::*;` at `src/check/mod.rs:7` (it re-exports `Constraint` once Task 2 is in). If the compiler reports `Constraint` unresolved, add `use crate::check::types::Constraint;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang`
Expected: PASS — all four new tests, plus the whole prior suite. The `bothEq` scheme now reads `forall a . Eq a => a, a -> Bool` internally (constraint attached). The corpus snapshots from Plan 4 are unaffected (those programs use no operators on type vars, so no constraints attach).

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/mod.rs tests/check_traits.rs
git commit   # "Plan 5 Task 9: per-SCC constraint solving and discharge"
```

---

### Task 10: Friendly type names in output

**Files:**
- Modify: `src/check/types.rs` (add `ty_to_string`, `scheme_to_string`, `render_typing` taking `&Resolution`; render constraints)
- Modify: `src/check/infer.rs` (error messages: replace `format!("{ty}")`/`format!("{resolved}")` with friendly names where a `Con` may appear — `require_numeric` is gone; `infer_field_access` `CannotAccessMember` at `:524`)
- Modify: `src/check/mod.rs` (`MissingImpl`/`DuplicateImpl` `ty` strings use friendly names)

- [ ] **Step 1: Write the failing test**

Add to `src/check/types.rs` tests:

```rust
#[test]
fn ty_to_string_uses_resolved_name_for_con() {
    use crate::resolve::{DefInfo, DefKind, Resolution};
    use crate::span::Span;
    let mut res = Resolution::default();
    res.defs.push(DefInfo {
        id: DefId(0),
        name: "Maybe".into(),
        kind: DefKind::Type,
        span: Span::new(0, 0),
    });
    let ty = Ty::Con(DefId(0), vec![Ty::Prim(PrimTy::Int)]);
    assert_eq!(ty_to_string(&ty, &res), "Maybe Int");
}

#[test]
fn scheme_to_string_includes_constraints() {
    let res = crate::resolve::Resolution::default();
    let s = Scheme {
        vars: vec![TyVarId(0)],
        constraints: vec![Constraint { trait_: crate::check::traits::TraitId::Eq, ty: Ty::Var(TyVarId(0)) }],
        ty: Ty::Fun(vec![Ty::Var(TyVarId(0))], Box::new(Ty::Prim(PrimTy::Bool))),
    };
    assert_eq!(scheme_to_string(&s, &res), "forall t0 . Eq t0 => t0 -> Bool");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p i-lang --lib check::types`
Expected: FAIL — `ty_to_string`, `scheme_to_string` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `src/check/types.rs` (free functions, alongside the `Display` impls):

```rust
use crate::resolve::Resolution;

/// Render a type using resolved names for constructors. Application uses
/// surface-style spacing: `Maybe Int`, `List a`. Falls back to `#<id>` when
/// the DefId is not in `res.defs` (e.g. a synthetic id in a unit test).
pub fn ty_to_string(ty: &Ty, res: &Resolution) -> String {
    match ty {
        Ty::Var(v) => format!("t{}", v.0),
        Ty::Prim(p) => format!("{}", Ty::Prim(*p)),
        Ty::Con(id, args) => {
            let name = res
                .defs
                .iter()
                .find(|d| d.id == *id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| format!("#{}", id.0));
            if args.is_empty() {
                name
            } else {
                let inner: Vec<String> = args.iter().map(|a| ty_to_string(a, res)).collect();
                format!("{name} {}", inner.join(" "))
            }
        }
        Ty::Fun(ps, r) => {
            let params: Vec<String> = ps.iter().map(|p| ty_to_string(p, res)).collect();
            format!("{} -> {}", params.join(", "), ty_to_string(r, res))
        }
    }
}

pub fn scheme_to_string(s: &Scheme, res: &Resolution) -> String {
    let mut out = String::new();
    if !s.vars.is_empty() {
        let vars: Vec<String> = s.vars.iter().map(|v| format!("t{}", v.0)).collect();
        out.push_str(&format!("forall {} . ", vars.join(" ")));
    }
    if !s.constraints.is_empty() {
        let cs: Vec<String> = s
            .constraints
            .iter()
            .map(|c| format!("{} {}", c.trait_.name(), ty_to_string(&c.ty, res)))
            .collect();
        out.push_str(&format!("{} => ", cs.join(", ")));
    }
    out.push_str(&ty_to_string(&s.ty, res));
    out
}

/// Render a whole `Typing`'s schemes with binding names, sorted by DefId for
/// stable snapshots.
pub fn render_typing(t: &Typing, res: &Resolution) -> String {
    let mut out = String::from("schemes:\n");
    let mut entries: Vec<(&DefId, &Scheme)> = t.schemes.iter().collect();
    entries.sort_by_key(|(id, _)| id.0);
    for (id, scheme) in entries {
        let name = res
            .defs
            .iter()
            .find(|d| d.id == **id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| format!("#{}", id.0));
        out.push_str(&format!("  {name} : {}\n", scheme_to_string(scheme, res)));
    }
    out
}
```

In `src/check/infer.rs`, replace the `CannotAccessMember` `ty` field construction (`:527`) so a `Con` prints friendly:

```rust
                        ty: crate::check::types::ty_to_string(&resolved, self.res),
```

In `src/check/mod.rs`, the `MissingImpl`/`DuplicateImpl` `ty` strings (Tasks 6 and 9) become:

```rust
                ty: crate::check::types::ty_to_string(&resolved, infer.res),
```
(and for `DuplicateImpl` use `ty_to_string(&target_ty, infer.res)`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p i-lang`
Expected: PASS. Existing tests that assert on error *kinds* (not message strings) are unaffected; if any assert on the old `#<id>` message, update to the friendly form.

- [ ] **Step 5: Code review, then commit**

```bash
git add src/check/types.rs src/check/infer.rs src/check/mod.rs
git commit   # "Plan 5 Task 10: friendly type names in errors and rendering"
```

---

### Task 11: Corpus fixtures + friendly snapshots

**Files:**
- Modify: `tests/check_corpus.rs` (render via `render_typing`)
- Create: `tests/corpus/check/operator-primitive.i`, `tests/corpus/check/impl-eq-point.i`, `tests/corpus/check/generic-eq.i`
- Snapshots: re-accepted by the user (`cargo insta review`)

- [ ] **Step 1: Write the failing test (new fixtures + renderer change)**

Change `tests/check_corpus.rs` line 17 to use the friendly renderer:

```rust
            insta::assert_snapshot!(i_lang::check::types::render_typing(&typing, &res));
```

Create the fixtures:

`tests/corpus/check/operator-primitive.i`:
```
addInts = 2 + 3
cmp = 2 < 3
```

`tests/corpus/check/impl-eq-point.i`:
```
type Point
    x : Int
    y : Int
impl Eq Point
    eq = a b -> a.x == b.x
    ne = a b -> not (a.x == b.x)
samePoint = a b -> a == b
```

`tests/corpus/check/generic-eq.i`:
```
bothEq = a b -> a == b
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test check_corpus`
Expected: FAIL — three new pending snapshots, and the six existing snapshots now differ (binding names instead of `#0`, friendly `Con` names). All flagged as snapshot mismatches.

- [ ] **Step 3: Implementation**

No source change — the renderer already exists (Task 10). The "implementation" is the fixtures and the test edit above.

- [ ] **Step 4: Generate snapshots and hand off for review**

Run: `cargo insta test --test check_corpus` (or `cargo test --test check_corpus` to produce `.snap.new` files).
Then **stop and tell the user** to run `cargo insta review`: the snapshots are the interactive checkpoint. Confirm the new fixtures read e.g.:

```
schemes:
  bothEq : forall t0 . Eq t0 => t0, t0 -> Bool
```

and that the existing six now show binding names. Do not accept on the user's behalf.

- [ ] **Step 5: Code review, then commit (after user accepts snapshots)**

```bash
git add tests/check_corpus.rs tests/corpus/check/ tests/snapshots/
git commit   # "Plan 5 Task 11: trait corpus fixtures, friendly snapshots"
```

---

### Task 12: End-to-end test

**Files:**
- Modify: `tests/check_end_to_end.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/check_end_to_end.rs`:

```rust
#[test]
fn program_with_impl_uses_operator_and_generic_helper() {
    let src = "\
type Point
    x : Int
    y : Int
impl Eq Point
    eq = a b -> a.x == b.x and a.y == b.y
    ne = a b -> not (a.x == b.x and a.y == b.y)
samePoint = a b -> a == b
allSame = a b c -> a == b and b == c
";
    let toks = i_lang::lex::lex(src).expect("lex");
    let file = i_lang::parse::parse(&toks).expect("parse");
    let res = i_lang::resolve::resolve_file(&file).expect("resolve");
    let typing = i_lang::check::check_file(&file, &res).expect("check should succeed");

    // samePoint generalises to `Eq a => a, a -> Bool` (used at Point below).
    let same = res.defs.iter().find(|d| d.name == "samePoint").unwrap();
    let scheme = &typing.schemes[&same.id];
    assert_eq!(scheme.constraints.len(), 1);
    assert_eq!(
        scheme.constraints[0].trait_,
        i_lang::check::traits::TraitId::Eq
    );
}
```

- [ ] **Step 2: Run test to verify it fails (or passes)**

Run: `cargo test --test check_end_to_end`
Expected: PASS if Tasks 6–10 are correct — this is an integration guard, so it may pass immediately. If it fails, the failure pinpoints a dispatch/solve gap. Keep it.

- [ ] **Step 3: Implementation**

None — exercises existing behavior end-to-end.

- [ ] **Step 4: Confirm green**

Run: `cargo test -p i-lang && make ci`
Expected: PASS, clean.

- [ ] **Step 5: Code review, then commit**

```bash
git add tests/check_end_to_end.rs
git commit   # "Plan 5 Task 12: end-to-end impl + operator + generic helper"
```

---

### Task 13: Documentation (docs-only — skip code review)

**Files:**
- Modify: `docs/checker.md` (Operators and lists; Pretty-printing; Errors; What's deferred)
- Modify: `docs/superpowers/plans/PROGRESS.md` (Phase 5)
- Modify: `README.md` (Status line)

- [ ] **Step 1: Update `docs/checker.md`**

Rewrite the **Operators and lists** section (`:179-187`) to describe trait dispatch: each arithmetic/ordering/equality/concat operator maps to a built-in `TraitId`, emits a `Constraint` on the operand type, and returns the trait's result shape (`Bool` for `Eq`/`Ord`, operand type otherwise); `and`/`or`/`xor`/`not` remain Bool functions. Add a **Traits** section covering: the built-in `TraitId` set, the impl table keyed by `(TraitId, TypeHead)`, synthetic primitive impls, user impls of built-in traits, coherence (`DuplicateImpl`, exact method set), and the per-SCC constraint solver (discharge concrete / retain generalisable / `AmbiguousConstraint`). Update **Pretty-printing** to note `ty_to_string`/`scheme_to_string`/`render_typing` render friendly names and `forall a . Eq a => ...`. Add the new error rows to the **Errors** table. In **What's deferred**, replace the "Traits and operator dispatch — Plan 5" and "Friendly type names — Plan 5" bullets with: user-declared `trait` blocks + explicit `Trait.method` calls (Plan 9), parameterised/conditional impls, runtime dispatch (Plan 8); keep effects (Plan 6) and totality (Plan 7).

- [ ] **Step 2: Update `PROGRESS.md`**

Change the Phase 4 heading block: add a Phase 5 section mirroring Phase 4's style, marked DONE, linking `2026-05-26-plan-5-traits.md`, with checked task ranges (Tasks 1–13 grouped). Change the "Later v1 phases" line `- [ ] Traits + operator desugaring — Plan 5 (TBD)` to `- [x]`.

- [ ] **Step 3: Update `README.md` status line**

Change `:29-31` to note the type checker now includes traits and operator dispatch; effects and totality are next.

- [ ] **Step 4: Verify**

Run: `make ci`
Expected: PASS (no `.rs` changes; markdown only).

- [ ] **Step 5: Commit (no code-review subagent — docs-only per CLAUDE.md)**

```bash
git add docs/checker.md docs/superpowers/plans/PROGRESS.md README.md
git commit   # "Plan 5 Task 13: document traits and operator dispatch"
```

---

## Self-review notes

- **Spec coverage:** built-in `TraitId` (T1), `Constraint`+`Scheme` (T2), `TypeHead`/impl table/`head_of` (T3), synthetic primitive impls (T4), error variants (T5), impl-table build + coherence/`UnknownTrait`/`Duplicate`/method-set (T6), ambient constraints + instantiation (T7), operator dispatch (T8), solve/discharge/retain/`MissingImpl`/`AmbiguousConstraint` (T9), friendly names (T10), corpus (T11), end-to-end (T12), docs (T13). All design sections map to a task.
- **Deferred, intentionally untasked:** user-declared `trait` blocks, explicit `Trait.method` calls, parameterised impls, runtime dispatch (see design "Explicitly deferred").
- **Type consistency:** `TraitId`, `Constraint { trait_, ty }`, `TypeHead`, `ImplInfo { trait_, head }`, `Infer.constraints: Vec<(Constraint, Span)>`, `instantiate(scheme, span)`, `head_of`, `seed_builtin_impls`, `ty_to_string`/`scheme_to_string`/`render_typing` are named identically across all tasks.
- **Watch item (flagged in tasks, not a blocker):** Task 8 removes `require_numeric`; any Plan-4 test asserting its exact "Int or Float" message must move to the unification-mismatch assertion. Task 8 Step 4 calls this out.
