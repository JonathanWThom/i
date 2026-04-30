# i — Language Design

**Name:** `i`. One letter. The language is itself an exercise in the proposition that less is more.

**File extension:** `.i`. (Conflict note: GCC's preprocessor also emits `.i` files. Won't matter for a hobby/learning language; would matter for production tooling.)

**Status:** Initial design, brainstorm complete.

## Goals

A small, compiled, strongly typed language whose surface is as sparse as possible without giving up safety.

Three guiding values, in priority order:

1. **Aesthetic minimalism.** A program looks like the idea it expresses, not like the language it's written in. Few keywords. Few sigils. Few rules.
2. **Fits in your head.** The whole semantic core is small enough to hold at once. Reading new code shouldn't require a manual at your elbow. ("Afternoon-fast" if you've used Roc, Elm, or OCaml; longer if Hindley-Milner inference and row-typed effects are new.)
3. **Ergonomic for real work.** Sparse should *help* you write more code per minute, not turn into a puzzle.

Strong typing and compilation are non-negotiable. Roc-grade safety: pure with tracked effects, no null, no exceptions, total functions, exhaustive matching, distinct newtypes.

## Non-goals

- Dependent / refinement types (contradicts "fits in your head").
- Linear or affine types.
- Lazy evaluation.
- Inheritance, subtype polymorphism. (Traits handle ad-hoc polymorphism.)
- Macros / metaprogramming. (Possible later, not in v1.)

## Lexical surface

- **Comments:** `# line comment`. No block comments.
- **Identifiers:** lowercase = values and type variables. Uppercase = concrete types and variants.
- **Whitespace is significant.** Indentation marks blocks. (Same model as Python/Haskell layout.)
- **Newlines end expressions.** A trailing operator or open paren continues to the next line.

## The whole grammar (informal)

Four operators carry the structure of a program:

| Symbol | Meaning |
|---|---|
| `:`  | has type — `name : Type` |
| `=`  | bind value — `name = expr` |
| `->` | function — `args -> body` |
| `.`  | member access / namespace — `Type.name`, `instance.name` |

Arithmetic and comparison (`+`, `-`, `*`, `/`, `^`, `==`, `<`, …) desugar
to trait calls; `++` is concatenation; `!` marks an effectful call; `?`
is the early-exit postfix; parens group; brackets build list literals.
None of those are part of the binding surface, but they're all part of
what you read on the page. The full operator table lives in
[syntax.md § 11](../../syntax.md).

### Precedence and associativity

Listed loosest to tightest. Operators on the same row have equal
precedence; right-most column says how they associate.

| Rank | Operators                          | Associativity   |
|------|------------------------------------|-----------------|
| 1    | `->` (lambda body)                 | right; greedy   |
| 2    | `or`                               | function — paren-free call |
| 3    | `and`                              | function — paren-free call |
| 4    | `not`                              | function — paren-free call |
| 5    | `==`, `/=`, `<`, `<=`, `>`, `>=`   | non-associative |
| 6    | `++`                               | right           |
| 7    | `+`, `-` (binary)                  | left            |
| 8    | `*`, `/`                           | left            |
| 9    | `^`                                | right           |
| 10   | `-` (unary)                        | prefix          |
| 11   | function call (juxtaposition)      | left            |
| 12   | `.`, postfix `?`, postfix `!`      | left            |
| 13   | atoms: literals, identifiers, `()` | —               |

Two consequences worth saying out loud. Comparison doesn't chain:
`a < b < c` is a parse error, not a conjunction. Lambda `->` has the
lowest precedence in expression position, so a body like
`x -> a + b * c` is `x -> (a + (b * c))`, with `*` tighter than `+` as
expected.

### Lambda body termination

`->` is greedy on the right. The body extends rightward until one of:

- a newline at or below the indentation of the binding the lambda lives
  inside,
- a `,` at the enclosing call's argument-list depth,
- a `)` or `]` at the enclosing depth,
- a closing `match` arm boundary (the next pattern in an indented arm
  block),
- end of input.

This is what makes multi-arg lambdas pass through call argument lists
without parens:

