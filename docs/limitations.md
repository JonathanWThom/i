# Limitations

What `i` v1 doesn't do, why, and what it would take to add. The source of
truth for "things that aren't in v1" and "things that aren't decided yet."
See [syntax.md](syntax.md) and [stdlib.md](stdlib.md) for what's in.

---

## 1. No row polymorphism

A function taking `{ name: Text }` accepts only that exact record type,
not "any record with a `name` field."
v1 inference is plain Hindley-Milner with traits. Row variables
noticeably complicate the unifier and the error messages.
Lifting this requires row-typed records in the type grammar and an
extension to the unifier. (User-writable explicit row variables for
effects are also out — the implicit form on HOF callbacks covers the
common case; see [effects.md § 7](effects.md).)

## 2. No macros or metaprogramming

No macro system, no quasiquotation, no compile-time evaluation.
`derive` is limited to built-in trait derivations.
Macros let library authors invent local dialects, which fights the goal
of one canonical reading per program.
Lifting this requires a macro expansion phase, a hygiene model, and a
stable AST surface.

## 3. No dependent, refinement, or linear types

Types don't depend on values, can't carry predicates
(`{ x: Int | x > 0 }`), and can't enforce single-use constraints.
Each is a research-grade extension. Any of them would push v1 past
"fits in your head."
Lifting any one means a new type theory, a new checker, and a new error
story.

## 4. No native code generation

The v1 implementation is a tree-walking interpreter. No bytecode VM,
no JIT, no AOT native compiler.
The v1 goal is "design correct, surface stable." Codegen locks in
semantics and is deferred.
Lifting this means a bytecode VM (v2) and later a native backend (v3).

## 5. No concurrency

The interpreter is single-threaded. No green threads, async/await,
channels, or shared-memory primitives.
Picking a concurrency model is load-bearing, so v1 ships
effects-with-handlers first (see [effects.md](effects.md)). That way v2
concurrency can be expressed as an effect.
Lifting this means choosing a model (the spec earmarks actors) and
implementing it on the v2 VM.

## 6. No stdlib outside the v1 list

No networking, async I/O, JSON, regex, date/time, cryptography, or
compression.
Each has design decisions worth a focused pass (effect boundaries,
error types, partial results) that v1 has not done.
Lifting any means a per-module spec round and adding under `Std.*`.

## 7. No FFI

No way to call Rust or C from `i`, and no way to expose `i` values to
a host runtime.
An FFI surface pins down memory representation, ownership, and effect
attribution. v1 doesn't commit to any of those before the VM exists.
Lifting this requires a stable value representation, a calling
convention, and a safety story for cross-boundary effects.

## 8. No package manager

No library registry, no `i add <package>`, no manifest for external
dependencies. A project is a single repo.
A package manager bakes in module identity, versioning, and trust. None
of those should lock in before the language surface is stable.
Lifting this requires a registry, a resolver, a lockfile format, and a
versioning policy.

## 9. `?` only works on `Result` and `Maybe`

The `?` postfix unwraps `Result` and `Maybe` values inside functions
that return the matching shape. It isn't a general early-exit operator
for arbitrary monads.
Generalizing `?` further needs either more ad-hoc desugaring or
`do`-notation, and both fight against keeping the language small.
Lifting this means adding `do`-notation and a `Monad` trait. On the
table for v2.

## 10. No `return`, `break`, or `continue`

No early-exit keywords. A function's value is the value of its body,
and loops are folds over lists.
Removing non-local control flow keeps every expression a value, which
the type and effect systems lean on.
Lifting this requires statement semantics or first-class delimited
continuations, and both would change the evaluation model.

## 11. No lazy evaluation

Every binding and argument is evaluated strictly. No `lazy`, no thunks,
no call-by-need.
Strictness keeps cost and effect ordering predictable, which matters
more here than the gains of laziness.
Lifting this means a `lazy` form or a library `Lazy a` type with
explicit `force`. The latter is the more likely path.

---

## Known TBDs

Items the spec has flagged as undecided. These get pinned before the
matching feature ships, not before v1 freezes.

- **Concurrency mechanism** — actor model is the working assumption;
  channels, structured concurrency, and effect-based scheduling are
  still candidates.
- **Trait coherence** — the rule for overlapping trait implementations
  across modules is not finalized; orphan rules and global coherence
  are both candidates.
- **Compile target after v1** — the v2 VM instruction set is not
  specified, and the v3 native backend (Cranelift vs. LLVM vs. custom)
  is open.
- **`corecursive` annotation** — syntax and checking rules for marking
  productive functions are sketched in [types.md § 8](types.md) but not
  pinned.
- **Rest patterns `[head, ...tail]`** — appear in examples; the
  exhaustiveness algorithm for them is not yet specified (see
  [patterns.md § 2](patterns.md)).
- **Guards in `match`** — `match ... if <expr> -> ...` is mentioned,
  but its interaction with exhaustiveness checking is undecided (see
  [patterns.md § 5](patterns.md)).
- **`Std.Env`** — reserved for reading program arguments and environment
  variables (see [building.md § 3](building.md)); its function list is
  not enumerated in [stdlib.md](stdlib.md).
- **Error type for `readFile` / `writeFile`** — variants of the I/O
  error type are not pinned; docs use a placeholder `IoError`.
