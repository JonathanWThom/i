# A tour of `i`

This is a tour, not a reference. It walks through the language so you can
read small programs and start writing your own. For the full rules on any
topic, follow the link at the end of each section.

I'm assuming you've used another statically typed language. You don't need
Haskell, OCaml, or Roc background. If a sentence mentions one of them and
you've never seen it, the example right below should be enough on its own.

---

## 1. Hello, world

The smallest runnable `i` program is two lines of declaration plus a tiny
body. See `examples/01-hello.i`.

```i
module Main
    expose main

main =
    print! "hello, world"
```

Three things to notice. First, every file declares its module on the first
line, and the indented `expose main` says the public surface of this module
is just one name. Second, a program runs by evaluating `main` in the `Main`
module. That's the entry point — no `int argc` ceremony. Third, `print!`.
The `!` isn't punctuation; it's the effect marker. It means "this call does
something effectful," in this case IO.

Next: how `=` and `:` introduce values and types. For lookup-style coverage
of every form here, see [syntax.md](syntax.md).

---

## 2. Values and bindings

`=` binds a name to a value. There is no `let`, no `var`, no `const`. You
also can't rebind: every binding is immutable for its scope.

```i
greeting = "hello"
answer   = 42
pi       = 3.14159
```

If you want to write down a type, use `:`:

```i
greeting : String
greeting = "hello"
```

Or fuse them on one line:

```i
greeting : String = "hello"
```

You rarely need the annotation. Inference handles local bindings. You'll
mostly write `:` to document a function's interface (next section), or when
inference can't decide on its own.

There's no mutation. To "change" a value, you make a new one. The rare case
where you genuinely need mutable state has `Ref` in the standard library;
those operations are tracked as a `State` effect.

For the full operator and binding rules, see [syntax.md](syntax.md).

---

## 3. Functions

A function is `args -> body`. Multiple parameters are space-separated. There's
no currying: `a b -> ...` is one two-argument function, not a chain.

```i
double = n -> n * 2
add    = a b -> a + b
ident  = x -> x
```

Calls don't use parens. You write the function, a space, and the arguments
separated by commas:

```i
double 21           # → 42
add 3, 4            # → 7
```

Parens are only for grouping. You reach for them when nesting one call inside
another:

```i
add 3, (double 4)   # → 11
```

The asymmetry is on purpose. Spaces separate *parameters of one function*;
commas separate *arguments at one call site*. Because they're different
separators, multi-arg lambdas pass through call argument lists without parens.

Lambdas are first-class. Anywhere an expression fits, `args -> body` fits:

```i
nums.map x -> x * 2
nums.filter x -> x > 0
nums.fold 0, acc x -> acc + x
```

If you want to write a function's type, the form is `args -> result`. Type
signatures use commas — the same separator the call site uses:

```i
double : Int -> Int
add    : Int, Int -> Int
```

`add` takes two `Int`s, not an `Int` that returns `Int -> Int`.

Next: bundling values into records. For the formal rules on functions and
type signatures, see [types.md § 5](types.md) and [syntax.md § 4](syntax.md).

---

## 4. Records

A record is a `type` block with named fields. Inside the block, `:` introduces
a field and `=` introduces a method or constant. Methods get an implicit
`self`. You don't put it in the parameter list; it's there because the
binding lives inside a `type` block.

```i
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5
```

Inside the `type` block, `distance` is short for `Point.distance`; the type
prefix is implicit on every member. (`^` is exponentiation.)

To construct, apply the type to keyword arguments. The parens are required.
Without them, `Type x = value, ...` is syntactically ambiguous with starting
a new value binding:

```i
p1 = Point(x = 0, y = 0)
p2 = Point(x = 3, y = 4)
```

Methods are called with a dot:

```i
p1.distance p2      # → 5.0
```

To make a copy with overrides, apply an instance to kwargs the same way:

```i
p3 = p1(x = 5)      # copy of p1 with x = 5
```

Same surface, two related operations. Type with kwargs constructs; instance
with kwargs updates. There's no inheritance; types compose through traits,
not subclassing.

For the deep dive, see [types.md](types.md).

---

## 5. Sum types and pattern matching

A `type` block can list capitalized variants. Each variant is its own
constructor and may carry its own fields.

See `examples/03-shapes.i`.

```i
module Main
    expose main

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

main =
    s = Circle(radius = 5.0)
    print! "area: " ++ show (area s)
```

Two new things. `Shape` has two cases, `Circle` and `Rect`, each with its
own fields. And `shape match` is the pattern-match form: write the value,
then `match`, then an indented block of `pattern -> body` arms. (`show`
is a stdlib function for converting displayable values to `String`.)

The compiler checks every `match` for exhaustiveness. Forget to handle
`Rect` and the program won't build; the error names the case you missed.
No fallthrough, and no default needed when you've covered the cases by name.

`area` here is a free function. The same idea written as a method on
`Shape` looks like this:

```i
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

    area = ->
        self match
            Circle r    -> 3.14159 * r^2
            Rect w, h   -> w * h
```

Inside the `type` block, `area =` binds `Shape.area`, and the implicit
`self` is the matched shape. You'd then call `s.area` instead of
`area s`. Pick the form that reads better — most code uses methods when
the function is logically a piece of the type's interface, and free
functions when it's a derived calculation.

For all the pattern kinds (literals, lists, nested), see [patterns.md](patterns.md).

---

## 6. Maybe and Result

`i` has no null and no exceptions. Absence is `Maybe a`. Failure is
`Result a e`. Both live in the standard library and both work with `match`.

