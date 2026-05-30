# Type checking

The type checker sits after name resolution. It takes a parsed `File`
plus its `Resolution` side-table and infers a type for every top-level
binding, every expression, and every pattern — rejecting programs that
don't type. It is a Hindley-Milner core: Algorithm W with eager
unification and let-generalisation at the top level only.

Source: `src/check/`. The public entry point is `check_file(&File,
&Resolution) -> Result<Typing, Vec<Error>>` in `src/check/mod.rs`.

The checker is *parasitic* on the resolver: it never re-decides what a
name means. At each `Var`/`Ctor`/pattern site it looks up
`res.refs[&span]` to get a `ResolvedName`, then maps that to a type. See
[Name resolution](resolution.md) for what those references mean.

---

## The type model

Four type shapes (`src/check/types.rs`):

```rust
pub enum Ty {
    Var(TyVarId),        // unification variable
    Prim(PrimTy),        // Int, Float, String, Bool, Unit
    Con(DefId, Vec<Ty>), // nominal type applied to args, e.g. Maybe Int
    Fun(Vec<Ty>, Box<Ty>),
}
```

`Con` carries a `DefId`, not a name: `Maybe Int` is `Con(maybe_def,
[Prim(Int)])`. The name lives in the resolver's `defs`; the type stores
only the identity. This is why the `Display` impl prints `#<id>` for
constructors (see *Pretty-printing* below) — Plan 4 has no name to print
without consulting the `Resolution`.

A **scheme** is a type with quantified variables — the `forall` of
let-polymorphism:

```rust
pub struct Scheme { pub vars: Vec<TyVarId>, pub ty: Ty }
```

`id = x -> x` gets `forall t1 . t1 -> t1`. Each *use* of `id`
**instantiates** the scheme with fresh variables, so two call sites can
specialise it to different types.

Unification (`src/check/unify.rs`) is the standard destructive variant:
a mutable `Subst` (`HashMap<TyVarId, Ty>`) records what each variable
has been pinned to. `unify` walks two types structurally, binds
variables, and runs the **occurs check** so `t = t -> t` is rejected
rather than building an infinite type.

---

## Output: `Typing`

Like `Resolution`, the result is a side-table — the AST is not mutated:

```rust
pub struct Typing {
    pub schemes: HashMap<DefId, Scheme>,  // one per top-level binding/ctor
    pub expr_types: HashMap<Span, Ty>,    // every expression's inferred type
    pub pattern_types: HashMap<Span, Ty>, // every pattern's inferred type
}
```

`schemes` is the public surface most tests assert against. `expr_types`
and `pattern_types` are span-keyed records for later passes (and for
debugging) — populated as a side effect of inference via
`record_expr_type` / `record_pattern_type`.

---

## Two-phase top-level inference

Top-level bindings can refer to each other in any order and can be
mutually recursive, so they can't be inferred in source order. The
checker (`check_file` in `src/check/mod.rs`):

1. **Builds a dependency graph.** Binding *i* depends on *j* if *i*'s
   body references *j* as a top-level `Var`. `tarjan_scc` returns the
   strongly-connected components in topological order — so each SCC is
   processed after the ones it depends on, which are already
   generalised.

2. **Per SCC:** pre-declare a fresh tyvar for every member (and unify it
   with the user annotation, if any, *before* inference, so recursive
   uses see the pinned type). Then infer each body and unify it with its
   pre-declared tyvar. Then generalise.

Annotations are merged from two sources: an inline `x : T = v`, or a
separate `x : T` signature line paired with `x = v` by name.

### Generalisation

A tyvar in an SCC member's resolved type is generalised (quantified in
its scheme) **unless** it escapes into:

- another already-finalised scheme's free variables, or
- another member of the *same* SCC.

The second rule is the deliberate "no polymorphic recursion across a
mutually-recursive group" choice. `ping = n -> pong n` /
`pong = n -> ping n` both come out as the *ungeneralised* `t -> t'`
(they share the variables), not `forall a b . a -> b`. This is recorded
in the `mutual-rec.i` corpus snapshot.

### Bidirectional parameter checking