```
nums.fold 0, acc x -> acc + x
```

`acc + x` extends to the end of the line; the `,` belongs to the outer
call, not the lambda. Same rule, different surface, with parens:

```
xs.map (x -> x + 1).reverse        # parens close the body before `.`
```

If you want a multi-argument lambda whose body needs to span multiple
lines, give it an indented block:

```
process =
    xs.fold initial, acc x ->
        cleaned = clean x
        acc.append cleaned
```

The block's indentation defines the body's right edge; the next sibling
binding closes it.

### Method chaining

`.` binds to the immediately preceding *atom* (an identifier, a paren
group, a literal, a list literal). It does **not** absorb a previously
consumed call's arguments. This is the rule that makes a written name
unambiguous:

```
nums.map double                    # OK — atom is `nums`, then `.map`, then call
nums.map double.filter pred        # parses as `nums.map (double.filter pred)`
(nums.map double).filter pred      # chain on a call result requires parens
```

The reading rule out loud: every `.` is a member access on whatever sits
immediately to its left. To "do this method, then that method on the
result," wrap the first call in parens. The price is one pair of parens
per chain link; the gain is that you never have to think about whether
your last token belongs to the method or the chain.

Pretty-printers and formatters are encouraged to break long chains
across lines:

```
nums
    .map double
    .filter positive
    .fold 0, acc x -> acc + x
```

Each `.method` aligns at the same column. Layout is a formatting
convention, not a parser rule.

The `!` postfix and the `?` postfix follow the same atom rule: they
attach to the call or expression immediately to their left. `print!`
attaches to `print`; `parseInt s?` attaches to `parseInt s` (the call,
treated as one expression at the call-juxtaposition level).

## Types

### Records

```
type Point
    x : Float
    y : Float
```

A `type` block is closed: every field lives in this one place. Members are introduced with `:` (type) or `=` (value or method). Inside a `type` block, the `Type.` prefix is implicit on each member name — `distance = ...` inside `type Point` binds `Point.distance`.

### Methods bind to instances

A function value bound at `Type.name` is a method. Methods receive an implicit `self`.

```
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5
```

The same `=` defines a top-level function when not nested under a `Type.`:

```
double = n -> n * 2
```

### Construction and update share syntax

```
p1 = Point(x = 0, y = 0)        # construct
p2 = p1(x = 5)                  # copy of p1 with x = 5
```

A type applied to kwargs constructs. An instance applied to kwargs produces a copy with overrides. Same surface, two related semantics.

Construction parens are *required* (not just grouping) because the inner `=` of a kwarg would otherwise collide with the outer binding `=`.

### Methods on sum types

