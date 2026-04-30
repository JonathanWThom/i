# The effect system

A deep dive on `!`. For surface forms, see [syntax.md § 8](syntax.md); for a
guided introduction, see [tour.md § 7](tour.md). This manual is the *why*.

Most code in `i` is pure, and most readers will spend most of their time
not thinking about effects at all. The system is built to stay out of your
way. You write ordinary functions, the inferencer figures out which ones
touch the outside world, and the `!` you put at call sites is the only
thing you ever have to type. The rest of this doc explains how that turns
out to be enough.

---

## 1. What "pure" means here

A function is **pure** when its inferred type contains no `!` row. That's
a strong claim, not a stylistic one. A pure function:

- does no IO. It doesn't print, read input, touch the filesystem, or hit
  the network.
- mutates no state. No in-place updates, no rebinding.
- has no observable side effects. No logging, no clocks, no randomness.

Given the same inputs, a pure function produces the same output every time,
with no externally visible trace. The compiler enforces this by tracking
*every* effectful call through the call graph (§ 6) and refusing to assign
a pure type to any function that participates in one.

```i
double : Int -> Int          # pure — no `!` in the type
double = n -> n * 2

quadruple : Int -> Int       # pure — calls double, which is pure
quadruple = n -> double (double n)
```

There's no annotation that opts a function into purity. Purity is the
*default*. The absence of `!` is the absence of effects, not a promise to
be verified later. See `examples/05-effects.i` for a side-by-side of pure
and effectful definitions.

---

## 2. The `!` marker

`!` appears at exactly one place in user code: a call site that performs
an effect. It's a postfix on the function name, written before the arguments.

```i
print! "hello"
name = readLine!
contents = readFile! "input.txt"
```

For a zero-argument procedure like `readLine`, the `!` is also how you
invoke it. Without it, `readLine` is a reference to the function value,
not a call. For a procedure with arguments, `!` sits between the function
name and the arguments: `print! "hello"`.

The `!` is *not* part of the function's name. `print` is the name; `print!`
is "calling `print` for its effect." The compiler takes the marker as
evidence that you knew this call was effectful, and propagates the effect
into the caller's type.

You almost never write `!` anywhere else. Type signatures pick effects up
from the body. You only write `! Eff` in a type when you're documenting
an interface by hand (§ 3).

---

## 3. Effect rows

When effects do appear in a type, they show up as a row between argument
types and the result type:

```i
print    : String ! IO -> Unit
readLine : ! IO -> String
writeFile : String, String ! IO -> Unit
```

Read it left to right: "`String`, with effect `IO`, to `Unit`." Or for the
zero-argument form: "with effect `IO`, to `String`." The row is *part of
the function's type*: two functions that differ only in their effect row
are different types.

A function with multiple effects separates them with commas in the row:
`! IO, State`. v1 only has two effect labels, so this is mostly
theoretical, but the slot is there.

The row gives you one place to look to understand a function's boundary
with the outside world. `add : Int, Int -> Int` has no `!`, so it has no
boundary. `print : String ! IO -> Unit` has `! IO`, so it does.

---

## 4. What effects exist in v1

Two labels, total:

- **`IO`** — anything that crosses the program/world boundary. The
  `Std.IO` operations all carry `! IO`: `print`, `println`, `readLine`,
  `readFile`, `writeFile`. The `readFile` / `writeFile` error surface is
  provisionally `IoError`; its variants aren't pinned yet (see
  [stdlib.md § `Std.IO`](stdlib.md) and [limitations.md](limitations.md)).
  Future filesystem or network primitives will carry `! IO` too.
- **`State`** — mutable cell operations, exposed through `Std.Ref`.
  `Ref.make`, `Ref.get`, and `Ref.set` all carry `! State`. A function that
  reads or writes a `Ref` picks up `! State` in its inferred row.

That's the entire effect alphabet for v1. There are no user-defined effects
in v1; adding effect labels (an exception effect, a logging effect, an
algebraic-effect handler system) is deferred to a later spec. See
[limitations.md](limitations.md).

