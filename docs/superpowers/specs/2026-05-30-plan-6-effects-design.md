# Plan 6: Effects + `?` early-exit — design

## Goal & framing

Plan 4 gave us Hindley-Milner inference; Plan 5 added ad-hoc
polymorphism through traits. Plan 6 adds the last piece of `i`'s type
story: **effect tracking**. After Plan 6:

- A function's type carries an effect row (`! IO`, `! State`, `! Env`),
  inferred from the `!`-marked calls in its body and propagated up the
  call graph.
- Higher-order functions are **effect-polymorphic**: `map` with a pure
  callback is pure; the same `map` with an effectful callback carries the
  callback's effects. This works for user-written HOFs, not just stdlib.
- The `!` marker is mandatory at effectful call sites and rejected on
  provably-pure ones — the compiler holds the "no accidental IO" line.
- The `?` early-exit operator type-checks against `Result`/`Maybe` and the
  enclosing function's return type.

This is **type-checking only**. There is no interpreter yet (Plan 8), so
nothing *performs* an effect or *short-circuits* a `?` at runtime. As in
Plans 4 and 5, the semantics are realised as type rules; the runtime
realisation is deferred.

`?` is bundled here because the roadmap bundles it, even though it is
mechanically independent of effect rows (it is checked control flow over
`Result`/`Maybe`, carrying no effect label — v1 has no exception effect).
It shares no machinery with effect rows beyond living in the same plan.

## What already exists

The surface syntax is parsed and resolved already:

- `ExprKind::Bang(Box<Expr>)` and `ExprKind::Question(Box<Expr>)` —
  `print! "x"` parses as `Call(Bang(Var print), [str])` (the `!` wraps the
  *function*, the call applies args around it); `parseInt s?` parses as
  `Call(parseInt, [Question(s)])` (`?` binds tight to its operand).
- `TypeKind::Function { params, effect: Option<EffectRow>, result }` with
  `EffectRow::{Empty, Named(Vec<String>)}` on the AST.

The checker does nothing useful with any of it today: `Bang`/`Question`
fall through `infer_expr`'s catch-all to a fresh unconstrained variable,
and `lower_type` actively rejects a named effect row with
`EffectsNotYetImplemented`. `Ty::Fun(params, result)` has no effect
component. Plan 6 is a genuine build-out, the same flavour as Plans 4–5.

## Data model (`src/check/types.rs`)

```rust
// 3-bit closed set; v1's whole effect alphabet (effects.md § 4).
struct EffectSet  // flags: IO, State, Env

// Row-polymorphism variable, analogous to TyVarId.
struct EffectVarId(u32)

// Open row: known labels plus an optional polymorphic tail.
struct EffectRow { labels: EffectSet, tail: Option<EffectVarId> }
//  pure              = { ∅,     None }
//  ! IO              = { {IO},  None }
//  callback param ρ  = { ∅,     Some(v) }
//  { {IO}, Some(v) } = "IO and whatever v turns out to be"

enum Ty { ... Fun(Vec<Ty>, EffectRow, Box<Ty>) ... }   // row added
```

- `Subst` gains a second map `EffectVarId → EffectRow` alongside its
  existing `TyVarId → Ty` map.
- `Scheme` gains `eff_vars: Vec<EffectVarId>`, quantified alongside the
  existing `vars` and `constraints`.

Why open rows (labels + optional tail) rather than a bare set or a bare
variable: effect polymorphism needs to express "these known labels *plus*
whatever this variable resolves to" — the union of a concrete row and an
unknown one. A bare set can't carry the unknown; a bare variable can't
carry the known labels. The open-row form is the standard, bounded
formulation and composes with the existing `Subst`/`unify`/generalize
design. Because the label set is closed at three, the row unification it
requires stays small.

## Row unification (`src/check/unify.rs`)

A bounded routine to unify `{L1, t1}` with `{L2, t2}`:

- Two closed rows (`t1 = t2 = None`): must have `L1 == L2`, else
  `EffectMismatch`.
- One side open: its tail variable solves to a row carrying the labels the
  other side has and it lacks, plus the other side's tail. (With a closed
  three-label alphabet this is a finite, decidable solve.)
- Both open: unify into a fresh shared tail carrying the union of labels.
- Occurs-check on effect-vars, mirroring the type-var occurs check.

This is the only genuinely new algorithm in the plan; everything else
reuses existing plumbing.

## Effect inference (`src/check/infer.rs`)

The checker threads an **ambient effect accumulator** while inferring a
binding body — a mutable `EffectRow`, the same way it already accumulates
`constraints`. Rules:

- **`Call(Bang(g), args)`** — infer `g : Fun(ps, row, res)`, unify `ps`
  against `args`, **union `row` into the ambient row**, result `res`. The
  `!` is the acknowledgment that authorises the effect.
