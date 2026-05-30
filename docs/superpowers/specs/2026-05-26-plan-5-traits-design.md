# Plan 5: Traits + operator desugaring — design

## Goal & framing

Replace Plan 4's hardcoded operator rules with real ad-hoc polymorphism.
After Plan 5:

- `a + b` means "call the `Add` impl for the operand type" — operators
  dispatch through traits, not a built-in Int/Float check.
- Generic functions can carry trait constraints, so a helper that uses
  `==` internally infers a type like `Eq a => List a -> Bool`.
- The checker proves the required impls exist and rejects programs that
  use an operator or trait method on a type with no impl.

This is **type-checking only**. There is no interpreter yet (Plan 8), so
nothing dispatches at runtime — no dictionary passing. "Operator
desugaring" is honoured as *semantics* (`+` means `Add.add`), realised
inside the checker. The AST keeps its `BinOp`/`UnaryOp` nodes untouched:
physically rewriting the tree would fight the "span on every node" rule
and the round-trip pretty-print property test (Plan 2 Task 30). The
interpreter will do the same small operator→method lookup later.

## New data model (`src/check/`)

In Plan 5 the only way a constraint can arise is using an operator —
constraint syntax in annotations (`Eq a => ...`) doesn't parse yet, and
explicit `Trait.method` calls are deferred. Every operator maps to a
fixed, known trait. There is no prelude declaring `Add`/`Eq`/… (Plan 9),
so they have no `DefId`. The operator-traits are therefore **built into
the checker** as a small enum, not modelled as resolver defs:

```rust
// The built-in operator traits. These are intrinsic in Plan 5; Plan 9's
// prelude replaces them with real `trait` declarations.
enum TraitId { Add, Sub, Mul, Div, Pow, Neg, Eq, Ord, Concat }

// A requirement: "this type must implement this trait."
struct Constraint { trait_: TraitId, ty: Ty }

// Scheme grows a constraint list — this is what makes `Eq a => ...` real.
struct Scheme { vars: Vec<TyVarId>, constraints: Vec<Constraint>, ty: Ty }

// Impl-table key. Primitives have no DefId, so the head covers both.
enum TypeHead { Prim(PrimTy), Con(DefId) }

struct ImplInfo { trait_: TraitId, head: TypeHead }
```

- **Built-in trait method signatures** live in code, not a registry: each
  `TraitId` knows its method shape — `Add`/`Sub`/`Mul`/`Div`/`Pow` are
  `a, a -> a`; `Neg` is `a -> a`; `Eq`/`Ord` are `a, a -> Bool`; `Concat`
  is `a, a -> a`. (`Eq` conceptually has `eq`/`ne` and `Ord` has
  `lt`/`le`/`gt`/`ge`, but for *typing* every method of a trait shares one
  shape, so the checker only needs the shape per `TraitId`.)
- **Impl table**: `HashMap<(TraitId, TypeHead), ImplInfo>`. The map key
  *is* the coherence rule: a duplicate insert is a `DuplicateImpl` error.
  One impl per `(trait, type)` pair across the whole program
  (Haskell-style global coherence, per the language spec § 7).
- **Synthetic primitive impls**: at startup the checker seeds the impl
  table with the prelude impls on primitives — `(Add, Prim(Int))`,
  `(Add, Prim(Float))`, `(Eq, Prim(Int))`, `(Ord, Prim(Float))`,
  `(Concat, Prim(String))`, and the rest of the standard set — except
  `Pow`, which ships on Float only (negative integer exponents wouldn't
  return an Int; see stdlib.md § Pow). This is the
  same "built-in until Plan 9" move the resolver already uses for
  primitive *types* (`resolve_type_name`). When Plan 9 ships a real
  `prelude.i`, the source impls replace this seeding; the leaf intrinsics
  (what `add` does to two machine Ints) move behind the prelude's impl
  bodies.
- **User impls of built-in traits** are the headline feature: `impl Eq
  Point` registers `(Eq, Con(point_def))`, which is exactly what lets
  `pt == pt2` type-check. The `Eq` in `impl Eq Point` is parsed from the
  `ImplDecl`'s `trait_name` string to a `TraitId`; an unrecognised name is
  `UnknownTrait`.

## Data flow through inference

1. **Collect.** `Infer` accumulates a `Vec<Constraint>` alongside its
   `Subst`, the way it already accumulates errors.
2. **Operators.** `infer_binop` / `infer_unaryop` stop hardcoding
   Int/Float. `+` maps to `TraitId::Add`: unify the two operands together,
   emit an `Add` constraint on the operand type, and return the result
   type from the trait's shape (`Add` → operand type, `Eq`/`Ord` → `Bool`,
   `Concat` → operand type).
