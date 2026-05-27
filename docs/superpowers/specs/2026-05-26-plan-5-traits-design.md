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

```rust
// A requirement: "this type must implement this trait."
struct Constraint { trait_id: DefId, ty: Ty }

// Scheme grows a constraint list — this is what makes `Eq a => ...` real.
struct Scheme { vars: Vec<TyVarId>, constraints: Vec<Constraint>, ty: Ty }

// Impl-table key. Primitives have no DefId, so the head covers both.
enum TypeHead { Prim(PrimTy), Con(DefId) }

struct TraitInfo { name: String, type_var: TyVarId, methods: Vec<MethodInfo> }
struct ImplInfo  { trait_id: DefId, head: TypeHead, methods: Vec<MethodInfo> }
```

- **Trait registry**: `HashMap<DefId, TraitInfo>`. Each method's scheme is
  written in terms of the trait's single type var and carries the
  self-constraint — e.g. `Eq.eq : forall a. Eq a => a, a -> Bool`.
- **Impl table**: `HashMap<(DefId, TypeHead), ImplInfo>`. The map key *is*
  the coherence rule: a duplicate insert is a `DuplicateImpl` error. One
  impl per `(trait, type)` pair across the whole program (Haskell-style
  global coherence, per the language spec § 7).
- **Synthetic primitive impls**: at startup the checker seeds the impl
  table with the prelude impls on primitives — `(Add, Prim(Int))`,
  `(Eq, Prim(Int))`, `(Ord, Prim(Float))`, `(Concat, Prim(String))`, and
  the rest of the standard set. This is the same "built-in until Plan 9"
  move the resolver already uses for primitive *types*
  (`resolve_type_name`). When Plan 9 ships a real `prelude.i`, the source
  impls replace this seeding; the leaf intrinsics (what `add` does to two
  machine Ints) move behind the prelude's impl bodies.

## Data flow through inference

1. **Collect.** `Infer` accumulates a `Vec<Constraint>` alongside its
   `Subst`, the way it already accumulates errors.
2. **Operators.** `infer_binop` / `infer_unaryop` stop hardcoding
   Int/Float. `+` looks up the `Add` trait, instantiates `Add.add`'s
   scheme, and applies it to the operands — which unifies the operands and
   emits an `Add` constraint on their type. The result type falls out of
   the method signature (`Add` → same type, `Eq`/`Ord` → `Bool`,
   `Concat` → same type).
3. **Instantiate.** When a scheme that carries constraints is used, its
   constraints are instantiated with the same fresh vars and added to the
   ambient set — so a caller of `eq` inherits the `Eq` obligation.
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

## Building traits & impls

A pre-inference pass (extending `build_registry`) walks every `TraitDecl`
into a `TraitInfo` and every `ImplDecl` into an `ImplInfo`, then seeds the
synthetic primitive impls. Checks performed here:

- the named trait exists (`UnknownTrait`);
- no duplicate impl for a `(trait, type)` pair (`DuplicateImpl`);
- the impl supplies *exactly* the trait's methods — none missing, none
  extra (`MissingMethod` / `UnknownMethod`).

Per the language spec, **no superclasses** (a trait can't require another)
and **no default method bodies** (every impl provides every method).

Explicit trait-qualified calls — `Eq.eq a, b`, which the spec uses inside
impl bodies — resolve `Eq.eq` to the trait method and instantiate its
constrained scheme like any other reference.

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
| `UnknownTrait { name }` | An `impl` or constraint names something that isn't a trait. |
| `MissingMethod` / `UnknownMethod` | An impl omits a trait method, or defines one the trait didn't declare. |
| `AmbiguousConstraint { trait_name }` | A constraint pins to neither a concrete type nor a generalisable var. |

## Testing

Per-task TDD throughout. Integration tests in `tests/`:

- operator dispatch on primitives and on a user type with an impl;
- a generic constrained function inferring `Eq a => ...`;
- missing-impl rejection;
- duplicate-impl rejection (coherence);
- impl method-set mismatch (missing / extra method);
- explicit `Eq.eq` trait-method calls;
- ambiguous-constraint rejection.

New corpus fixtures under `tests/corpus/check/` for trait, impl, and
constrained-generic snapshots. An end-to-end test: a `type` with an
`impl`, exercised through both an operator and a generic helper. Update
`docs/checker.md` (replace the "operators are hardcoded" and "friendly
names deferred" notes) and `PROGRESS.md`.

## Explicitly deferred

- **Parameterised / conditional impls** (`impl Eq (List a)` needing
  `Eq a`) — Plan 5 matches only on the type *head*; conditional impls and
  their constraint propagation come later.
- **Runtime dispatch / dictionary passing** — Plan 8 (interpreter).
- **Effects** — Plan 6. **Totality** — Plan 7. Unchanged by this plan.
