# Plan 1.6 — Grammar, precedence, and the parser-relevant gaps

**Goal:** Pin down the parser-shaped corners of the spec before Plan 2 (lexer
+ parser) starts. Six items came out of a second external review and a
cover-to-cover read of the docs. Each is small on its own. Together they
remove the ambiguity that would otherwise show up the moment the parser
sees real code.

**Why now:** the implementation hasn't started, so this is still a free
edit. After the parser ships, every one of these becomes a breaking
change. I'd rather spend an afternoon on prose than a week on a
deprecation window.

## Items

1. **Precedence and associativity table.** The spec leaned on "operators
   desugar to traits, the trait dispatch handles it" without saying where
   `+` sits relative to `*`, what `^` does on the right, or whether
   comparison chains. Add an explicit table. Lock in standard math
   precedence; comparisons non-associative; `++` higher than comparison
   but lower than arithmetic. `?` is postfix at the same tightness as
   `.`. Lambda `->` is the lowest-precedence thing in expression position.

2. **Lambda body termination.** `nums.fold 0, acc x -> acc + x` parses
   today by appeal to "spaces vs commas" — but the actual rule is that
   the lambda body ends when the *containing* construct ends. I'll write
   that rule down: the body extends right until an unindented newline, a
   `,` or `)` at the enclosing depth, or EOF. Parens around the lambda
   extend the body inside them.

3. **Method chaining.** `nums.map double` works. `nums.map double.filter
   pred` is genuinely ambiguous — does `double.filter` parse as a method
   on `double`, or is `.filter` applied to the result of `nums.map
   double`? Pick the latter is wrong (it breaks "method on the previous
   atom"); pick the former is right but readers won't know that without
   the rule written down. Rule: `.` binds to the immediately preceding
   *atom* (identifier, paren group, literal). To chain on a call result,
   wrap the call: `(nums.map double).filter pred`.

4. **`self` in sum-type method blocks.** Spec covers `self` for records
   but doesn't say what it means in a `type` block with variants. Decide
   and document: `self` is the matched value at the type level, and a
   method written outside any specific variant dispatches via `match`.
   Methods declared inside a single variant's block bind `self` to that
   variant's payload.

5. **Explicit-pure callback annotation.** Effect polymorphism is fine
   for the common case, but trait methods that *must* be pure (e.g.
   `Show.show`, the key function passed to a hash-keyed cache) need a
   way to pin the callback. Add `(a -> b ! ())` as the spelling for
   "this callback has the empty effect row." User-named row variables
   stay out of v1; this is the one explicit form.

6. **Marketing claims softened in `README.md` and `tour.md`.** "Four
   operators" is true at the level of *binding* operators, but a reader
   counting things on the page will land closer to a dozen (arithmetic,
   comparison, `!`, `?`, list literals, parens-as-grouping, the call
   juxtaposition). "Fits in an afternoon" is overconfident for someone
   without an ML background. I want the framing to match what readers
   will actually experience: small, but not trivially small.

## Smaller items rolled in

While the diff is open:

- `building.md` gains a worked diagnostic for the cross-type `?`
  mismatch (using `?` on a `Result` from a `Maybe`-returning function).
- `stdlib.md` gains `Std.Env` (`args`, `var`); a third effect label
  `Env` is added in `effects.md § 4`. `limitations.md` drops the
  "Std.Env reserved" TBD.
- `limitations.md` documents the v1 workaround for general recursion
  (fold over a list, or use `Std.Ref` for a bounded loop) until the
  `corecursive` annotation is pinned.
- `tour.md § 5` mentions the method form for `area` alongside the
  free-function form, since both shapes are idiomatic and readers will
  see both in real code.

## Files affected

**Spec:** `docs/superpowers/specs/2026-04-27-i-language-design.md` —
precedence section; lambda termination clarification; method chaining
rule; `self` in sum-type blocks; explicit-pure callback syntax.

**Docs:**
- `syntax.md` — precedence and associativity table; lambda termination
  subsection; method chaining subsection; explicit-pure callback entry.
- `tour.md` — method-form alternative in §5; softer framing in §1.
- `types.md` — note on `self` in sum-type blocks; tiny humanize pass
  if anything stands out.
- `effects.md` — explicit-pure callback subsection in §7; `Env` mention
  in §4.
- `stdlib.md` — new `Std.Env` module section.
- `building.md` — `?` cross-type diagnostic example; humanize pass on
  the error catalog.
- `limitations.md` — drop `Std.Env` TBD; add `corecursive` workaround.
- `README.md` — softer framing of "four operators" / "fits in an
  afternoon."
- `modules.md`, `patterns.md` — humanize pass only where AI tells remain
  after the user's earlier sweep.

**Examples:** none (no syntax changes that affect existing examples).

## Approach

One feature branch, several commits grouped by topic so the diff is
reviewable. Spec first. Then `syntax.md` (the lookup reference everything
else points to). Then the prose docs. Humanize pass last, so I'm editing
voice on already-correct content rather than two things at once.

## Acceptance

After Plan 1.6:

- The spec has a precedence table that any parser implementer can use
  without guessing.
- The three "where does this token bind?" questions (lambda body end,
  method chain, `self` in sums) each have a written rule and an example.
- `(a -> b ! ())` is in the spec and documented in `syntax.md` and
  `effects.md`. The implicit form remains the default.
- `README.md` and `tour.md` no longer over-claim. The reader knows what
  they're getting before they invest the afternoon.
- Every docs file has been read for AI tells; surviving prose sounds
  like a person wrote it.
- `Std.Env` exists in `stdlib.md`; the `Env` effect label exists in
  `effects.md`; the v1 workaround for general recursion is documented.

After this plan, **Plan 2 (lexer + parser) starts.**