```i
type Maybe a
    None
    Some
        value : a

type Result a, e
    Ok    : a
    Error : e
```

A function that might fail returns a `Result`:

```i
parseInt : String -> Result Int ParseError
```

You then handle both arms:

```i
parseInt "42" match
    Ok n      -> ...
    Error err -> ...
```

When you have several fallible calls in a row, `?` collapses the boilerplate.
`expr?` reads: if `expr` represents failure, return the failure from the
enclosing function; otherwise unwrap the success value.

`?` works on both `Result` and `Maybe`. Inside a function returning
`Result _ e` it propagates `Error e`. Inside a function returning `Maybe _`
it propagates `None`. The two cases are syntactically identical; the type
checker picks the right interpretation from `expr`'s type and the enclosing
function's return type.

```i
parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys]  -> Ok Point(x = parseFloat xs?, y = parseFloat ys?)
        _         -> Error WrongShape

firstEven : List Int -> Maybe Int
firstEven = xs ->
    found = xs.find x -> x % 2 == 0
    Some (found?)               # ? on Maybe — early-returns None if not found
```

If either `parseFloat` returns `Error`, `parsePoint` returns that error
immediately. Otherwise the unwrapped values feed into the `Point`
constructor.

The compiler still checks every error path. `?` is sugar, not an escape hatch.

For the full safety story, see [types.md](types.md) and the stdlib
[Maybe](stdlib.md) and [Result](stdlib.md) entries.

---

## 7. Effects

A function whose type doesn't mention `!` does no IO, no mutation, no other
observable side effect. The compiler enforces this; it's not a guideline.

Mark effectful call sites with `!`:

```i
main =
    print! "what's your name?"
    name = readLine!
    print! "hi, " ++ name
```

See `examples/02-greet.i` for the runnable version.

You almost never write `!` in a type signature. The compiler infers the
effect row and propagates it up the call graph:

```i
print    : String ! IO -> Unit       # in stdlib
readLine : ! IO -> String            # in stdlib
greet    : String ! IO -> Unit       # inferred from body
add      : Int, Int -> Int           # pure
```

The only visible thing is `!` at the call site. If you accidentally call
`print!` from a function you thought was pure, the inferred type now carries
`! IO`, and you'll find out the moment you try to use that function
somewhere a pure value is expected.

For the deep dive, see [effects.md](effects.md).

---

## 8. Lists

Lists are written with square brackets. The standard operations are `map`,
`filter`, and `fold`.

See `examples/04-list-map.i`.

```i
module Main
    expose main

main =
    nums = [1, 2, 3, 4, 5]
    doubled = nums.map x -> x * 2
    print! "doubled: " ++ show doubled
```

`nums.map x -> x * 2` is two things glued together. `nums.map` is a method
call with implicit `self = nums`, and `x -> x * 2` is the lambda passed in.

A few more, for flavor:

```i
positives = nums.filter x -> x > 0
total     = nums.fold 0, acc x -> acc + x
```

`fold` takes the initial accumulator and a two-arg function that combines
the accumulator with the next element. Because parameters of the lambda are
space-separated and call arguments are comma-separated, the lambda passes
through the call argument list without parens. There's no `for` loop in `i`.
To walk a list, reach for `map`, `filter`, or `fold`.

When you do need to group two values together as one — say, an index and an
element from `zip` — use a tuple: `(a, b)` is a two-tuple value with type
`(A, B)`. Tuples destructure with the same shape. See
[syntax.md § 7](syntax.md) and the spec for the full rule.

The standard library never crashes on an empty list. `head` returns `Maybe a`,
not `a`. For the full set of operations, see
[stdlib.md § `Std.List`](stdlib.md).

---

## 9. Modules

One file is one module. The first non-blank line declares the module name and
what it exposes. Everything else in the file is private.

```i
module Geometry
    expose Point(..), distance

type Point
    x : Float
    y : Float

distance = a b ->
    ((a.x - b.x)^2 + (a.y - b.y)^2)^0.5

# private helper, not exposed
square = x -> x * x
```

A type is exposed in one of two forms. `expose Point` exposes the *type* but
not its constructors or fields — outside code can hold `Point` values but
not construct or destructure them. `expose Point(..)` exposes the type *and*
all of its constructors and fields. The opaque form is what enables
smart-constructor invariants; the `(..)` form is the everyday choice when
you want callers to construct values directly.

To use names from another module, write `use`:

```i
use Std.IO                              # whole module — Std.IO.print, etc.
use Std.IO (print, readLine)            # cherry-pick names
use Geometry as Geo                     # rename for local use
```

The `Main` module with its `main` value is the program entry point. That's
why every example so far has started with `module Main` and `expose main`.

For the full module rules, see [modules.md](modules.md).

---

## 10. Where to go next

That's most of the language on one page. The reference docs go deeper on
the corners I skipped — operator precedence, exhaustiveness rules, what
the type checker actually does, how the effect system handles
higher-order functions, and the rest. Pick the depth you want:

- [Syntax reference](syntax.md) — every form, every operator, look-up style
- [Type system](types.md) — records, sums, generics, traits, totality
- [Effect system](effects.md) — what `!` actually does
- [Pattern matching](patterns.md) — every kind of pattern, with examples
- [Modules](modules.md) — file layout, `expose`, `use`, project structure
- [Standard library](stdlib.md) — every type and function in v1
- [Building and running](building.md) — installing the toolchain, the CLI
- [Limitations](limitations.md) — what v1 explicitly doesn't do

If you only have time for one more, read [syntax.md](syntax.md). It's the
densest map of the surface. Everything else goes deep on a single axis.