A method written at the *type level* of a sum (outside any specific
variant's block) binds `self` to the matched value. The body almost
always opens with a `match` on `self`:

```
type Shape
    Circle
        radius : Float
    Rect
        width  : Float
        height : Float

    area = ->
        self match
            Circle r    -> 3.14159 * r^2
            Rect w, h   -> w * h
```

The `area = -> ...` form is a zero-argument method (other than the
implicit `self`). Calling it is `myShape.area`, no parens.

Methods on individual variants — `=` bindings nested *inside* a single
variant's field block — are out of v1. The per-variant binding raises
the question of how the variant's method type relates to the sum type
itself, which is the kind of design step that shouldn't slip in via the
back door. v1 sum-type methods always live at the type level and
dispatch with `match`.

### Sum types

Capitalized members of a `type` block are variants. Each variant can have its own fields.

```
type Color
    Red
    Green
    Blue

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float
```

**Single-payload shorthand.** A variant carrying exactly one anonymous payload may be written with a `:` instead of a field block:

```
type Maybe a
    None
    Some : a            # shorthand for `Some` with a block `value : a`

type Result a, e
    Ok : a
    Error : e
```

This is purely surface sugar. `Some : a` and the block form `Some\n    value : a` produce different access patterns — the shorthand exposes the payload as the variant itself (`Some x` matches `x : a` directly), while the block form exposes a named field (`Some s` gives `s.value : a`). Use the shorthand for variants where the payload has no useful name.

A "record" is a sum with one implicit case. The `Point` type above is sugar for a single-case sum named `Point`.

### Generics

Lowercase identifiers in type position are type variables. Uppercase are concrete types. No annotation needed to introduce a type variable — first use binds it.

```
type List a
    Empty
    Cons
        head : a
        tail : List a

type Maybe a
    None
    Some
        value : a

type Result a, e
    Ok    : a
    Error : e
```

### Newtypes

A `type` with one case wrapping one value is the idiomatic newtype. They are nominally distinct — a `UserId` cannot be passed where an `OrderId` is expected, even though both wrap `Int`.

```
type UserId  = Int
type OrderId = Int
```

(The single-line form `type Name = T` is sugar for a one-case wrapper.)

## Functions and calls

### Definition

`params -> body`. Lambda parameters are space-separated. No currying — `a b -> ...` is one 2-arg function, not a chain.

```
add  = a b -> a + b
ident = x -> x
```

Lambdas are first-class. The `->` form is a value anywhere an expression is expected.

```
nums.map x -> x * 2
nums.filter x -> x > 0
nums.fold 0, acc x -> acc + x
```

The space-vs-comma distinction is: spaces separate *parameters of one function*; commas separate *arguments at one call site*. Because they're different separators, multi-arg lambdas inside calls don't need parens.

### Calls

Default form: paren-free, comma-separated. Parens are grouping only — used when nesting forces it.

```
add 3, 4                        # plain call
p.distance p2                   # method call
add 3, (mul 4, 5)               # nest → parens for the inner
result = f Point(x = 0, y = 0)  # construction nested in a call
```

The reader rule: `,` is "next argument"; parens group. Space inside a lambda separates that lambda's parameters; it has no other meaning. Multi-arg lambdas pass through call argument lists without parens because the two separators don't collide:

```
nums.fold 0, acc x -> acc + x   # acc and x are lambda params; 0 and the lambda are call args
```

There is no separate "function-call" operator.

### Type signatures

A signature is `name : args -> result`, with effects appearing as `! Effect` between args and result if any. Signatures are usually inferred; you write them when documenting interfaces or when inference cannot find a type.

```
add      : Int, Int -> Int
print    : String ! IO -> Unit
distance : Point, Point -> Float
```

## Pattern matching

`expr match` introduces an indented block of `pattern -> body` arms. Exhaustive — the compiler rejects any `match` that does not cover every constructor.

```
area = shape ->
    shape match
        Circle r    -> 3.14 * r^2
        Rect w, h   -> w * h

unwrap = m default ->
    m match
        None      -> default
        Some v    -> v
```

`match` works on any sum type. Records can be destructured with the same syntax (one arm, the constructor name). Tuples destructure with parens-and-commas:

```
swap = pair ->
    pair match
        (a, b)  -> (b, a)
```

Tuple patterns also work in lambda parameters directly: `swap = (a, b) -> (b, a)`.

## Effects

The language is pure: a function that does not mention `!` in its type does no IO, no mutation, and no other observable effect.

Calls that perform effects are marked with `!` at the call site:

```
main =
    print! "hello"
    name = readLine!
    print! "hi " ++ name
```

The effect row is inferred and attached to the function's type:

```
print    : String ! IO -> Unit
readLine : ! IO -> String
greet    : String ! IO -> Unit       # inferred from body
add      : Int, Int -> Int           # pure, no `!`
```

You almost never write `!` in a signature. The compiler propagates it. The `!` at call sites is the visible thing.

Mutation is *not* a built-in effect. There is no `mut`, no rebinding, no in-place updates. For genuine mutable state, use a `Ref` type from the standard library; its operations are tracked as `! State` in the effect row.

### Effect polymorphism for higher-order functions

A function-typed parameter with no explicit effect annotation is **effect-polymorphic**: the compiler attaches an implicit, fresh effect row variable to it. The enclosing function's effect row then includes whatever effects flow through the callback. This is what makes `things.map fetch!` work without forcing the user to write row variables explicitly.

```
map : List a, (a -> b) -> List b
```

Reads as: `map` takes a list and a callback that returns `b`; `map` itself returns `List b`. The callback's effect row is implicit. If you call `map` with a pure callback, `map` is pure. If you call it with an effectful one, `map` inherits those effects:

```
nums.map x -> x * 2          # pure callback → map is pure
urls.map x -> fetch! x       # IO callback → this expression is ! IO
```

User-writable explicit effect-row variables (e.g., `(a -> b !e)` with `e` named) are not in v1. The implicit form covers the common case.

The same rule applies to `filter`, `fold`, `flatMap`, and any other higher-order function: function-typed parameters carry inferred, implicit effect rows.

#### Explicit-pure callbacks

One escape hatch from the implicit form: `! ()` written as a callback's
effect row pins the callback as **pure**. The empty row `()` admits no
effects at all.

```
trait Show a
    show : a -> String                # implicit row, but...
    show : (a -> String ! ())         # explicit form: cannot do IO
```

This matters in two situations. Trait methods that have to remain pure
to make sense (`Show.show` mustn't depend on the world; `Eq.eq`
mustn't have side effects) can pin the row. And HOFs whose semantics
depend on the callback being pure — a memoization cache keyed on the
input, an idempotent retry — can refuse to compile against an
effectful callback at the type level.

The implicit form stays the default. The vast majority of HOFs do the
right thing without anyone naming a row. `! ()` is the one explicit
spelling, and it's the only place a user can write an effect row on a
callback in v1.

## Errors

Errors are values. The standard idiom is `Result a e`. There are no exceptions, no panics, no implicit failure.

```
parseInt : String -> Result Int ParseError

parseInt "42" match
    Ok n      -> ...
    Error err -> ...
```

### `?` early-exit sugar

`expr?` is sugar for early-exit when an `expr` represents failure. It works on both `Result` and `Maybe`:

- If `expr : Result a e` and the enclosing function returns `Result _ e` (same error type), then `expr?` unwraps `Ok v` to `v` and propagates `Error e` outward.
- If `expr : Maybe a` and the enclosing function returns `Maybe _`, then `expr?` unwraps `Some v` to `v` and propagates `None` outward.

The two cases are syntactically identical; the type checker picks the right interpretation from `expr`'s type and the enclosing function's return type.

```
use Std.Float as F

parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys]  -> Ok Point(x = F.parse xs?, y = F.parse ys?)
        _         -> Error WrongShape

firstEven : List Int -> Maybe Int
firstEven = xs ->
    found = xs.find x -> x % 2 == 0
    Some (found?)               # ? on Maybe — early-returns None if not found
```

Without `?`, both bodies would need nested `match` for every fallible call. With `?`, error plumbing collapses to a single character. The compiler still verifies every error path — `?` is sugar, not an escape hatch.

There is no `return` keyword. The function's value is the value of its body expression. Early-exit on errors goes through `?`. Early-exit on success would go through restructuring the expression — and is rare enough that adding a keyword for it is not worth the surface cost.

## Totality

All functions are total: they terminate and handle every input. The compiler enforces:

- Exhaustive pattern matching.
- No partial standard library functions (`List.head : List a -> Maybe a`, never crashes).
- Termination checking on recursive functions (structural recursion is accepted; general recursion requires explicit `corecursive` annotation, TBD).

## Traits

Ad-hoc polymorphism is via traits. Operators desugar to trait methods.

```
trait Eq a
    eq : a, a -> Bool

trait Add a
    add : a, a -> a

# a + b   desugars to   Add.add a, b
# a == b  desugars to   Eq.eq  a, b
```

Implementations:

```
impl Add Int
    add = a b -> intAdd a, b      # primitive

impl Eq Point
    eq = a b -> a.x == b.x and a.y == b.y
```

A type can implement multiple traits. There is no inheritance.

## Modules

One file is one module. The first line declares the module name and what it exposes; everything else in the file is private.

```
module Geometry
    expose Point(..), Shape(..), distance, area

type Point
    x : Float
    y : Float

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

distance = a b -> ...
area     = shape -> ...

# private helper, not exposed
square = x -> x * x
```

**Type exports come in two forms.** `expose Point` exposes the *type* but not its constructors or fields — outside code can hold `Point` values but cannot construct or destructure them directly. `expose Point(..)` exposes the type *and* every constructor (and, for records, every field). The opaque form (`Point` without `(..)`) is what enables smart-constructor invariants:

```
module User
    expose User, make, name        # User is opaque

type User
    name : String
    age  : Int

# Only `make` can construct a User from outside this module.
make = name age ->
    age < 0 match
        True   -> Error InvalidAge
        False  -> Ok User(name = name, age = age)

name = u -> u.name
```

Outside `User`, `User(name = ..., age = ...)` is a type error; only `User.make` produces them. Field access through accessor functions (`User.name u`) still works for whatever the module re-exports.

Imports use `use`:

```
use Std.IO                              # whole module
use Std.IO (print, readLine)            # cherry-pick names
use Geometry as Geo                     # rename
```

## Program entry

A program is a module named `Main` containing a `main` value. `main` has type `! IO -> Unit` (or whatever effect row its body inherits).

```
module Main
    expose main

main =
    print! "hello, world"
```

## Worked examples

### A small program

```
module Main
    expose main

use Std.IO (print, readLine)

main =
    print! "what's your name?"
    name = readLine!
    print! "hi, " ++ name
```

### A library module

```
module Shapes
    expose Shape, area, scale

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

area = shape ->
    shape match
        Circle r    -> 3.14159 * r^2
        Rect w, h   -> w * h

scale = shape factor ->
    shape match
        Circle r    -> Circle(radius = r * factor)
        Rect w, h   -> Rect(width = w * factor, height = h * factor)
```

### Error handling end-to-end

```
module Parse
    expose parsePoint

use Std.Float as F

type ParseError
    WrongShape
    BadNumber                       # produced by F.parse

parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys]  -> Ok Point(x = F.parse xs?, y = F.parse ys?)
        _         -> Error WrongShape
```

The `?` after each `F.parse` call propagates an `Error ParseError` outward if the parse failed; otherwise unwraps to the `Float`. (Note: `F.parse` returns `Result Float Std.Float.ParseError`, which must be either compatible with the function's `ParseError` type or mapped via `mapError`. Real code would handle this; the example elides it for clarity.)

## Open questions

The following remain undecided. I've recorded recommendations next to each; the design is not finalized until they're picked.

- **Concurrency model.** Committed to "no shared mutable state across threads," but the mechanism is open. *Recommendation:* actor-based message passing for v2 (out of v1 scope; v1 is single-threaded).
- **Trait coherence.** Haskell-style global single-instance vs Rust-style orphan rules. *Recommendation:* Haskell-style global. One instance per (trait, type) pair, anywhere in the program. Simpler to teach; orphan-instance hazards are rare in a small ecosystem.
- **Compile target.** Out of v1 scope (v1 is interpreter only). For v2 the realistic options are bytecode VM, WASM, or LLVM native. *Recommendation:* bytecode VM for v2, native for v3.

## Explicitly out of v1

These have been considered and deferred:

- **Row polymorphism.** "Any record with an `x : Float` field" is genuinely sparse-feeling but expensive in the type checker. Revisit if the language outgrows nominal records. (User-writable explicit row variables for effects are also out — the implicit form covers HOFs.)
- **Macros / metaprogramming.** Conflicts with "fits in your head."
- **Linear / affine / dependent types.** Excluded earlier under safety choices.
- **Lazy evaluation.** Strict by default; `Lazy a` library type if needed.

## Tuples

`(a, b)` is a 2-tuple value with type `(A, B)`. `(a, b, c)` is a 3-tuple. Larger arities are allowed but discouraged — when you find yourself writing a 4-tuple, it's almost always clearer as a record.

```
pair  = (1, "hello")              # value, type (Int, String)
swap  = (a, b) -> (b, a)          # destructured in the lambda's pattern
fst   = (a, _) -> a
```

Tuples destructure with the same shape: `(x, y) -> ...` in a lambda, `(x, y) ->` in a `match` arm. There is no field access — tuples are positional. If you find yourself reaching for `.first`/`.second`, use a record instead.

A 1-tuple `(x)` is just `x` in parens (grouping, not a tuple). The shortest tuple is `(a, b)`.

## v1 standard library

The minimum stdlib v1 must ship:

- `Std.Bool` — `True`, `False`, `and`, `or`, `not`, `xor`
- `Std.Int` (= i64) — arithmetic via traits, `compare`, `parse : String -> Result Int ParseError`, `toFloat`, `toString`
- `Std.Float` (= f64) — same shape as `Int`; plus `sqrt`, `pow`, `sin`, `cos`, `tan`, `exp`, `ln`, `parse : String -> Result Float ParseError`
- `Std.Char` — `toUpper`, `toLower`, `isDigit`, `isAlpha`
- `Std.String` — `length`, `++` (concat), `split`, `toChars`, `fromChars`, `contains`, `trim`
- `Std.List a` — constructors `Empty` / `Cons`, ops `map`, `filter`, `fold`, `flatMap`, `concat`, `length`, `isEmpty`, `reverse`, `head : List a -> Maybe a`, `tail`, `take`, `drop`, `zip : List a, List b -> List (a, b)`, `find : List a, (a -> Bool) -> Maybe a`, `any`, `all`, `sort` (when `Ord a`), `sortBy` (custom key), `intercalate`
- `Std.Map k v` (requires `Ord k`) — `empty`, `insert`, `lookup : Map k v, k -> Maybe v`, `delete`, `member`, `keys`, `values`, `toList : Map k v -> List (k, v)`, `fromList`, `size`
- `Std.Set a` (requires `Ord a`) — `empty`, `insert`, `member`, `delete`, `toList`, `fromList`, `union`, `intersection`, `difference`, `size`
- `Std.Maybe a` — `None`, `Some`, `withDefault`, `map`, `andThen`, plus `?` sugar in the language
- `Std.Result a e` — `Ok`, `Error`, `withDefault`, `map`, `mapError`, `andThen`, plus `?` sugar in the language
- `Std.IO` — `print`, `println`, `readLine`, `readFile`, `writeFile`. All return `! IO`.
- `Std.Env` — `args : ! Env -> List String`, `var : String ! Env -> Maybe String`. Read-only access to the program environment; a third effect label `Env`, separate from `IO`.
- `Std.Ref a` — `make`, `get`, `set` for mutable cells; ops are `! State`

**Traits in the prelude** (auto-imported, not in any one module): `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Pow`, `Concat`, `Show`. Operators desugar to these — including `^` to `Pow.pow` (implemented on `Float`) and `++` to `Concat.concat` (implemented on `String` and `List a`). `Show.show : a -> String` is the conversion that `print!` and string-concatenation idioms rely on; every primitive type and most stdlib types have a derived `Show` impl.

Higher-order functions in the stdlib (`map`, `filter`, `fold`, `flatMap`, `find`, `any`, `all`, `sortBy`) follow the effect-polymorphism rule from the Effects section: their callback's effect row is implicit, so the same function works with pure or effectful callbacks.

Numeric tower stays minimal: only `Int` (i64) and `Float` (f64). Sized integers (`Int8`, `UInt32`, etc.) and arbitrary-precision can be added without breaking changes.

There is no `Std.Parse`. Parsing functions live on the type they parse into (`Std.Int.parse`, `Std.Float.parse`, etc.). Imports look like `use Std.Float (parse)` or `use Std.Float as F` and then `F.parse "3.14"`.

## What v1 actually is

Concretely, v1 is a parser, type checker, and interpreter (or simple compiler) that supports:

- The grammar above
- Records and sum types with generics
- Pattern matching with exhaustiveness checking
- Methods with implicit `self`
- Traits with explicit implementations
- Inferred types and inferred effect rows
- A minimal standard library: `Bool`, `Int`, `Float`, `String`, `List`, `Maybe`, `Result`, `IO` (print, readLine, readFile, writeFile)
- Modules with `expose` / `use`
- A `main` entry point

That is enough to write small command-line programs with full type and effect safety. Refinement features (linearity, dependent types, macros, row polymorphism) are explicitly out of scope for v1.
