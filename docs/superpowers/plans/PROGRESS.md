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

## Phase 2: Implementation — in progress (Plan 2)
- [x] Foundation: span, error, token, lexer scaffold (Tasks 1-3)
- [x] Lexer: punctuation, idents/keywords, numbers, strings, comments (Tasks 4-8)
- [x] Lexer refactor: extract token scanners (Task 8.5)
- [x] Lexer: layout (newline + line continuation), indent/dedent, mixed-tabs error (Tasks 9-11)
- [x] CI on every push: fmt + clippy + test (Task 11.5)
- [x] Lexer corpus snapshots over `examples/` (Task 12)
- [x] Makefile + husky pre-commit (Task 12.5)
- [x] AST data types (Task 13)
- [ ] AST custom Display (Task 14)
- [ ] Parser: atoms, Pratt expressions, calls/postfix, construction (Tasks 15-20)
- [ ] Parser: patterns, match, types (Tasks 21-23)
- [ ] Parser: bindings, type/trait/impl decls, modules (Tasks 24-26)
- [ ] Parser error tests + corpus snapshots (Tasks 27-28)
- [ ] Pretty printer + round-trip property test (Tasks 29-30)
- [ ] Document testing strategy (Task 31)

## Later phases
- [ ] Name resolution — Plan 3 (TBD)
- [ ] Type checker — Plan 4 (TBD)
- [ ] Interpreter — Plan 5 (TBD)
- [ ] Stdlib — Plan 6 (TBD)
- [ ] Driver / CLI — Plan 7 (TBD)
- [ ] Golden test harness — Plan 8 (TBD)