Inference is otherwise pure Algorithm W (synthesis), with one
checking-mode exception. When a binding's tyvar already resolves to a
`Fun` *before* its body runs — via an annotation, or via a
mutually-recursive sibling that constrained it — the expected parameter
types are pushed into the lambda's patterns before the body is inferred
(`Infer::infer_lambda_checked`). Without this, `getX : Point -> Float;
getX = p -> p.x` fails: eager member access on `p` would see only an
unresolved tyvar, because the annotation would otherwise unify too late.
General bidirectional checking for nested positions is **deferred**.

---

## The type registry

Before inference, `build_registry` walks every `type` declaration into a
`TypeRegistry` (`src/check/registry.rs`) keyed by `DefId`:

```rust
pub struct TypeDeclInfo {
    pub name: String,
    pub body: TypeDeclBody,    // Record(fields) | Sum(variants) | Newtype(ty)
    pub methods: Vec<MethodInfo>,
}
```

This is the lookup the inferer consults for fields, variants, and
methods. Constructor and method *schemes* are seeded into
`Typing::schemes` so a `Ctor`/method reference instantiates like any
other binding.

- **Records.** Construction `Point(x = .., y = ..)` checks every field
  is present and unifies each value with the declared field type;
  update `p(x = ..)` keeps `p`'s type and only re-checks the changed
  fields. Field access `p.x` resolves the receiver to a `Con`, looks the
  field up in the registry, and returns its (instantiated) type.
- **Sums.** Each variant becomes a constructor scheme: a bare variant
  has the parent type (`None : Maybe a`); a payload variant is a
  function into the parent (`Some : a -> Maybe a`). Type parameters are
  instantiated fresh at each use.
- **Methods.** A method is a value binding inside a `type` block with a
  synthetic `self : ThatType`. `p.x` may resolve to either a field or a
  zero-arg method — the disambiguation the resolver deferred to here.

---

## Patterns and match

`infer_pattern` returns a type plus the locals it binds. Each
`PatternKind` decides its own type: wildcard/var are fresh vars;
literals are their primitive; constructor, record, and list patterns
destructure against the registry and recursively unify sub-patterns. In
a `match`, every arm's pattern unifies with the scrutinee and every
arm's body unifies with a single result variable.

### Exhaustiveness

After typing a `match`, `exhaust::check_arms` (`src/check/exhaust.rs`)
checks coverage. A wildcard or variable arm makes the match exhaustive.
Otherwise, for a sum-typed scrutinee, it diffs the covered constructor
names against the registered variants and reports the missing ones as
`NonExhaustiveMatch { missing }`. Primitives, unresolved types, and
records fall through to "exhaustive" — Plan 4 only flags missing sum
variants; full totality is Plan 7.

---

## Operators and lists

Operators dispatch through traits. Each arithmetic, ordering, equality,
and concat operator maps to a built-in `TraitId` (`+` → `Add`, `<` →
`Ord`, `==` → `Eq`, `++` → `Concat`, unary `-` → `Neg`). `infer_binop`
unifies the two operands, emits a `Constraint` on the agreed operand
type, and returns the trait's result shape: `Bool` for `Eq`/`Ord`,
otherwise the operand type. `and`/`or`/`xor`/`not` are not trait
operators — they stay direct `Bool` operations. A list literal
`[a, b, c]` types as `List elem`, unifying every element against one
fresh element variable; `List` is an ordinary library type, so an
out-of-scope `List` is an `UnknownType` error.

---

## Traits

