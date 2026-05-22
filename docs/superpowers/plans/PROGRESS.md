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
- [x] AST custom Display (Task 14)
- [x] Parser scaffold (Task 15)
- [x] Atom expressions (Task 16)
- [x] Pratt expressions: arithmetic + comparison (Task 17)
- [x] Logical ops + lambda (Task 18)
- [x] Calls + postfix `.` `!` `?` (Task 19)
- [x] Construction + record update (Task 20)
- [x] Patterns (Task 21)
- [ ] Parser: match, types (Tasks 22-23)
- [ ] Parser: bindings, type/trait/impl decls, modules (Tasks 24-26)
- [ ] Parser error tests + corpus snapshots (Tasks 27-28)
- [ ] Pretty printer + round-trip property test (Tasks 29-30)
- [ ] Document testing strategy (Task 31)

## Later v1 phases
- [ ] Name resolution — Plan 3 (TBD)
- [ ] Type checker — Plan 4 (TBD)
- [ ] Interpreter — Plan 5 (TBD)
- [ ] Stdlib — Plan 6 (TBD)
- [ ] Driver / CLI — Plan 7 (TBD)
- [ ] Golden test harness — Plan 8 (TBD)

## After v1
- [ ] Formatter (`i fmt`) — Plan 9 (TBD)
- [ ] Bytecode VM (v2) — Plan 10 (TBD)
- [ ] Native codegen (v3) — Plan 11 (TBD)
- [ ] Language server — Plan 12 (TBD)
- [ ] Editor plugins — Plan 13 (TBD)
- [ ] REPL — Plan 14 (TBD)
- [ ] Doc generator — Plan 15 (TBD)
- [ ] Concurrency (actors) — Plan 16 (TBD)
- [ ] Package manager — Plan 17 (TBD)
- [ ] Stdlib expansion — Plan 18 (TBD)
- [ ] String interpolation — Plan 19 (TBD)