- **`Call(g, args)`, `g` not `Bang`** — if `g`'s row is non-empty *or* a
  variable (i.e. not provably pure) → `MissingBang`. Union the row anyway,
  to avoid cascading errors downstream.
- **Bare `Bang(g)`** (zero-argument procedure, `readLine!`) — `g :
  Fun([], row, res)`, union `row`, result `res`.
- **Strict marker check** — a `Bang` whose acknowledged row is concretely
  `∅` → `UnnecessaryBang`. A *variable* row is exempt: that is exactly the
  polymorphic-callback case (`f!` inside a HOF), where the row is unknown
  at the marker site and only solves to `∅` at some call sites.

After a binding's body is inferred and its substitution is final, the
accumulated ambient row *is* that binding's effect row. It generalises:
effect-vars free in the type and not in the environment are quantified
into the scheme's `eff_vars`, exactly as type-vars are quantified into
`vars`. Instantiation refreshes `eff_vars` to fresh effect-vars, so each
use of an effect-polymorphic function gets its own tail.

## Effect polymorphism (effects.md § 7)

A **function-typed parameter with no explicit effect annotation** is
effect-polymorphic: it lowers to `Fun(ps, {∅, Some(v)}, res)` with a fresh
tail `v`. Because that row is a variable (not provably pure), every call
to the parameter inside the HOF body is `!`-marked, which threads `v` into
the HOF's ambient row. At each call site of the HOF, row unification
solves `v` to the actual callback's effect:

```
applyTwice = f x -> f! (f! x)
applyTwice double   # f's row solves to ∅   → pure
applyTwice shout    # f's row solves to {IO} → ! IO
```

**Pinning a callback pure**: `! ()` in an annotation lowers to a *closed*
empty row `{∅, None}`. A callback typed `(a -> b ! ())` refuses any
effectful function — unifying its closed-empty row against a non-empty row
is `EffectMismatch`. This is the explicit opt-out the spec reserves for
trait methods like `Show.show` and order-sensitive HOFs.

User-named row variables (`! e` with `e` a bound name) stay out of v1; the
implicit tail covers every case v1 can express.

## Annotations (`lower_type`)

`lower_type` stops erroring on `EffectRow::Named` and instead lowers it:

- `! IO` / `! IO, State` → closed `EffectRow { labels, tail: None }`.
- `! ()` (`EffectRow::Empty`) → closed `{∅, None}`.
- **No row written** on a function type means pure (`{∅, None}`) for that
  position — *except* an unannotated function-typed **parameter**, which
  is the effect-polymorphic case above (fresh open tail).

When a hand-written annotation pins a binding's type, an inferred row that
exceeds the annotated row is caught by the existing annotation-mismatch
path, surfaced as `EffectMismatch`. This is the mechanism behind "pinning
the inferred row in the signature prevents accidental widening"
(effects.md § 6).

## `?` early-exit (syntax.md § 9)

`expr?` unwraps a success value or propagates failure to the enclosing
function. It is checked control flow, not an effect.