Plan 5 adds typeclass-style traits as a pure type-checking concern (no
runtime dispatch — that's Plan 8).

**Built-in traits.** There's no prelude yet (Plan 9), and operators are
the only thing that names a trait, so the trait universe is a closed
`TraitId` enum (`src/check/traits.rs`): `Add`, `Sub`, `Mul`, `Div`,
`Pow`, `Neg`, `Eq`, `Ord`, `Concat`. Each knows its display name, its
required method set, and whether its result is `Bool`.

**Impl table.** The registry holds `impls: HashMap<(TraitId, TypeHead),
ImplInfo>`. A `TypeHead` is either `Prim(PrimTy)` or `Con(DefId)` — the
matchable head of a type, unifying primitives and nominal types under
one key. `head_of` extracts it (a type variable or function type has no
head, so it can't carry an impl).

**Synthetic primitive impls.** `seed_builtin_impls` populates the table
with what `prelude.i` will eventually supply in source: `Eq`/`Ord` on
every primitive, the numeric traits on `Int` and `Float`, and `Concat`
on `String`. It runs last during registry build, so user impls take
precedence on collision.

**User impls.** An `impl Eq Point` registers `(Eq, Con(Point))` after
coherence checks: unknown trait name → `UnknownTrait`; a `(trait, head)`
already present → `DuplicateImpl` (Haskell-style global coherence, one
impl per pair); missing or extra methods → `MissingMethod` /
`UnknownMethod` (the exact method set is enforced, both checks run so
all problems surface at once). Method *bodies* aren't checked against
trait signatures in Plan 5.

**Constraints and the solver.** Inferring a body accumulates obligations
in `Infer.constraints: Vec<(Constraint, Span)>` — operator dispatch
pushes them, and instantiating a constrained scheme carries them in
(rewritten through the fresh-var substitution). After each SCC
generalises, the solver splits off that SCC's constraints and discharges
each: a concrete head checks the impl table now (`MissingImpl` if
absent); a constraint on a generalised var attaches to that var's scheme
(so `bothEq` becomes `forall a . Eq a => a, a -> Bool`); anything else
is `AmbiguousConstraint`.

---

## Pretty-printing

`ty_to_string`/`scheme_to_string`/`render_typing` (`src/check/types.rs`)
render types with friendly names, looking up constructor `DefId`s in the
`Resolution` so a type prints as `Maybe Int` rather than `#1(Int)`.
`scheme_to_string` also renders the constraint context, e.g.
`forall t0 . Eq t0 => t0 -> Bool`. These take `&Resolution` and are the
authoritative renderers for user-facing output (error messages, corpus
snapshots). The older `Display for Ty`/`Scheme` impls remain for cases
without a `Resolution` to hand (they still print `#<DefId>`).

---

## Errors

The checker returns `Vec<Error>` — it accumulates rather than bailing on
the first failure.

| Variant | When |
| --- | --- |
| `TypeMismatch { expected, found }` | Unification failed (incl. operand type constraints). |
| `OccursCheck { var }` | A variable would occur in its own binding (infinite type). |
| `ArityMismatch { expected, found }` | Call/constructor applied to the wrong number of arguments. |
| `UnknownType { name }` | A required nominal type (e.g. `List`) isn't in scope. |
| `UnknownField { type_name, field }` | Construction/access names a field the record doesn't have. |
| `MissingField { type_name, field }` | Construction omits a required field. |
| `NonExhaustiveMatch { missing }` | A `match` on a sum type omits variants and has no wildcard. |
| `CannotAccessMember { ty, member }` | `.member` on something that isn't a record/type with that member. |
| `EffectsNotYetImplemented` / `TuplesNotYetImplemented` | Annotation uses a feature deferred to a later plan. |
| `MissingImpl { trait_name, ty }` | An operator needs a trait impl the type doesn't have (e.g. `Point == Point` with no `Eq Point`). |
| `DuplicateImpl { trait_name, ty }` | Two impls of the same trait for the same type (coherence). |
| `UnknownTrait { name }` | An `impl` names a trait that doesn't exist. |
| `MissingMethod { trait_name, method }` | An `impl` omits a method the trait requires. |
| `UnknownMethod { trait_name, method }` | An `impl` defines a method the trait doesn't declare. |
| `AmbiguousConstraint { trait_name }` | A trait obligation falls on a var that never resolves and never generalises. |

---

## What's deferred

- **User-declared `trait` blocks and explicit `Trait.method` calls** —
  Plan 9. Today only the built-in operator traits exist; `trait` blocks
  parse and resolve but aren't checked, and `Eq.eq` doesn't resolve.
- **Parameterised / conditional impls** — `impl Eq (List a)` and the
  like. Plan 5 only matches on a type's head, so impls can't be
  conditioned on argument types.
- **Runtime trait dispatch** — Plan 8. Plan 5 type-checks impls but does
  no dictionary passing; there's no interpreter yet.
- **Effects** — Plan 6. Effect rows in annotations currently raise
  `EffectsNotYetImplemented`; `!` and `?` aren't typed yet.
- **Totality / exhaustiveness beyond sums** — Plan 7. Today only missing
  sum variants are flagged; primitive and nested-pattern coverage is
  not.
- **General bidirectional checking** — only the top-level-annotation /
  recursive-sibling case flows expected types in; nested expected-type
  positions still rely on synthesis.
- **Tuples** — annotations with tuple types raise
  `TuplesNotYetImplemented`.
