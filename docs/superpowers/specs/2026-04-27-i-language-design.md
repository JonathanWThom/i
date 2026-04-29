# Pare — Language Design

**Working name:** Pare (rename freely). The character of the language is "pared back to essentials."

**Status:** Initial design, brainstorm complete.

## Goals

A small, compiled, strongly typed language whose surface is as sparse as possible without giving up safety.

Three guiding values, in priority order:

1. **Aesthetic minimalism.** A program looks like the idea it expresses, not like the language it's written in. Few keywords. Few sigils. Few rules.
2. **Fits in your head.** The whole semantic core can be learned in an afternoon. New users should be able to read code before they can write it.
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

There are exactly four binding operators a reader needs to know:

| Symbol | Meaning |
|---|---|
| `:`  | has type — `name : Type` |
| `=`  | bind value — `name = expr` |
| `->` | function — `args -> body` |
| `.`  | member access / namespace — `Type.name`, `instance.name` |

Plus literals (`42`, `3.14`, `"hi"`, `[1, 2, 3]`), parens for grouping, and the `!` effect marker. That is the entire surface.

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

`args -> body`. Multiple args are comma-separated. No currying — `a, b -> ...` is one 2-arg function, not a chain.

```
add  = a, b -> a + b
ident = x -> x
```

Lambdas are first-class. The `->` form is a value anywhere an expression is expected.

```
nums.map x -> x * 2
nums.filter x -> x > 0
```

### Calls

Default form: paren-free, comma-separated. Parens are grouping only — used when nesting forces it.

```
add 3, 4                        # plain call
p.distance p2                   # method call
add 3, (mul 4, 5)               # nest → parens for the inner
result = f Point(x = 0, y = 0)  # construction nested in a call
```

The reader rule: `,` is "next argument"; parens group. There is no separate "function-call" operator.

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

unwrap = m, default ->
    m match
        None      -> default
        Some v    -> v
```

`match` works on any sum type. Records can be destructured with the same syntax (one arm, the constructor name).

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

## Errors

Errors are values. The standard idiom is `Result a e`. There are no exceptions, no panics, no implicit failure.

```
parseInt : String -> Result Int ParseError

parseInt "42" match
    Ok n      -> ...
    Error err -> ...
```

A `?` postfix operator (TBD — not yet specified) may be added as sugar for "early-return on Error" inside functions whose return type is also a `Result`. Open question.

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
    add = a, b -> intAdd a, b      # primitive

impl Eq Point
    eq = a, b -> a.x == b.x and a.y == b.y
```

A type can implement multiple traits. There is no inheritance.

## Modules

One file is one module. The first line declares the module name and what it exposes; everything else in the file is private.

```
module Geometry
    expose Point, Shape, distance, area

type Point
    x : Float
    y : Float

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

distance = a, b -> ...
area     = shape -> ...

# private helper, not exposed
square = x -> x * x
```

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

scale = shape, factor ->
    shape match
        Circle r    -> Circle(radius = r * factor)
        Rect w, h   -> Rect(width = w * factor, height = h * factor)
```

### Error handling end-to-end

```
module Parse
    expose parsePoint

use Std.Parse (parseFloat)

type ParseError
    NotANumber
    WrongShape

parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys] ->
            x = parseFloat xs match
                Ok n     -> n
                Error _  -> return Error NotANumber
            y = parseFloat ys match
                Ok n     -> n
                Error _  -> return Error NotANumber
            Ok Point(x = x, y = y)
        _ ->
            Error WrongShape
```

(The `return` keyword above is provisional — see open questions.)

## Open questions / deferred

- **`?` early-return sugar for `Result`.** Desirable; not yet specified. Without it, error-plumbing code is verbose (see `parsePoint` above).
- **`return` keyword inside `match` arms.** Used in the example above as a way to escape the surrounding function on an error. May be replaced by `?` sugar; may be removed entirely; not yet decided.
- **Concurrency model.** Committed to "no shared mutable state across threads." Actor-based or STM-based — undecided.
- **Trait coherence rules.** Single global instance per type/trait pair (Haskell-style)? Or module-scoped instances (Rust-style orphan rules)? Not yet decided.
- **Numeric tower.** `Int`, `Float`, `Nat`, sized integers, arbitrary precision — not yet decided. Default to `Int = i64` and `Float = f64` for v1.
- **Record row polymorphism.** Functions that work on "any record with an `x` field" — useful and sparse-feeling, but adds complexity. Not in v1.
- **Compile target.** Native via LLVM? WASM? Bytecode VM? Not yet decided.
- **Standard library shape.** Just sketched (`Std.IO`, `Std.Parse`, `Maybe`, `Result`, `List`); needs full enumeration before implementation.

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
