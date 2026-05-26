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

Primitive operators are typed directly, *not* via traits (that's
Plan 5). Arithmetic and ordering unify their operands and require the
result to be `Int` or `Float`; equality returns `Bool`; logical
operators expect `Bool`; `++` expects `String`. There is no "numeric"
type variable, so the Int/Float constraint is a direct check on the
resolved type. A list literal `[a, b, c]` types as `List elem`, unifying
every element against one fresh element variable; `List` is an ordinary
library type, so an out-of-scope `List` is an `UnknownType` error.

---

## Pretty-printing

`Display for Ty`/`Scheme`/`Typing` (`src/check/types.rs`) renders types
in surface-ish syntax for error messages and corpus snapshots:
`Int, Int -> Bool`, `forall t0 . t0 -> t0`. Constructors print as
`#<DefId>` because the type carries no name; a `Resolution`-aware
printer that prints friendly names is deferred to Plan 5.

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

---

## What's deferred

- **Traits and operator dispatch** — Plan 5. Operators are typed with
  hardcoded Int/Float/Bool/String rules today; Plan 5 replaces that with
  real `impl` resolution and rips out the placeholder.
- **Effects** — Plan 6. Effect rows in annotations currently raise
  `EffectsNotYetImplemented`; `!` and `?` aren't typed yet.
- **Totality / exhaustiveness beyond sums** — Plan 7. Today only missing
  sum variants are flagged; primitive and nested-pattern coverage is
  not.
- **Friendly type names in output** — Plan 5. `Display` prints
  `#<DefId>`; a `Resolution`-aware printer would show `Maybe Int`.
- **General bidirectional checking** — only the top-level-annotation /
  recursive-sibling case flows expected types in; nested expected-type
  positions still rely on synthesis.
- **Tuples** — annotations with tuple types raise
  `TuplesNotYetImplemented`.