- **Recognise `Result`/`Maybe`; do not seed them.** `Result a e`
  (`Ok`/`Error`) and `Maybe a` (`Some`/`None`) are defined as ordinary sum
  types — by the fixtures in Plan 6, by `prelude.i` in Plan 9 — using the
  existing sum-type machinery the language already supports (`type Maybe a
  / None / Some : a` already type-checks today; see `tests/check_sums.rs`).
  When `build_registry` runs, it **tags** the `DefId`s of any registered
  type named `Result`/`Maybe` whose variant set matches (`Ok`/`Error`,
  `Some`/`None`) and stores them (e.g. `builtin.result`, `builtin.maybe`).
  `?` recognises failure types by those **stored `DefId`s**, never by name
  at the use site — so the recognition is DefId-based and robust (it
  handles `Result a e`'s positional error slot exactly), while the
  name+variant match is a one-shot bootstrap used only to *find* the
  canonical type. This needs no resolver changes and no built-in seeding,
  reuses the existing sum-type path, and breaks none of Plan 4's fixtures.

  *Why not seed built-ins (the rejected alternative):* primitives carry no
  `DefId` (`lower_type` string-matches them; the resolver skips them), but
  `Result`/`Maybe` are parameterised nominals with constructors that must
  resolve and need ctor schemes — so seeding them would invent a new
  synthetic-nominal path in the resolver that nothing else needs, collide
  with the user-defined `type Maybe a` already in three fixtures, and force
  a removal-migration when Plan 9 defines them in `prelude.i`. The
  recognise-don't-seed approach is invisible to `?` when that source-moves
  to the prelude. (Decision reached via two-subagent analysis, 2026-05-30;
  supersedes this section's earlier "seed as built-in types" wording.)

  **Debt (→ Plan 9):** the name+variant *bootstrap* is a heuristic — a user
  could declare an unrelated `type Maybe` with the wrong variants. Plan 9
  retires it: once `prelude.i` is the canonical source, recognise
  `Result`/`Maybe` by the prelude's `DefId`s and reject user redefinitions
  that shadow the prelude types. Recorded on the Plan 9 line of
  `PROGRESS.md` and in "Explicitly deferred" below.
- Infer threads the **enclosing function's return type** as a stack,
  pushed when descending into a lambda or binding body and popped on the
  way out. `?` consults the innermost entry.
- **Type rule for `Question(e)`** — inspect the head of `e`'s type, or of
  the enclosing return type when `e`'s is still a variable:
  - `e : Result a e'` ⇒ unify enclosing-return with `Result _ e'` (same
    error type), result type `a`.
  - `e : Maybe a` ⇒ unify enclosing-return with `Maybe _`, result type
    `a`.
  - Neither head is `Result`/`Maybe`, or there is no `Result`/`Maybe`
    enclosing context, or the error types disagree ⇒
    `QuestionContextMismatch`.

## Friendly rendering (`ty_to_string`)

`ty_to_string` (and `scheme_to_string`) render effect rows: `String ! IO
-> Unit`, the zero-argument form `! IO -> String`, and a pure function
with no row at all. A polymorphic tail variable renders with a synthetic
name drawn from the same pool as type-vars (`a`, `b`, …) or is elided; the
exact spelling is settled in the rendering task, and corpus snapshots are
re-accepted then (the user's interactive checkpoint).

## Errors added

| Variant | When |
| --- | --- |
| `MissingBang { name }` | An effectful call written without `!`. |
| `UnnecessaryBang` | `!` on a provably-pure call (strict rule). |
| `EffectMismatch { expected, found }` | An inferred row exceeds a hand-written annotation, or a `! ()` callback receives an effectful function. |
| `QuestionContextMismatch` | `?` outside a `Result`/`Maybe`-returning function, on a non-`Result`/`Maybe` value, or with a mismatched error type. |

`EffectsNotYetImplemented` is removed.

## Testing

Per-task TDD throughout. Integration tests in `tests/`:

- effect propagation: a function calling a hand-annotated effectful
  function infers the same row;
- `MissingBang` on an unmarked effectful call; `UnnecessaryBang` on a
  marked pure call;
- effect polymorphism: a user HOF stays pure with a pure callback and
  picks up `! IO` with an effectful one;
- `! ()` rejects an effectful callback (`EffectMismatch`);
- annotation pinning: an inferred row exceeding the annotation is rejected;
- `?` on `Result` and on `Maybe`; `QuestionContextMismatch` outside a
  failure-returning context and on a mismatched error type.

New corpus fixtures under `tests/corpus/check/` for an effectful
signature, an effect-polymorphic HOF, and a `?` chain. An end-to-end test
combining an annotated effectful function, a HOF applied to both pure and
effectful callbacks, and a `?`-using function. Update `docs/checker.md`
(replace the "effects deferred" note) and `PROGRESS.md`.

## Explicitly deferred — and where it is accounted for

- **Concrete effectful bindings** — `print`, `println`, `readLine`,
  `readFile`, `writeFile`, `Ref.make`/`get`/`set`, `Env.args`/`var` — and
  the resolver wiring that makes those names resolve, are deferred to
  **Plan 9 (stdlib)**, where `prelude.i` defines them with the
  effect-bearing signatures effects.md and stdlib.md already specify. Plan
  6 tests effect propagation through *hand-written annotations* instead,
  which is the purest test of the machinery and needs no prelude. This
  deferral is recorded both here and on the Plan 9 line of `PROGRESS.md`.
- **Canonical `Result`/`Maybe` recognition** — Plan 6 finds these types by
  a name+variant bootstrap heuristic at registry-build (see the `?`
  section). **Plan 9 (stdlib)** retires the heuristic: with `prelude.i` as
  the canonical source, recognise them by the prelude's `DefId`s and reject
  user redefinitions that shadow the prelude types. Recorded on the Plan 9
  line of `PROGRESS.md`.
- **User-named effect-row variables** (`(a -> b ! e)` with `e` bound) —
  post-v1 (effects.md § 7). The implicit tail covers every v1 case.
- **An exception / error effect** — modelling `?` as an effect rather than
  as sugar over `Result`/`Maybe` is out of v1 (effects.md § 4).
- **Runtime semantics** — actually performing IO, mutating a `Ref`, and
  short-circuiting `?` — belong to **Plan 8 (interpreter)**.
- **Conditional / parameterised propagation beyond a single tail** — Plan
  6 matches the spec's v1 surface exactly; richer row algebra is not
  needed and is not built.