`IO` and `State` are tracked independently. A function that prints to the
console and reads a `Ref` has type `... ! IO, State -> ...`. Order within
a row doesn't matter.

---

## 5. Why mutation isn't built-in

`i` has no `mut`, no rebinding, no in-place updates. Every "modification"
makes a new value:

```i
p1 = Point(x = 0, y = 0)
p2 = p1(x = 5)              # new Point, p1 unchanged
```

This fits the rest of the language: values are facts, and new facts are
new values. It's also what makes the purity guarantee in § 1 worth anything.
If two function calls could quietly mutate a shared record, "no observable
effect" would mean nothing.

For the rare case that actually needs mutable state — an iteration counter
threaded through a recursion, a memoization table, a buffer being built up
— the stdlib exposes `Std.Ref`. `Ref a` is a mutable cell:

```i
Std.Ref.make : a ! State -> Ref a
Std.Ref.get  : Ref a ! State -> a
Std.Ref.set  : Ref a, a ! State -> Unit
```

`Ref` operations all carry `! State`. The cost of using one is that the
function becomes effectful: its type now mentions `! State`, and any caller
that wants to stay pure has to encapsulate the use locally. Mutation is
available; it's just *visible* in the type, the same way IO is. You can't
do mutation invisibly.

The pattern this supports is "small effectful core, large pure surround."
The effectful part of a program lives in `main` and a few helpers, while
most of the logic stays pure and trivially testable.

---

## 6. What you don't have to write

Effect rows are inferred. You don't annotate your own functions with
`! IO` when they call effectful things — the compiler propagates the row
upward through the call graph automatically. The rule is the natural one:
a function's effect row is the union of the rows of every effectful call
in its body.

```i
shout : Int ! IO -> Unit       # inferred — calls print!
shout = n ->
    print! "the number is " ++ show n

main : ! IO -> Unit            # inferred — calls print! and shout
main =
    print! "starting"
    shout 21
    print! "done"
```

The `!` at each call site is the only thing you wrote. The signatures
above are what inference produces; you'd normally not write them out.
Module-exposed names are the exception: pinning the inferred row in the
signature documents the boundary and prevents accidental widening, the
same way you might pin a value's type at a module boundary (see
[types.md § 10](types.md)).

You also don't declare an effect alphabet, register handlers, or import an
effect library. The two labels live in the language. The stdlib carries
them on the operations that produce them. Your code picks them up by
calling those operations.

---

## 7. What this catches

The everyday payoff is simple: you can't accidentally do IO from a
function you thought was pure. Suppose you drop a `print!` into a function
for debugging:

```i
double = n ->
    print! "doubling " ++ show n
    n * 2
```

`double`'s inferred type is now `Int ! IO -> Int`, not `Int -> Int`. Every
caller of `double` picks up `! IO`. Any function that *was* pure but now
transitively calls `double` has `! IO` in its inferred type too. The
moment one of those gets used somewhere a pure value is required — a
trait method that returns a pure type, an exposed signature you wrote by
hand, a context that doesn't permit IO — the compiler reports the mismatch
and points at the offending call.

You don't have to remember to clean up the debug print. The type system
won't let you forget. Leaving `print!` in production code isn't a
discipline question, it's a compile error in every place that expected a
pure function.

The same mechanism catches less obvious cases. A helper that quietly
mutates a `Ref` carries `! State` and can't be called from a function
whose signature pins it as pure. A refactor that pushes IO into a deeper
utility shows up as `! IO` everywhere it's reached. The effect row makes
the call graph's relationship with the outside world readable.

---

## See also

- [syntax.md § 8](syntax.md) — surface forms for `!` and effect rows.
- [tour.md § 7](tour.md) — the introductory walkthrough.
- [types.md](types.md) — the rest of the type system; effect rows live
  inside function types and follow the same inference rules.
- [stdlib.md](stdlib.md) — `Std.IO` and `Std.Ref`, the only effect-bearing
  modules in v1.
- [limitations.md](limitations.md) — user-defined effects, handlers, and
  other effect-system extensions are deferred.
