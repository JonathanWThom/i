# A tour of `i`

This is a tour, not a reference. It walks through `i` end-to-end so you can read
small programs and start writing your own. For the full rules of any topic
shown here, follow the link at the end of each section to the matching manual.

The reader this tour assumes: someone who has used another statically typed
language. You don't need to know Haskell, OCaml, or Roc — but if a sentence
mentions one and you've never seen it, that's fine, the example just below
will speak for itself.

---

## 1. Hello, world

The smallest runnable `i` program is three lines of declaration plus one line
of body. See `examples/01-hello.i`.

```i
module Main
    expose main

main =
    print! "hello, world"
```

Three things to notice. First, every file declares its module on the first
line. The indented `expose main` says: this module's public surface is one
name, `main`. Second, a program runs by evaluating the `main` value of the
`Main` module — that's the entry point, no separate `int argc` ceremony.
Third, `print!`. The `!` is not punctuation; it's the effect marker. It says
"this call performs an effect" — here, IO.

Next: how `=` and `:` introduce values and types.

---

## 2. Values and bindings

In `i`, `=` binds a name to a value. There is no `let`, no `var`, no `const`.
There is no rebinding either — every binding is immutable for its scope.

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

You almost never need the annotation. Inference handles local bindings. The
common reason to write `:` is to document a function's interface (next
section) or to disambiguate when inference can't decide on its own.

There is no mutation. To "change" a value, you produce a new one. (For the
rare case you genuinely need mutable state, the standard library has `Ref`;
its operations are tracked as a `State` effect.)

For the full operator and binding rules, see [syntax.md](syntax.md).

---

## 3. Functions

A function is `args -> body`. Multiple arguments are comma-separated. There is
no currying — `a, b -> ...` is one two-argument function, not a chain.

```i
double = n -> n * 2
add    = a, b -> a + b
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

Lambdas are first-class. Anywhere an expression fits, `args -> body` fits:

```i
nums.map x -> x * 2
nums.filter x -> x > 0
```

If you want to write a function's type, the form is `args -> result`:

```i
double : Int -> Int
add    : Int, Int -> Int
```

Next: bundling values into records.

---

## 4. Records

A record is a `type` block with named fields. Inside the block, `:` introduces
a field and `=` introduces a method or constant. Methods receive an implicit
`self`.

```i
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5
```

To construct, apply the type to keyword arguments. Construction parens are
**required** — the `=` of a kwarg would otherwise collide with the `=` of the
binding:

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

Same surface, two related operations: type-applied-to-kwargs constructs;
instance-applied-to-kwargs updates. There is no inheritance — types compose
through traits, not subclassing.

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

Two new things. The `Shape` type has two cases, `Circle` and `Rect`, each with
its own fields. And `shape match` is the pattern-match form: write the value,
then `match`, then an indented block of `pattern -> body` arms.

The compiler checks every `match` for exhaustiveness. Forget to handle
`Rect`, and the program won't build — the error tells you which case you
missed. There is no fallthrough and no default needed when you've covered the
cases by name.

For all the pattern kinds (literals, lists, nested), see [patterns.md](patterns.md).

---

## 6. Maybe and Result

`i` has no null and no exceptions. Absence of a value is `Maybe a`; failure is
`Result a e`. Both come from the standard library; both work with `match`.

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

When you have several fallible calls in a row, the `?` operator collapses the
boilerplate. `expr?` means: if `expr` is `Error e`, return `Error e` from
the enclosing function; otherwise unwrap the `Ok` value. It only type-checks
inside a function whose return type is a `Result _ e` with the same error
type.

```i
parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys]  -> Ok Point(x = parseFloat xs?, y = parseFloat ys?)
        _         -> Error WrongShape
```

The compiler still checks every error path. `?` is sugar, not an escape hatch.

For the full safety story, see [types.md](types.md) and the stdlib
[Maybe](stdlib.md) and [Result](stdlib.md) entries.

---

## 7. Effects

A function whose type does not mention `!` does no IO, no mutation, and
nothing else observable. That's enforced — not a guideline.

When you call something effectful, you mark the call site with `!`:

```i
main =
    print! "what's your name?"
    name = readLine!
    print! "hi, " ++ name
```

See `examples/02-greet.i` for the runnable version.

You almost never write `!` in a type signature. The compiler infers the
effect row and propagates it:

```i
print    : String ! IO -> Unit       # in stdlib
readLine : ! IO -> String            # in stdlib
greet    : String ! IO -> Unit       # inferred from body
add      : Int, Int -> Int           # pure
```

The visible thing is `!` at the call site. If you accidentally call `print!`
from a function you thought was pure, that function's inferred type now
carries `! IO`, and you'll see it as soon as you try to use that function
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

`nums.map x -> x * 2` is two things glued: `nums.map` is a method call (with
implicit `self = nums`), and `x -> x * 2` is the lambda passed in.

A few more, for flavor:

```i
positives = nums.filter x -> x > 0
total     = nums.fold 0, (acc, x -> acc + x)
```

`fold` takes the initial accumulator and a two-arg function combining
accumulator with the next element. The parens around the lambda are the
same nesting trick from section 3 — they keep the lambda's commas from
being read as more arguments to `fold`. There is no `for` loop in `i`; when
you want to walk a list, you reach for `map`, `filter`, or `fold`.

Note `head : List a -> Maybe a`. The standard library never crashes on an
empty list; it gives back a `Maybe`.

For the full list of stdlib operations, see [stdlib.md](stdlib.md).

---

## 9. Modules

One file is one module. The first non-blank line declares the module name and
what it exposes; everything else in the file is private.

```i
module Geometry
    expose Point, distance

type Point
    x : Float
    y : Float

distance = a, b ->
    ((a.x - b.x)^2 + (a.y - b.y)^2)^0.5

# private helper, not exposed
square = x -> x * x
```

To use names from another module, write `use`:

```i
use Std.IO                              # whole module — Std.IO.print, etc.
use Std.IO (print, readLine)            # cherry-pick names
use Geometry as Geo                     # rename for local use
```

The `Main` module with its `main` value is the program entry point — that's
why every example so far has started with `module Main` and `expose main`.

For the full module rules, see [modules.md](modules.md).

---

## 10. Where to go next

You have now seen every concept in the language. Pick the depth you want:

- [Syntax reference](syntax.md) — every form, every operator, look-up style
- [Type system](types.md) — records, sums, generics, traits, totality
- [Effect system](effects.md) — what `!` actually does
- [Pattern matching](patterns.md) — every kind of pattern, with examples
- [Modules](modules.md) — file layout, `expose`, `use`, project structure
- [Standard library](stdlib.md) — every type and function in v1
- [Building and running](building.md) — installing the toolchain, the CLI
- [Limitations](limitations.md) — what v1 explicitly doesn't do

If you only have time to read one more, read [syntax.md](syntax.md) — it's
the densest map of the surface. Everything else is depth on a particular
axis.
