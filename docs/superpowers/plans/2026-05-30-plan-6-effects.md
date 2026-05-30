# Plan 6: Effects + `?` early-exit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Track effect rows (`! IO`, `! State`, `! Env`) through function types with HOF effect-polymorphism and a strict `!` marker, and type-check the `?` early-exit operator against `Result`/`Maybe`.

**Architecture:** Open effect rows (`{ labels, tail }`) ride on `Ty::Fun`; `Subst` becomes a struct holding a type map and an effect-var map. Effects accumulate into a per-binding ambient row during inference, generalise into schemes alongside type-vars, and unify via a small bounded row-unifier. `?` is independent: the checker tags the `Result`/`Maybe` `DefId`s at registry-build (recognise, don't seed — see the design spec) and checks `?` against a threaded enclosing-return type.

**Tech Stack:** Rust 1.95.0, hand-written HM checker (`src/check/`), insta snapshot tests, `make ci`.

**Source of truth:** `docs/superpowers/specs/2026-05-30-plan-6-effects-design.md`. Read it before starting.

---

## Working rhythm (from CLAUDE.md)

One task at a time. TDD: failing test → run-it-fails → minimal impl → run-it-passes → `make ci` → fresh-subagent code review (skip for docs-only tasks) → report findings → commit on approval → **push** (the commit finalizer). Then a 4-6 line summary and wait for "yes". Identifiers in `i` are camelCase only. Commit headline `Plan 6 Task N: <verb-led>`, trailer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

## File structure

| File | Responsibility | Change |
| --- | --- | --- |
| `src/check/types.rs` | `Ty`, `Scheme`, `Subst`, effect types, rendering | Add `EffectSet`/`EffectVarId`/`EffectRow`; row on `Ty::Fun`; `Subst` struct; `eff_vars` on `Scheme`; render rows |
| `src/check/unify.rs` | unification + substitution | Apply eff-subst to rows; row unification; eff occurs-check |
| `src/check/infer.rs` | inference | Ambient effect row; `Bang`/`Call` effect rules; `?`; enclosing-return stack; effect instantiation; lower effect rows |
| `src/check/mod.rs` | orchestration | Effect generalisation; tag `Result`/`Maybe` in `build_registry` |
| `src/check/registry.rs` | type registry | `builtin: BuiltinTypes { result, maybe }` |
| `src/error.rs` | error kinds | `MissingBang`/`UnnecessaryBang`/`EffectMismatch`/`QuestionContextMismatch`; drop `EffectsNotYetImplemented` |
| `tests/check_effects.rs` | effect integration tests | New |
| `tests/check_question.rs` | `?` integration tests | New |
| `tests/corpus/check/*.i` | corpus fixtures | New fixtures + re-accepted snapshots |

---

### Task 1: Effect row data types

Add the effect alphabet and the open-row type as pure additive code — nothing else references them yet, so the existing suite is untouched.

**Files:**
- Modify: `src/check/types.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/check/types.rs`:

```rust
#[test]
fn effect_set_union_and_membership() {
    let io = EffectSet::single(Effect::Io);
    let state = EffectSet::single(Effect::State);
    assert!(io.contains(Effect::Io));
    assert!(!io.contains(Effect::State));
    let both = io.union(state);
    assert!(both.contains(Effect::Io) && both.contains(Effect::State));
    assert!(EffectSet::empty().is_empty());
    assert!(!io.is_empty());
}

#[test]
fn effect_row_pure_is_concrete_empty() {
    let pure = EffectRow::pure();
    assert!(pure.is_concrete_empty());
    let io = EffectRow::concrete(EffectSet::single(Effect::Io));
    assert!(!io.is_concrete_empty());
    let open = EffectRow::open(EffectSet::empty(), EffectVarId(0));
    assert!(!open.is_concrete_empty()); // a tail var is not provably pure
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib check::types`
Expected: FAIL — `Effect`, `EffectSet`, `EffectRow`, `EffectVarId` not found.

- [ ] **Step 3: Implement the types**

Add near the top of `src/check/types.rs` (after `PrimTy`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    Io,
    State,
    Env,
}

/// A set of concrete effect labels — v1's whole alphabet fits in three bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EffectSet(u8);

impl EffectSet {
    pub fn empty() -> Self {
        EffectSet(0)
    }
    pub fn single(e: Effect) -> Self {
        EffectSet(1 << e as u8)
    }
    pub fn contains(self, e: Effect) -> bool {
        self.0 & (1 << e as u8) != 0
    }
    pub fn union(self, other: EffectSet) -> EffectSet {
        EffectSet(self.0 | other.0)
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    /// Labels present, in fixed order, for stable rendering.
    pub fn iter(self) -> impl Iterator<Item = Effect> {
        [Effect::Io, Effect::State, Effect::Env]
            .into_iter()
            .filter(move |&e| self.contains(e))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectVarId(pub u32);

/// An open effect row: known `labels` plus an optional polymorphic `tail`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectRow {
    pub labels: EffectSet,
    pub tail: Option<EffectVarId>,
}

impl EffectRow {
    pub fn pure() -> Self {
        EffectRow { labels: EffectSet::empty(), tail: None }
    }
    pub fn concrete(labels: EffectSet) -> Self {
        EffectRow { labels, tail: None }
    }
    pub fn open(labels: EffectSet, tail: EffectVarId) -> Self {
        EffectRow { labels, tail: Some(tail) }
    }
    /// Provably pure: no labels and no unknown tail. Drives the strict `!`
    /// rule — a variable tail is NOT provably pure.
    pub fn is_concrete_empty(&self) -> bool {
        self.labels.is_empty() && self.tail.is_none()
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib check::types`
Expected: PASS.

- [ ] **Step 5: Commit and push**

```bash
git add src/check/types.rs
git commit -m "Plan 6 Task 1: effect row data types

Add Effect/EffectSet/EffectVarId/EffectRow — v1's three-label alphabet
as a bitset plus an open row (labels + optional polymorphic tail). Pure
additive types; nothing references them yet.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git push origin main
```

---

### Task 2: Put the row on `Ty::Fun` and make `Subst` a struct

The invasive-but-mechanical task. `Ty::Fun` grows an `EffectRow`; `Subst` becomes a struct with a type map and an effect-var map. Every `Ty::Fun` construction/match (≈32) and every direct `Subst` poke (≈23) updates; `unify`/`apply_subst` *signatures* stay (`&Subst`/`&mut Subst`), so their ~60 call sites are untouched. Existing tests are the safety net; default every constructed row to `EffectRow::pure()` so behaviour is unchanged.

**Files:**
- Modify: `src/check/types.rs`, `src/check/unify.rs`, `src/check/infer.rs`, `src/check/mod.rs`, `src/check/registry.rs` (only if it constructs `Ty::Fun` — it does not; `head_of` matches `Ty::Fun(..)`)

- [ ] **Step 1: Write the failing test**

Add to `src/check/unify.rs` tests:

```rust
#[test]
fn apply_subst_resolves_effect_tail_in_fun_row() {
    use crate::check::types::{Effect, EffectRow, EffectSet, EffectVarId, Subst};
    let mut s = Subst::default();
    s.effs.insert(EffectVarId(0), EffectRow::concrete(EffectSet::single(Effect::Io)));
    let f = Ty::Fun(
        vec![],
        EffectRow::open(EffectSet::empty(), EffectVarId(0)),
        Box::new(Ty::Prim(PrimTy::Unit)),
    );
    let out = apply_subst(&f, &s);
    match out {
        Ty::Fun(_, row, _) => {
            assert!(row.labels.contains(Effect::Io));
            assert!(row.tail.is_none());
        }
        _ => panic!("expected Fun"),
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib check::unify`
Expected: FAIL to compile — `Ty::Fun` takes 2 args, `Subst` has no `.effs`/`.default()`.

- [ ] **Step 3: Change `Ty::Fun` and `Subst`**

In `src/check/types.rs`:

```rust
pub enum Ty {
    Var(TyVarId),
    Prim(PrimTy),
    Con(DefId, Vec<Ty>),
    Fun(Vec<Ty>, EffectRow, Box<Ty>),
}
```

Replace the `Subst` alias with a struct (keep the name so call sites that pass `&subst` are unaffected):

```rust
#[derive(Debug, Default, Clone)]
pub struct Subst {
    pub tys: HashMap<TyVarId, Ty>,
    pub effs: HashMap<EffectVarId, EffectRow>,
}

impl Subst {
    pub fn new() -> Self {
        Subst::default()
    }
}
```

Update `Display for Ty` and `ty_to_string`'s `Fun` arms to ignore the row for now (rendering is Task 8): match `Ty::Fun(ps, _row, r)`.

- [ ] **Step 4: Sweep every `Ty::Fun` site and every `Subst` poke**

Mechanical recipe (apply uniformly):
- **Construction** `Ty::Fun(params, Box::new(result))` → `Ty::Fun(params, EffectRow::pure(), Box::new(result))`. Sites: `infer.rs` (lambda, call `expected`, lambda_checked, method), `mod.rs` (ctor schemes ~`551-560`, method `fun_ty` ~`637`).
- **Match** `Ty::Fun(ps, r)` → `Ty::Fun(ps, _row, r)` in: `types.rs` (Display, ty_to_string, ty_arg_to_string), `unify.rs` (apply_subst, occurs, unify), `mod.rs` (free_vars, the lambda-checked guard at ~`134`), `registry.rs` (`head_of`'s `Ty::Fun(..)` is fine).
- **Subst pokes**: `subst.insert(v, t)` → `subst.tys.insert(v, t)`; `subst.get(v)` → `subst.tys.get(v)`; `subst.is_empty()` → `subst.tys.is_empty()`; `let mut s: Subst = HashMap::new()` → `let mut s = Subst::new()`; in test helpers `HashMap::new()` typed as `Subst` → `Subst::new()`.
- In `apply_subst`, the `Ty::Fun` arm resolves the row tail:

```rust
Ty::Fun(params, row, result) => Ty::Fun(
    params.iter().map(|p| apply_subst(p, subst)).collect(),
    apply_eff_row(row, subst),
    Box::new(apply_subst(result, subst)),
),
```

Add the helper to `unify.rs`:

```rust
/// Resolve an effect row's tail through the substitution, folding any
/// resolved labels into the row. Returns a row whose tail is either None or
/// an unbound effect var.
pub fn apply_eff_row(row: &EffectRow, subst: &Subst) -> EffectRow {
    let mut labels = row.labels;
    let mut tail = row.tail;
    while let Some(v) = tail {
        match subst.effs.get(&v) {
            Some(next) => {
                labels = labels.union(next.labels);
                tail = next.tail;
            }
            None => break,
        }
    }
    EffectRow { labels, tail }
}
```

(`occurs` and `unify`'s `Fun` arm keep matching the row with `_` until Task 3.)

- [ ] **Step 5: Run the full suite**

Run: `cargo test`
Expected: PASS (all existing tests green; the new `apply_subst_resolves_effect_tail_in_fun_row` passes). Then `make ci`.

- [ ] **Step 6: Commit and push**

```bash
git add src/check/
git commit -m "Plan 6 Task 2: effect row on Ty::Fun; Subst becomes a struct

Ty::Fun now carries an EffectRow (defaulting to pure everywhere) and
Subst is a struct holding the type map plus an effect-var map.
apply_subst resolves a Fun's row tail through the new eff map. Pure
mechanical sweep; behaviour unchanged, full suite green.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git push origin main
```

---

### Task 3: Row unification

Teach `unify` to unify the rows when unifying two `Ty::Fun`s, and add the effect occurs-check. Bounded because the label alphabet is closed.

**Files:**
- Modify: `src/check/unify.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn unify_closed_rows_equal_ok_mismatch_errors() {
    use crate::check::types::{Effect, EffectRow, EffectSet};
    let mut s = Subst::new();
    let io = EffectRow::concrete(EffectSet::single(Effect::Io));
    assert!(unify_rows(&mut s, &io, &io.clone()).is_ok());
    let mut s2 = Subst::new();
    assert!(unify_rows(&mut s2, &io, &EffectRow::pure()).is_err());
}

#[test]
fn unify_open_row_solves_tail_to_missing_labels() {
    use crate::check::types::{Effect, EffectRow, EffectSet, EffectVarId};
    let mut s = Subst::new();
    let open = EffectRow::open(EffectSet::empty(), EffectVarId(0));
    let io = EffectRow::concrete(EffectSet::single(Effect::Io));
    unify_rows(&mut s, &open, &io).unwrap();
    let solved = s.effs.get(&EffectVarId(0)).unwrap();
    assert!(solved.labels.contains(Effect::Io) && solved.tail.is_none());
}

#[test]
fn unify_fun_unifies_rows() {
    use crate::check::types::{Effect, EffectRow, EffectSet, EffectVarId};
    let mut s = Subst::new();
    let f_open = Ty::Fun(vec![], EffectRow::open(EffectSet::empty(), EffectVarId(0)), Box::new(Ty::Prim(PrimTy::Unit)));
    let f_io = Ty::Fun(vec![], EffectRow::concrete(EffectSet::single(Effect::Io)), Box::new(Ty::Prim(PrimTy::Unit)));
    unify(&mut s, &f_open, &f_io).unwrap();
    assert!(s.effs.get(&EffectVarId(0)).unwrap().labels.contains(Effect::Io));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib check::unify`
Expected: FAIL — `unify_rows` not found; `unify`'s Fun arm ignores rows.

- [ ] **Step 3: Implement `unify_rows` + eff occurs, wire into `unify`**

Add a `UnifyError::EffectMismatch { expected: EffectRow, found: EffectRow }` variant. Add:

```rust
fn occurs_eff(var: EffectVarId, row: &EffectRow, subst: &Subst) -> bool {
    apply_eff_row(row, subst).tail == Some(var)
}

pub fn unify_rows(subst: &mut Subst, a: &EffectRow, b: &EffectRow) -> Result<(), UnifyError> {
    let a = apply_eff_row(a, subst);
    let b = apply_eff_row(b, subst);
    match (a.tail, b.tail) {
        // Both closed: label sets must match exactly.
        (None, None) => {
            if a.labels == b.labels {
                Ok(())
            } else {
                Err(UnifyError::EffectMismatch { expected: a, found: b })
            }
        }
        // One open: solve its tail to carry the other side's extra labels
        // (and the other side's tail). Guard with an occurs-check.
        (Some(v), _) => bind_eff(subst, v, &b),
        (_, Some(v)) => bind_eff(subst, v, &a),
    }
}

fn bind_eff(subst: &mut Subst, v: EffectVarId, other: &EffectRow) -> Result<(), UnifyError> {
    // Solve v so that {v.labels..} == other. v carries other's labels and tail.
    let solved = other.clone();
    if occurs_eff(v, &solved, subst) {
        // v occurs in its own solution only via itself → already consistent.
        return Ok(());
    }
    subst.effs.insert(v, solved);
    Ok(())
}
```

In `unify`'s `Ty::Fun` arm, after unifying params and result, add `unify_rows(subst, &row1, &row2)?;` (bind the rows in the match: `(Ty::Fun(p1, e1, r1), Ty::Fun(p2, e2, r2))`).

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib check::unify` then `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 3: effect row unification`)

---

### Task 4: Lower effect rows in annotations

Stop erroring on `EffectRow::Named`; lower `! IO`, `! IO, State`, and `! ()`. An unannotated function-typed **parameter** becomes effect-polymorphic (fresh open tail); a written function type with no row is pure.

**Files:**
- Modify: `src/check/infer.rs`

- [ ] **Step 1: Write the failing test**

In `tests/check_effects.rs` (new file):

```rust
use i_lang::{lex, parse, resolve_file, check_file};

fn check(src: &str) -> Result<i_lang::check::Typing, Vec<i_lang::error::Error>> {
    let toks = lex(src).unwrap();
    let file = parse(&toks).unwrap();
    let res = resolve_file(&file).unwrap();
    check_file(&file, &res)
}

#[test]
fn annotated_effectful_signature_lowers_without_error() {
    // `! IO` in an annotation must lower, not raise EffectsNotYetImplemented.
    let src = "\
readLine : ! IO -> String
readLine = readLine
";
    // Self-referential value is fine for lowering; we only assert no
    // EffectsNotYetImplemented error surfaces.
    let result = check(src);
    let has_effects_err = result.as_ref().err().map(|es| {
        es.iter().any(|e| matches!(e.kind, i_lang::error::ErrorKind::EffectsNotYetImplemented))
    }).unwrap_or(false);
    assert!(!has_effects_err);
}
```

(Confirm the exact public API names — `lex`/`parse`/`resolve_file`/`check_file` — against `src/lib.rs`; adjust the harness to match existing integration tests like `tests/check_traits.rs`.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test check_effects annotated_effectful_signature_lowers_without_error`
Expected: FAIL — `EffectsNotYetImplemented` present (or compile error if the API helper differs; align it first).

- [ ] **Step 3: Implement lowering**

In `lower_type_in_scope` (`src/check/infer.rs`), replace the `TypeKind::Function` arm. Lower the AST `EffectRow` to a checker `EffectRow`:

```rust
TypeKind::Function { params, effect, result } => {
    let row = match effect {
        None => EffectRow::pure(),
        Some(EffectRow_ast::Empty) => EffectRow::pure(), // `! ()` closed-empty
        Some(EffectRow_ast::Named(names)) => {
            let mut set = EffectSet::empty();
            for n in names {
                match n.as_str() {
                    "IO" => set = set.union(EffectSet::single(Effect::Io)),
                    "State" => set = set.union(EffectSet::single(Effect::State)),
                    "Env" => set = set.union(EffectSet::single(Effect::Env)),
                    _ => self.errors.push(Error {
                        span: t.span,
                        kind: ErrorKind::Unresolved { name: n.clone() },
                    }),
                }
            }
            EffectRow::concrete(set)
        }
    };
    let ps = params.iter().map(|p| self.lower_param_type(p, ctx)).collect();
    let r = self.lower_type_in_scope(result, ctx);
    Ty::Fun(ps, row, Box::new(r))
}
```

Add `lower_param_type`: if the param's AST is itself a `Function` with `effect: None`, give it a fresh open tail (effect-polymorphic callback); otherwise defer to `lower_type_in_scope`:

```rust
fn lower_param_type(&mut self, t: &Type, ctx: &HashMap<String, TyVarId>) -> Ty {
    if let TypeKind::Function { params, effect: None, result } = &t.node {
        let tail = EffectVarId(self.fresh_eff());
        let ps = params.iter().map(|p| self.lower_param_type(p, ctx)).collect();
        let r = self.lower_type_in_scope(result, ctx);
        return Ty::Fun(ps, EffectRow::open(EffectSet::empty(), tail), Box::new(r));
    }
    self.lower_type_in_scope(t, ctx)
}
```

Add a `fresh_eff(&mut self) -> u32` counter to `Infer` (mirror `fresh`/`next_var` with `next_eff`). Use the AST enum via its real path (e.g. `use crate::ast::EffectRow as EffectRow_ast;` or qualify inline). Delete the `EffectsNotYetImplemented` push.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test check_effects` then `cargo test`
Expected: PASS. (`EffectsNotYetImplemented` is now unused; Task 5 removes the variant.)

- [ ] **Step 5: Commit and push** (`Plan 6 Task 4: lower effect rows in annotations`)

---

### Task 5: New error variants

Add the four Plan 6 error kinds and remove the dead `EffectsNotYetImplemented`. Map the unifier's `EffectMismatch` into a checker error.

**Files:**
- Modify: `src/error.rs`, `src/check/mod.rs` (`unify_error_to_error`)

- [ ] **Step 1: Write the failing test**

In `src/error.rs` tests:

```rust
#[test]
fn new_effect_error_variants_exist() {
    let _ = ErrorKind::MissingBang { name: "print".into() };
    let _ = ErrorKind::UnnecessaryBang;
    let _ = ErrorKind::EffectMismatch { expected: "! ()".into(), found: "! IO".into() };
    let _ = ErrorKind::QuestionContextMismatch;
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib error`
Expected: FAIL — variants not found.

- [ ] **Step 3: Implement**

In `ErrorKind`: remove `EffectsNotYetImplemented`; add

```rust
    MissingBang { name: String },
    UnnecessaryBang,
    EffectMismatch { expected: String, found: String },
    QuestionContextMismatch,
```

In `unify_error_to_error` (`src/check/mod.rs`), add the arm:

```rust
    UnifyError::EffectMismatch { expected, found } => ErrorKind::EffectMismatch {
        expected: render_row(&expected),
        found: render_row(&found),
    },
```

where `render_row` is a temporary `format!("{:?}", ...)` until Task 8 supplies the friendly renderer (note this in a comment).

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib error` then `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 5: effect/question error variants`)

---

### Task 6: Ambient effect accumulation + `!` rules

The heart of effect inference. Add a per-binding ambient effect row; propagate the callee's row at `!`-marked calls; enforce the strict `!` rules.

**Files:**
- Modify: `src/check/infer.rs`

- [ ] **Step 1: Write the failing tests**

In `tests/check_effects.rs`:

```rust
use i_lang::error::ErrorKind;

#[test]
fn calling_effectful_fn_without_bang_errors() {
    // `doIo` is annotated `! IO`; calling it unmarked must be MissingBang.
    let src = "\
doIo : String ! IO -> Unit
doIo = doIo
run = s -> doIo s
";
    let errs = check(src).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e.kind, ErrorKind::MissingBang { .. })));
}

#[test]
fn bang_on_pure_call_is_unnecessary() {
    let src = "\
double = n -> n * 2
run = n -> double! n
";
    let errs = check(src).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e.kind, ErrorKind::UnnecessaryBang)));
}
```

(`doIo = doIo` is a self-binding placeholder so the annotation drives the type; if the checker rejects trivial self-reference, use a body that returns `Unit` via a parameter, keeping the annotation as the type source.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test check_effects`
Expected: FAIL — no such errors raised (Bang/Call ignore effects today).

- [ ] **Step 3: Implement ambient row + rules**

Add to `Infer`: `pub ambient: EffectRow` (init `EffectRow::pure()`). Reset it to `pure()` at the start of inferring each top-level binding body (in `check_file`, before the body-inference loop per binding; thread the resulting row into the binding's function result — see Task 7 for generalisation). Provide:

```rust
fn add_effect(&mut self, row: &EffectRow) {
    let resolved = apply_eff_row(row, &self.subst);
    let merged = self.ambient.labels.union(resolved.labels);
    // Union tails by unifying: a fresh tail carrying both. For v1's single
    // tail, fold the incoming tail in by unifying ambient.tail with it.
    self.ambient.labels = merged;
    if let Some(t) = resolved.tail {
        match self.ambient.tail {
            None => self.ambient.tail = Some(t),
            Some(existing) if existing == t => {}
            Some(existing) => {
                // Both polymorphic: unify them so they share a tail.
                let _ = unify_rows(
                    &mut self.subst,
                    &EffectRow::open(EffectSet::empty(), existing),
                    &EffectRow::open(EffectSet::empty(), t),
                );
            }
        }
    }
}
```

In `infer_expr`, replace the catch-all handling of `Bang`/`Question` with explicit arms. For `Bang`, handle two shapes — but note `print! x` parses as `Call(Bang(func), args)`, so the `Call` arm must detect a `Bang` func:

```rust
ExprKind::Call { func, args } => {
    let (callee, marked) = match &func.node {
        ExprKind::Bang(inner) => (inner.as_ref(), true),
        _ => (func.as_ref(), false),
    };
    let fn_ty = self.infer_expr(callee);
    let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();
    let result_v = self.fresh();
    let row_v = EffectVarId(self.fresh_eff());
    let expected = Ty::Fun(arg_tys, EffectRow::open(EffectSet::empty(), row_v), Box::new(Ty::Var(result_v)));
    if let Err(err) = unify(&mut self.subst, &fn_ty, &expected) {
        self.errors.push(crate::check::unify_error_to_error(callee.span, err));
    }
    let callee_row = apply_eff_row(&EffectRow::open(EffectSet::empty(), row_v), &self.subst);
    self.check_marker(marked, &callee_row, func.span);
    Ty::Var(result_v)
}
ExprKind::Bang(inner) => {
    // Bare `readLine!` — a zero-arg effectful invocation.
    let fn_ty = self.infer_expr(inner);
    let result_v = self.fresh();
    let row_v = EffectVarId(self.fresh_eff());
    let expected = Ty::Fun(vec![], EffectRow::open(EffectSet::empty(), row_v), Box::new(Ty::Var(result_v)));
    if let Err(err) = unify(&mut self.subst, &fn_ty, &expected) {
        self.errors.push(crate::check::unify_error_to_error(e.span, err));
    }
    let callee_row = apply_eff_row(&EffectRow::open(EffectSet::empty(), row_v), &self.subst);
    self.check_marker(true, &callee_row, e.span);
    Ty::Var(result_v)
}
```

with:

```rust
fn check_marker(&mut self, marked: bool, callee_row: &EffectRow, span: Span) {
    let provably_pure = callee_row.is_concrete_empty();
    if marked {
        if provably_pure {
            self.errors.push(Error { span, kind: ErrorKind::UnnecessaryBang });
        } else {
            self.add_effect(callee_row);
        }
    } else if !provably_pure {
        let name = String::new(); // best-effort; fill from callee Var if available
        self.errors.push(Error { span, kind: ErrorKind::MissingBang { name } });
        self.add_effect(callee_row); // propagate anyway to avoid cascade
    }
}
```

(Keep the existing non-Bang `Call` behaviour folded into the new arm. The `Question` arm is added in Task 10; until then leave it falling through, or stub to `infer_expr(inner)`.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test check_effects` then `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 6: ambient effects and strict ! rules`)

---

### Task 7: Effect generalisation and instantiation

Make effect-polymorphism real: a binding's ambient row becomes its function type's row, free effect-vars generalise into the scheme, and instantiation refreshes them.

**Files:**
- Modify: `src/check/types.rs` (`Scheme.eff_vars`), `src/check/infer.rs` (`instantiate`), `src/check/mod.rs` (generalise)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn hof_is_effect_polymorphic() {
    // applyTwice stays pure with a pure callback, gains IO with an effectful one.
    let src = "\
applyTwice = f x -> f! (f! x)
doIo : Int ! IO -> Int
doIo = doIo
pureUse = n -> applyTwice (m -> m * 2), n
ioUse = n -> applyTwice doIo, n
";
    let typing = check(src).expect("should type-check");
    // pureUse has no IO; ioUse carries ! IO. Assert via rendered schemes
    // (Task 8 finalises rendering; here assert pureUse != ioUse row).
    // Placeholder assertion refined once render_row exists:
    assert!(typing.schemes.len() >= 2);
}
```

(Refine the assertion to compare the inferred rows once Task 8's renderer lands; the structural check that both type-check and carry distinct rows is the goal. Use comma-separated multi-arg calls — `applyTwice doIo, n` — per `i`'s call syntax.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test check_effects hof_is_effect_polymorphic`
Expected: FAIL (effect-vars not generalised/instantiated; rows not threaded).

- [ ] **Step 3: Implement**

In `Scheme` add `pub eff_vars: Vec<EffectVarId>` (update the two literal `Scheme { .. }` constructions in `mod.rs` and the `types.rs` tests to include `eff_vars: Vec::new()`).

In `instantiate` (`infer.rs`), refresh eff-vars too:

```rust
pub fn instantiate(&mut self, scheme: Scheme, span: Span) -> Ty {
    let mut s = Subst::new();
    for v in &scheme.vars {
        let fresh = self.fresh();
        s.tys.insert(*v, Ty::Var(fresh));
    }
    for ev in &scheme.eff_vars {
        let fresh = EffectVarId(self.fresh_eff());
        s.effs.insert(*ev, EffectRow::open(EffectSet::empty(), fresh));
    }
    for c in &scheme.constraints { /* unchanged: apply_subst(&c.ty, &s) */ }
    apply_subst(&scheme.ty, &s)
}
```

In `check_file` (`mod.rs`): when resolving each binding's scheme, fold the ambient row into the function type if the body produced one. Concretely, after `let resolved = apply_subst(&Ty::Var(v), &infer.subst);`, if `resolved` is `Ty::Fun(ps, _pure, r)` rewrite it with `infer.ambient` applied:

```rust
let resolved = apply_subst(&Ty::Var(v), &infer.subst);
let resolved = attach_ambient(resolved, &apply_eff_row(&infer.ambient, &infer.subst));
```

```rust
fn attach_ambient(ty: Ty, ambient: &EffectRow) -> Ty {
    match ty {
        Ty::Fun(ps, _row, r) => Ty::Fun(ps, ambient.clone(), r),
        other => other, // non-function bindings carry no row
    }
}
```

Add a `free_eff_vars(ty) -> HashSet<EffectVarId>` mirroring `free_vars`, and in the generalisation block compute each scheme's `eff_vars` as the effect-vars free in its type minus those free in the environment (reuse the existing `env_free` pattern over eff-vars). Reset `infer.ambient = EffectRow::pure()` before inferring each binding body in the SCC loop.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test` then `make ci`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 7: effect generalisation and instantiation`)

---

### Task 8: Render effect rows

Show rows in `ty_to_string`/`scheme_to_string`: `String ! IO -> Unit`, zero-arg `! IO -> String`, pure prints no row, a tail var prints with a synthetic name. Replace the temporary `render_row` from Task 5.

**Files:**
- Modify: `src/check/types.rs`; re-accept any corpus snapshots

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ty_to_string_renders_effect_rows() {
    let res = crate::resolve::Resolution::default();
    let io = Ty::Fun(vec![Ty::Prim(PrimTy::String)], EffectRow::concrete(EffectSet::single(Effect::Io)), Box::new(Ty::Prim(PrimTy::Unit)));
    assert_eq!(ty_to_string(&io, &res), "String ! IO -> Unit");
    let pure = Ty::Fun(vec![Ty::Prim(PrimTy::Int)], EffectRow::pure(), Box::new(Ty::Prim(PrimTy::Int)));
    assert_eq!(ty_to_string(&pure, &res), "Int -> Int");
    let zero = Ty::Fun(vec![], EffectRow::concrete(EffectSet::single(Effect::Io)), Box::new(Ty::Prim(PrimTy::String)));
    assert_eq!(ty_to_string(&zero, &res), "! IO -> String");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib check::types ty_to_string_renders_effect_rows`
Expected: FAIL — rows not rendered.

- [ ] **Step 3: Implement**

Add `pub fn render_row(row: &EffectRow) -> String` returning `""` for pure, else `" ! " + labels joined by ", "` (e.g. `IO, State`), with a tail var rendered as a lowercase synthetic name (`e`, `f`, …) when `labels` empty-but-open. In `ty_to_string`'s `Fun` arm:

```rust
Ty::Fun(ps, row, r) => {
    let params: Vec<String> = ps.iter().map(|p| ty_to_string(p, res)).collect();
    let eff = render_row(row);
    format!("{}{} -> {}", params.join(", "), eff, ty_to_string(r, res))
}
```

Note the zero-arg case yields `" ! IO -> String"`; trim a leading space so it reads `! IO -> String`. Update `unify_error_to_error`'s `render_row` call (Task 5) to this real one. Update the `Display for Ty` `Fun` arm similarly for consistency (or leave `Display` row-free and document that `ty_to_string` is canonical).

- [ ] **Step 4: Run and re-accept snapshots**

Run: `cargo test`. Existing `check_corpus` snapshots that contain function types may shift (pure rows render identically, so likely no change). If any change: `cargo insta review` is the user's checkpoint — generate, hand off, do not auto-accept.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 8: render effect rows`) — after the user accepts any snapshots.

---

### Task 9: Tag `Result`/`Maybe` in the registry

Recognise (don't seed) the failure types: store their `DefId`s when `build_registry` sees a sum type named `Result`/`Maybe` with the right variants.

**Files:**
- Modify: `src/check/registry.rs`, `src/check/mod.rs`

- [ ] **Step 1: Write the failing test**

In `tests/check_question.rs` (new):

```rust
use i_lang::{lex, parse, resolve_file, check_file};
// (mirror the harness from tests/check_traits.rs)

#[test]
fn registry_tags_maybe_and_result_defids() {
    let src = "\
type Maybe a
    None
    Some : a
type Result a, e
    Ok : a
    Error : e
v : Maybe Int
v = None
";
    // Build through to the registry. Expose enough to assert the tags, e.g.
    // a check_file variant returning Typing whose registry carries builtins,
    // OR assert indirectly via a `?` test in Task 10. If no accessor exists,
    // assert the program type-checks here and defer the tag assertion to
    // Task 10's behavioural tests.
    assert!(check(src).is_ok());
}
```

- [ ] **Step 2: Run to verify failure / baseline**

Run: `cargo test --test check_question`
Expected: PASS for the type-check (sum types already work) — this task is mostly internal wiring; the behavioural proof is Task 10. If asserting tags directly, FAIL until the field exists.

- [ ] **Step 3: Implement the tags**

In `registry.rs`:

```rust
#[derive(Debug, Default, Clone)]
pub struct BuiltinTypes {
    pub result: Option<DefId>,
    pub maybe: Option<DefId>,
}
```

Add `pub builtin: BuiltinTypes` to `TypeRegistry`.

In `build_registry` (`mod.rs`), after inserting a `TypeDeclInfo` whose body is `Sum(variants)`, tag it:

```rust
let variant_names: std::collections::HashSet<&str> =
    variants.iter().map(|v| v.name.as_str()).collect();
match name.as_str() {
    "Maybe" if variant_names == HashSet::from(["Some", "None"]) => {
        infer.registry.builtin.maybe = Some(type_def_id);
    }
    "Result" if variant_names == HashSet::from(["Ok", "Error"]) => {
        infer.registry.builtin.result = Some(type_def_id);
    }
    _ => {}
}
```

(Match on name + exact variant set — the bootstrap heuristic recorded as Plan 9 debt in the spec.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test` then `make ci`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 9: tag Result/Maybe DefIds in the registry`)

---

### Task 10: `?` early-exit inference

Thread the enclosing function's return type and check `Question` against the tagged `Result`/`Maybe`.

**Files:**
- Modify: `src/check/infer.rs`

- [ ] **Step 1: Write the failing tests**

```rust
use i_lang::error::ErrorKind;

#[test]
fn question_unwraps_maybe_in_maybe_context() {
    let src = "\
type Maybe a
    None
    Some : a
firstOf : Maybe Int -> Maybe Int
firstOf = m ->
    x = m?
    Some x
";
    assert!(check(src).is_ok());
}

#[test]
fn question_outside_failure_context_errors() {
    let src = "\
type Maybe a
    None
    Some : a
bad : Maybe Int -> Int
bad = m -> m?
";
    let errs = check(src).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e.kind, ErrorKind::QuestionContextMismatch)));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test check_question`
Expected: FAIL — `?` not type-checked (falls through / wrong result type).

- [ ] **Step 3: Implement the enclosing-return stack + rule**

Add `return_stack: Vec<Ty>` to `Infer`. Push the result tyvar when entering a lambda/binding body, pop on exit. The simplest hook: in `infer_lambda_checked` and the `Lambda` arm, push the body's expected/fresh result type before inferring `body`, pop after. For top-level non-lambda bindings, push the binding's tyvar in `check_file` around body inference.

`Question` arm:

```rust
ExprKind::Question(inner) => {
    let inner_ty = apply_subst(&self.infer_expr(inner), &self.subst);
    let enclosing = self.return_stack.last().cloned();
    let result = self.infer_question(&inner_ty, enclosing.as_ref(), e.span);
    result
}
```

```rust
fn infer_question(&mut self, inner: &Ty, enclosing: Option<&Ty>, span: Span) -> Ty {
    let maybe_id = self.registry.builtin.maybe;
    let result_id = self.registry.builtin.result;
    let head = head_of_con(inner).or_else(|| enclosing.and_then(head_of_con));
    match head {
        Some(id) if Some(id) == maybe_id => {
            let payload = self.fresh();
            let maybe_payload = Ty::Con(id, vec![Ty::Var(payload)]);
            let _ = unify(&mut self.subst, inner, &maybe_payload);
            if let Some(ret) = enclosing.cloned() {
                let _ = unify(&mut self.subst, &ret, &Ty::Con(id, vec![Ty::Var(self.fresh())]));
            } else {
                self.errors.push(Error { span, kind: ErrorKind::QuestionContextMismatch });
            }
            Ty::Var(payload)
        }
        Some(id) if Some(id) == result_id => {
            let ok = self.fresh();
            let err = self.fresh();
            let _ = unify(&mut self.subst, inner, &Ty::Con(id, vec![Ty::Var(ok), Ty::Var(err)]));
            match enclosing.cloned() {
                Some(ret) => {
                    // enclosing must be Result _ err (same error slot)
                    let _ = unify(&mut self.subst, &ret, &Ty::Con(id, vec![Ty::Var(self.fresh()), Ty::Var(err)]));
                }
                None => self.errors.push(Error { span, kind: ErrorKind::QuestionContextMismatch }),
            }
            Ty::Var(ok)
        }
        _ => {
            self.errors.push(Error { span, kind: ErrorKind::QuestionContextMismatch });
            Ty::Var(self.fresh())
        }
    }
}
```

Add `fn head_of_con(ty: &Ty) -> Option<DefId>` (returns the `DefId` of a `Ty::Con`, else `None`). Note `?` does not touch `self.ambient` — it is pure control flow.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test check_question` then `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit and push** (`Plan 6 Task 10: ? early-exit inference`)

---

### Task 11: Corpus fixtures + end-to-end test

Snapshot the new surface and prove the pieces compose.

**Files:**
- Create: `tests/corpus/check/effectful-signature.i`, `tests/corpus/check/effect-poly-hof.i`, `tests/corpus/check/question-chain.i`
- Modify: `tests/check_corpus.rs` glob if needed; `tests/check_end_to_end.rs`

- [ ] **Step 1: Write the fixtures**

`effectful-signature.i`:
```
doIo : String ! IO -> Unit
doIo = doIo
shout = s -> doIo! s
```

`effect-poly-hof.i`:
```
applyTwice = f x -> f! (f! x)
double = n -> n * 2
```

`question-chain.i`:
```
type Maybe a
    None
    Some : a
firstOf : Maybe Int -> Maybe Int
firstOf = m ->
    x = m?
    Some x
```

- [ ] **Step 2: Write the end-to-end test**

In `tests/check_end_to_end.rs`, add a test combining an annotated effectful function, a HOF used with a pure and an effectful callback, and a `?`-using function; assert it type-checks and (once rendering is final) that the effectful path carries `! IO` while the pure path does not.

- [ ] **Step 3: Run and accept snapshots**

Run: `cargo test`. New corpus snapshots are generated; `cargo insta review` is the user's checkpoint — hand off, do not auto-accept.

- [ ] **Step 4: Commit and push** (`Plan 6 Task 11: effect/question corpus + end-to-end`) — after acceptance.

---

### Task 12: Documentation (docs-only — skip code review)

**Files:**
- Modify: `docs/checker.md`, `docs/superpowers/plans/PROGRESS.md`, `README.md`

- [ ] **Step 1: Update docs**

In `docs/checker.md`, replace the "effects deferred" note with how rows, the strict `!`, polymorphism, and `?` work. In `PROGRESS.md`, add the Plan 6 completion block and check the Effects line. In `README.md`, update the Status section ("Effects and `?` are now checked; totality is next").

- [ ] **Step 2: Verify and commit**

Run: `make ci` (docs-only, but the hook runs it). Commit (`Plan 6 Task 12: document effects and ?`) and push.

---

## Self-review notes (author)

- **Spec coverage:** data model (T1–2), row unification (T3), annotations + `! ()` + poly param (T4), errors (T5), ambient + strict `!` + bare-bang (T6), generalisation/instantiation/polymorphism (T7), rendering (T8), recognise Result/Maybe (T9), `?` + enclosing-return + QuestionContextMismatch (T10), corpus + e2e (T11), docs (T12). All spec sections map to a task.
- **Deferred per spec:** concrete `print`/`Ref`/`Env` bindings, user-named row vars, exception effect, runtime semantics, and the Result/Maybe recognition firm-up — all Plan 8/9, recorded in the spec + PROGRESS.md.
- **Known soft spots to confirm during execution:** (a) exact public test-harness API (`lex`/`parse`/`resolve_file`/`check_file`) — align with `tests/check_traits.rs`; (b) the `add_effect` tail-union for the rare two-distinct-tails case — the unify-tails approach is sufficient for v1's single-tail surface; (c) whether `Display for Ty` should also render rows or defer to `ty_to_string` — pick one in T8 and keep snapshots canonical on `ty_to_string`.