3. **Instantiate.** When a scheme that carries constraints is used, its
   constraints are instantiated with the same fresh vars and added to the
   ambient set — so a caller of a constrained generic (e.g. one inferred
   as `Eq a => ...`) inherits the `Eq` obligation at its own call site.
4. **Solve** (per SCC, before generalising). For each constraint, apply
   the current substitution and inspect the head:
   - **Concrete** (`Prim` or `Con`) → look up the impl. Found ⇒
     discharged. Missing ⇒ `MissingImpl`.
   - **A variable about to be generalised** → retain it; it becomes part
     of the scheme (`Eq a => ...`).
   - **A variable that is neither** (monomorphic, unconstrained) →
     `AmbiguousConstraint`.
5. **Generalise.** Retained constraints attach to the scheme alongside the
   quantified vars.

## Building the impl table

A pre-inference pass (extending `build_registry`) walks every `ImplDecl`
into an `ImplInfo`, then seeds the synthetic primitive impls. Checks
performed here:

- the `trait_name` is a known built-in trait (`UnknownTrait` otherwise);
- the target type's head resolves (via `lower_type`) to a `TypeHead`;
- no duplicate impl for a `(trait, type)` pair (`DuplicateImpl`);
- the impl supplies *exactly* the trait's methods — none missing, none
  extra (`MissingMethod` / `UnknownMethod`), checked against the built-in
  method-name set for that `TraitId`.

Per the language spec, **no superclasses** (a trait can't require another)
and **no default method bodies** (every impl provides every method).

Explicit trait-qualified calls (`Eq.eq a, b`) and **user-declared `trait`
blocks** are **deferred** — see below. Plan 5 reaches trait methods only
through operators, which is sufficient because every operator trait is
built in and every method in those traits has an operator.

## Friendly type names

A registry-aware printer renders errors and corpus snapshots with real
names — `Maybe Int`, `Point doesn't implement Eq` — instead of the `#5`
DefId form. The bare `Display for Ty` stays for the var/prim/fun cases;
constructor names come from the registry. Corpus snapshots are
re-accepted with friendly names (the user's interactive checkpoint).

## Errors added

| Variant | When |
| --- | --- |
| `MissingImpl { trait_name, ty }` | A required `(trait, type)` has no impl. |
| `DuplicateImpl { trait_name, ty }` | Two impls for the same `(trait, type)` — coherence violation. |
| `UnknownTrait { name }` | An `impl` names something that isn't a known built-in trait. |
| `MissingMethod` / `UnknownMethod` | An impl omits a trait method, or defines one the trait didn't declare. |
| `AmbiguousConstraint { trait_name }` | A constraint pins to neither a concrete type nor a generalisable var. |

## Testing

Per-task TDD throughout. Integration tests in `tests/`:

- operator dispatch on primitives and on a user type with an impl;
- a generic constrained function inferring `Eq a => ...`;
- missing-impl rejection;
- duplicate-impl rejection (coherence);
- impl method-set mismatch (missing / extra method);
- ambiguous-constraint rejection.

New corpus fixtures under `tests/corpus/check/` for trait, impl, and
constrained-generic snapshots. An end-to-end test: a `type` with an
`impl`, exercised through both an operator and a generic helper. Update
`docs/checker.md` (replace the "operators are hardcoded" and "friendly
names deferred" notes) and `PROGRESS.md`.

## Explicitly deferred

- **User-declared `trait` blocks and explicit trait-qualified calls**
  (`trait Foo a`, `Eq.eq a, b`, `Show.show x`) — these land together,
  naturally with **Plan 9 (stdlib)**. They share one prerequisite: a way
  to *name and invoke* a user-defined trait. In Plan 5 there is none —
  constraint syntax in annotations doesn't parse, explicit calls are
  deferred, and operators only ever reach the built-in traits — so a
  user-declared trait would be inert. Plan 9 is the first code that must
  name a trait method with no operator (`Show.show` has none) and write
  real prelude impl bodies. Explicit calls are also primarily a *resolver*
  feature: the resolver currently rejects `Trait.method` (it tries `Eq` as
  a module, then as a constructor, then errors `Unresolved`). Nothing in
  Plans 6–8 needs either; both can be pulled forward as a small standalone
  task if wanted sooner. (`TraitDecl` still parses and resolves today; the
  Plan 5 checker simply ignores it.)
- **Parameterised / conditional impls** (`impl Eq (List a)` needing
  `Eq a`) — Plan 5 matches only on the type *head*; conditional impls and
  their constraint propagation come later.
- **Runtime dispatch / dictionary passing** — Plan 8 (interpreter).
- **Effects** — Plan 6. **Totality** — Plan 7. Unchanged by this plan.
