# Progress

## Phase 0: Setup
- [x] Cargo skeleton, README, LICENSE — Plan 1, Task 1-2

## Phase 1: Documentation
- [x] Tour
- [x] Syntax reference
- [x] Type system manual
- [x] Effect system manual
- [x] Pattern matching reference
- [x] Standard library reference
- [x] Modules
- [x] Build/run/CLI (forward-looking)
- [x] Limitations
- [x] Cross-link pass

## Phase 1.6: Spec revisions
- [x] Precedence + associativity table
- [x] Lambda body termination rule
- [x] Method chaining rule
- [x] `self` in sum-type method blocks
- [x] Explicit-pure callback `! ()`
- [x] Marketing-claims softening + smaller items

## Phase 2: Implementation — Plan 2 (lexer + parser) DONE
- [x] Foundation: span, error, token, lexer scaffold (Tasks 1-3)
- [x] Lexer: punctuation, idents/keywords, numbers, strings, comments (Tasks 4-8)
- [x] Lexer refactor: extract token scanners (Task 8.5)
- [x] Lexer: layout (newline + line continuation), indent/dedent, mixed-tabs error (Tasks 9-11)
- [x] CI on every push: fmt + clippy + test (Task 11.5)
- [x] Lexer corpus snapshots over `examples/` (Task 12)
- [x] Makefile + husky pre-commit (Task 12.5)
- [x] AST data types (Task 13)
- [x] AST custom Display (Task 14)
- [x] Parser scaffold (Task 15)
- [x] Atom expressions (Task 16)
- [x] Pratt expressions: arithmetic + comparison (Task 17)
- [x] Logical ops + lambda (Task 18)
- [x] Calls + postfix `.` `!` `?` (Task 19)
- [x] Construction + record update (Task 20)
- [x] Patterns (Task 21)
- [x] Match expressions (Task 22)
- [x] Type expressions (Task 23)
- [x] Bindings and block bodies (Task 24)
- [x] Type, trait, impl declarations (Task 25)
- [x] Module header, use, and top-level dispatch (Task 26)
- [x] AST/parser refactor for file length and modularity (Task 26.5)
- [x] Parser error tests (Task 27)
- [x] Multi-line lambda body (Task 27.5)
- [x] Lambda-as-arg, upper in use cherry (Task 27.6)
- [x] Parser corpus snapshots (Task 28)
- [x] Pretty printer (Task 29)
- [x] Round-trip property test (Task 30)
- [x] Document testing strategy (Task 31)

## Phase 3: Implementation — Plan 3 (name resolution) DONE
- [x] Resolver scaffold + data model (Task 1)
- [x] Top-level collection + duplicate detection (Tasks 2-3)
- [x] Var, Ctor, expression walker (Tasks 4-5)
- [x] Locals: lambda, patterns, blocks, self (Tasks 6-9)
- [x] Type expressions (Task 10)
- [x] Cross-module: use, cherry, alias, cycles, exposure (Tasks 11-15)
- [x] Corpus + integration tests (Tasks 16-17)
- [x] Resolver documentation (Task 18)

## Phase 4: Implementation — Plan 4 (type checker, HM core) DONE
See [`2026-05-22-plan-4-type-checker.md`](2026-05-22-plan-4-type-checker.md). Scope: Hindley-Milner inference, records, sums, patterns, exhaustiveness, primitive operators, list literals. Defers traits, effects, totality, `?` operator to later plans.
- [x] Scaffold + Ty/Scheme/Subst + unification (Tasks 1-3)
- [x] Inference context, literals, variables, lambdas, applications (Tasks 4-9)
- [x] Blocks, generalisation, annotations (Tasks 10-11)
- [x] Type registry, newtypes, records, sums (Tasks 12-13)
- [x] Construction, update, field access, methods, constructors (Tasks 14-16)
- [x] Patterns and match with exhaustiveness (Tasks 17-21)
- [x] Primitive operators and list literals (Tasks 22-23)
- [x] Pretty-printing, corpus snapshots, end-to-end test (Tasks 24-26)
- [x] Documentation (Task 27)

## Later v1 phases
- [ ] Traits + operator desugaring — Plan 5 (TBD)
- [ ] Effects (IO/State/Env, HOF effect-polymorphism, `?`) — Plan 6 (TBD)
- [ ] Totality / termination checking — Plan 7 (TBD)
- [ ] Interpreter — Plan 8 (TBD)
- [ ] Stdlib — Plan 9 (TBD)
- [ ] Driver / CLI — Plan 10 (TBD)
- [ ] Golden test harness — Plan 11 (TBD)

## After v1
- [ ] Formatter (`i fmt`) — Plan 12 (TBD)
- [ ] Bytecode VM (v2) — Plan 13 (TBD)
- [ ] Native codegen (v3) — Plan 14 (TBD)
- [ ] Language server — Plan 15 (TBD)
- [ ] Editor plugins — Plan 16 (TBD)
- [ ] REPL — Plan 17 (TBD)
- [ ] Doc generator — Plan 18 (TBD)
- [ ] Concurrency (actors) — Plan 19 (TBD)
- [ ] Package manager — Plan 20 (TBD)
- [ ] Stdlib expansion — Plan 21 (TBD)
- [ ] String interpolation — Plan 22 (TBD)
