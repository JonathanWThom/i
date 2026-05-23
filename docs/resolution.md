# Name resolution

The resolver sits between the parser and the type checker. It walks a
parsed `File` (or `ModuleSet`) and, for every name *reference* — every
`Var`, `Ctor`, type name, effect-row member, and `self` — decides what
that name refers to. It also rejects things that ought to be rejected:
unknown names, duplicate top-level definitions, duplicate names in the
same scope, cyclic module imports, cherry-picks of unexposed names.

Source: `src/resolve/`. Public entry points are `resolve_file` and
`resolve_project` in `src/resolve/mod.rs`.

---

## Two namespaces

Top-level definitions split into two independent namespaces:

- **Value namespace** — `=` bindings, constructors, cherry-picked
  values, lambda parameters, pattern variables, block let-bindings, and
  the synthetic `self`. Looked up from `Var` and `Ctor` expression
  positions, and from constructor patterns.
- **Type namespace** — type declarations, traits, and effect-row
  member names. Looked up from `TypeKind::Named`, `Construct`/`Update`
  type-name slots, and `EffectRow::Named` entries.

`type Result a` and `Result = ...` can coexist without colliding — the
former lives in the type namespace, the latter in the value namespace.
Resolution routes the lookup by syntactic position, not by the name's
case.

---

## Output: `Resolution`

Resolution is a side-table (`src/resolve/types.rs`), not a new AST:

```rust
pub struct Resolution {
    pub defs: Vec<DefInfo>,
    pub refs: HashMap<Span, ResolvedName>,
}
```

- `defs` lists every top-level definition the file introduced — values,
  types, traits, and constructors (with `of_type` back-references).
- `refs` maps each reference site's `Span` to a `ResolvedName`:
  `Local(LocalId)`, `TopLevel(DefId)`, `Ctor(DefId)`,
  `Imported { module, name }`, or `Module(ModulePath)`.

Downstream passes look references up by the AST node's span. The AST
itself is not mutated and not duplicated. The trade-off: span-keyed
lookups are O(1) but cost one `HashMap` per file. The alternative
(parallel `ResolvedExpr`/`ResolvedPattern`/... types) duplicates ~15
enums for no semantic benefit at this stage.

---

## Scope rules

- **Top-level is whole-file.** Mutual recursion between top-level
  bindings is fine — the resolver collects every top-level def into the
  module scope before walking any expression. Within-module value
  dependency analysis (for let-poly groups) is Plan 4's job, not this
  pass's.
- **Block let-bindings are sequential.** `a = 1\nb = a` works;
  `a = a` does not. The walker evaluates the binding's value *before*
  pushing the local, so the inner `a` can't see itself.
- **Lambda params shadow.** `x = 1` followed by `f = x -> x` resolves
  the body's `x` to the lambda parameter, not the top-level `x`.
  Shadowing across nested scopes is fine; duplicate names in the *same*
  scope (two params, two let-bindings) are errors.
- **Pattern vars scope to one match arm.** `Some y -> y` binds `y` in
  that arm's body only. Outside the arm, `y` is unresolved.
- **Synthetic `self`.** Any `=` binding inside a `type` / `trait` /
  `impl` block gets a `self` local injected before its body is walked.
  Field declarations (`x : Float`) and bare variants don't receive
  `self`. `self` outside such a block is an `Unresolved` error.

See `src/resolve/scope.rs` for `ScopeStack` and the top-level
collection pass; `src/resolve/walker.rs` for everything else.

---

## Cross-module

Three `use` forms (`src/resolve/scope.rs::collect_imports`):

- `use Path` — registers `Path` in the file's module list. Qualified
  access `Path.name` resolves via longest-prefix matching.
- `use Path (a, b)` — cherry-picks. Each name goes into a local
  cherry table; `a` resolves directly to `Imported { module: Path,
  name: "a" }`. Cherries are validated against the source module's
  `expose` list — unexposed cherries error with `NotExposed`.
- `use Path as Alias` — registers `Alias -> Path` in the alias table.
  `Alias.name` resolves as if you'd written `Path.name`. The original
  `Path.name` is *not* in scope (per `docs/modules.md`).

Longest-prefix matching: `Std.IO.print` parses as
`FieldAccess(FieldAccess(Var("Std"), "IO"), "print")`. The walker
collects the chain into `[Std, IO, print]`, then tries the longest
prefix that matches a registered module: is `Std.IO` a `use`d module?
If yes, that's `Imported { module: [Std, IO], name: "print" }`. If
not, falls back to plain field access (resolved by the type checker
later).

Module cycles are rejected: a DFS over the import graph at the top of
`resolve_project` emits `ModuleCycle { members }` and short-circuits
the per-file pass. Mutual recursion within a single module is fine —
only the module-level graph forms a DAG.

---

## Errors

| Variant | When | Source |
| --- | --- | --- |
| `Unresolved { name }` | Name isn't a local, top-level def, ctor, cherry, alias, or imported module. | walker.rs |
| `DuplicateDefinition { name, first_span }` | Two `=` bindings or two types with the same name in one file. | scope.rs (top-level) |
| `DuplicateLocal { name }` | Two lambda params, two pattern vars in one arm, or two block let-bindings in one block. | walker.rs |
| `UnknownModule { path }` | `use Path` where `Path` isn't in the project's `ModuleSet`. | scope.rs (collect_imports) |
| `NotExposed { module, name }` | Cherry-picked name isn't in the source module's `expose` list. | scope.rs (validate_cherries) |
| `ModuleCycle { members }` | DFS found a back-edge in the import graph. | module_set.rs |

The resolver returns `Vec<Error>` — one unrelated bug shouldn't mask
the rest of the file's errors.

---

## What's deferred

- **Trait method dispatch** (`a + b` → which `impl Add`?) — Plan 4
  (type checker). Picking which `impl` to call needs type information.
- **Field-vs-method disambiguation** on `instance.x` — Plan 4. Today
  the resolver records `self` as a local on `self.x`, but doesn't
  decide whether `x` is a field or a zero-arg method.
- **Stdlib auto-import** — Plan 6 ships the stdlib modules; Plan 7
  wires them into the default `ModuleSet`. Until then, resolver tests
  that reference `Int`, `Float`, etc. need explicit stubs.
- **Project file loader** — Plan 7 walks `src/`, parses each `.i`,
  and assembles the `ModuleSet`. The resolver consumes the set; it
  doesn't read files itself.
- **Value-dependency analysis** for let-polymorphism — Plan 4 needs
  this for let-group generalisation. The resolver tracks references
  but doesn't graph them by binding.
- **Better spans on graph-level errors** — `ModuleCycle` and
  `NotExposed` currently emit `Span::new(0, 0)`. A follow-up should
  attribute them to one of the offending `use` decls.
