# Plan 1.5 — Spec Revisions from External Review

**Goal:** Apply six design revisions surfaced by an external review of the docs. The revisions are all spec-level changes; the implementation has not started, so this is the right time to make them. Plan 2 (lexer + parser) starts against the revised contract.

**Why:** A docs-cover-to-cover read uncovered real friction (paren-around-multi-arg-lambdas in folds, `?` not working on `Maybe`) and one structural hole (effect polymorphism for higher-order functions). The cost of fixing now is hours of doc edits; the cost of fixing after the parser ships is a breaking change to the language.

## Revisions

1. **Lambda parameter separator changes from `,` to space.** `add = a b -> a + b`. Calls still use `,` (`add 3, 4`). Eliminates the parens-around-lambda dance in folds: `nums.fold 0, acc x -> acc + x` now parses cleanly. Type signatures keep commas (`add : Int, Int -> Int`) — the asymmetry exists already at the single-arg case (`x -> ...` has no separator).

2. **Higher-order function parameters are effect-polymorphic by default.** A function-type parameter with no explicit effect annotation gets an implicit fresh effect row variable. The HOF's own effect row is the union of those callback rows plus the body's effects. This makes `things.map fetch!` work without exposing user-writable row variables.

3. **`?` postfix works on `Maybe` as well as `Result`.** Inside a function whose return type is `Maybe a`, `expr?` on a `Maybe` value either unwraps `Some v` or returns `None`. Symmetric with the existing `Result` behaviour.

4. **Opaque type exports.** `expose Point` exposes the type but not its constructors/variants. `expose Point(..)` exposes the type and all constructors. Standard ML/Haskell move; enables smart-constructor invariants.

5. **Tuples are in v1.** `(a, b)` is a 2-tuple value with type `(A, B)`. Destructuring patterns `(x, y) -> ...`. `Std.Pair` removed; `zip` returns `List (a, b)` again.

6. **Stdlib expansion.**
   - `Std.List` gains `find`, `any`, `all`, `flatMap`, `concat`, `sort`, `sortBy`, `intercalate`, `isEmpty`.
   - New module `Std.Map` (requires `Ord k`): `Map k v`, `empty`, `insert`, `lookup`, `delete`, `keys`, `values`, `toList`, `fromList`, `size`.
   - New module `Std.Set` (requires `Ord a`): `Set a`, `empty`, `insert`, `member`, `delete`, `toList`, `fromList`, `union`, `intersection`, `difference`, `size`.

## Files affected

**Spec:** `docs/superpowers/specs/2026-04-27-i-language-design.md` — all six revisions.

**Docs:**
- `tour.md` — every multi-arg lambda example; tuples appear in §8 Lists
- `syntax.md` — Calls (multi-arg lambda parens rule changes), Lambdas, Patterns (tuple), Modules (`(..)`), `?` (Maybe + Result)
- `types.md` — §4 Sum types (rest unchanged), §9 No null/no exceptions (`?` on Maybe), traits (effect-poly HOFs)
- `effects.md` — new section on effect polymorphism in higher-order functions
- `patterns.md` — tuple patterns
- `stdlib.md` — drops `Std.Pair`, expands `Std.List`, adds `Std.Map` and `Std.Set`
- `modules.md` — `expose Type(..)` syntax, opacity rules
- `limitations.md` — remove "no tuples" entry; remove `Pair` from "Known TBDs"

**Examples:**
- `examples/04-list-map.i` — uses single-arg lambda, no change needed
- `examples/06-tree.i` — uses multi-arg constructor pattern (already space-separated), no lambda change needed
- `examples/07-result.i` — uses `?` on Result, no change to that; refresh comments
- `examples/05-effects.i` — no multi-arg lambdas, no change
- Add a small example demonstrating tuples and effect-polymorphic map (`examples/09-effect-map.i`)

## Approach

Apply the changes in one focused pass. Spec first, then docs, then examples, then a final consistency check. Commit at logical points. Single feature branch.

## Acceptance

After Plan 1.5:

- Spec describes all six revisions in their canonical sections
- Every doc reflects the new rules; no stale references to `, ` in lambda definitions, `Pair`, or "no tuples"
- Every existing example file parses under the new rules (verified by careful reading; lexer doesn't exist yet)
- The new `examples/09-effect-map.i` demonstrates effect polymorphism end-to-end
- `limitations.md` and the spec's "Open questions" / "Out of v1" sections reflect the new scope

After this plan, **Plan 2 (lexer + parser) starts.**
