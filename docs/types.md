# The type system

A deep dive on what the type checker does, what guarantees it provides, and
how the pieces fit together. For surface forms, see [syntax.md](syntax.md);
for a guided introduction, see [tour.md](tour.md). This manual is the *why*.

---

## 1. What "strongly typed" means here

Every value in `i` has a static type known at compile time. There are no
runtime type errors, no implicit conversions, and no null. A program that
type-checks is, by construction, free of the bug families those features
admit:

- Types that share a representation are not interchangeable. An `Int` is
  not a `Float`; a `UserId` is not an `OrderId` (see § 6).
- A value cannot become "nothing" at runtime. Absence is the separate type
  `Maybe a` (see § 9), and the compiler forces you to handle `None` before
  using the inner value.
- A function cannot silently fail or throw. Failure is `Result a e`, and
  again both arms must be handled.

There are no automatic coercions — not even `Int` to `Float`. To convert,
you call `Std.Int.toFloat` explicitly. The type checker prefers a hard
error to a guessed conversion, on the principle that conversions you
didn't write are conversions you didn't think about.

The whole system is nominal: two types with identical structure are still
distinct unless they are the *same* declared type. There is no structural
subtyping, no row polymorphism, and no inheritance.

---

## 2. Type inference scope

You almost never write a type. Inference handles the interior of a function;
you write a type only at interfaces — module-exposed names, trait
declarations — or when inference can't decide on its own.

**What inference produces.** The algorithm is Hindley-Milner-style: a
single bottom-up pass with let-polymorphism at binding sites. Top-level
bindings are *generalized* — `id = x -> x` at the top level gets the
polymorphic type `a -> a` and can be used at any type. Local bindings
inside a function body are *not* generalized; they are monomorphic,
fixed at their first use. Mutual recursion is allowed within a single
let-group, and a module's top-level bindings form one such group.

Trait usage propagates as constraints into the inferred signature. The
notation `Constraint => ...` reads "for any type satisfying this
constraint":

```i
eq : Eq a => a, a -> Bool       # for any a that has an Eq impl
```

The rule of thumb:

- **Local bindings:** inferred. `n = 42` does not need an annotation.
- **Lambda parameters and return:** inferred from the call context.
  `nums.map x -> x * 2` infers `x : Int` from `nums`.
- **Top-level functions:** inferred from the body; a signature is
  encouraged for documented interfaces.
- **Trait method signatures:** required, because they *are* the interface.
- **Record fields and variant payloads:** required, because they have no
  body to infer from.

When inference cannot find a type — usually because a polymorphic function
is used at no concrete type — the error names the variable and the
location. The fix is an annotation at that site, not a global hint.

```i
# fails: `[]` produces a `List a` for any `a`, and nothing in this
# binding pins the element type.
xs = []

# fixed by annotating the binding:
xs : List Int = []
```

```i
double : Int -> Int               # explicit at the interface
double = n -> n * 2
```

Module-exposed names get inferred types unchanged; writing the signature
is still recommended, because it pins the interface against accidental
widening.

---

## 3. Records

A record is a `type` block whose members are fields and methods. Fields are
declared with `:`, methods (and constants) with `=`. Inside the block the
`Type.` prefix is implicit on every member name, and any function bound with
`=` receives an implicit `self` parameter referring to the instance.

```i
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5
```

`distance` here binds `Point.distance`. `self` is not in the parameter list;
the binding's location supplies it.

Construction and update share one surface. A type applied to keyword
arguments constructs; an instance applied to keyword arguments produces a
copy with overrides. Both express the same idea — "an instance with
these field values" — and differ only in where the unsupplied fields
come from: you must give all of them at construction; the prior instance
supplies them on update. Construction parens are required (not just
grouping) because the inner `=` of a kwarg would otherwise collide with
the outer binding `=`.

```i
p1 = Point(x = 0, y = 0)        # construct
p2 = p1(x = 5)                  # copy with x = 5
```

Every field of a record must be supplied at construction. There are no
default values; if you want one, write a separate constructor function. A
record is internally a sum type with one implicit case — see § 4.

---

## 4. Sum types

Capitalized members of a `type` block are variants. A variant may stand
alone, carry its own field block, or use the single-payload shorthand
`Variant : T` for an anonymous payload of type `T`.

```i
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

type Maybe a
    None
    Some : a            # single-payload shorthand
```

The shorthand and block forms have different access patterns. `Some : a`
exposes the payload as the variant itself — `Some x` in a pattern binds
`x : a` directly. The block form `Some\n    value : a` exposes a named
field — `Some s` in a pattern gives `s.value : a`. Use the shorthand when
the payload has no useful name; use the block form when the payload is a
record-like cluster of named fields.

**Exhaustiveness checking.** Every `match` on a sum type must cover every
constructor. A `match` that omits a case is a compile-time error, not a
warning, and the error names the missing constructor. There is no
fallthrough; arms do not "fall through" to subsequent ones, and there is
no implicit default. When you genuinely want a default, use a wildcard
arm:

```i
shape match
    Circle r    -> 3.14159 * r^2
    Rect w, h   -> w * h
    # forgetting either arm is a compile error
```

Constructor patterns also support positional binding in field-declaration
order — `Rect w, h` binds `w = width` and `h = height`. Both positional
and named-field forms are accepted; pick whichever reads better.

This is the language's most-relied-on safety property. You can refactor a
sum type — add a variant, rename one — and the compiler will list every
`match` that needs updating. The cost of one extra arm at every match site
buys mechanical safety on every change to the data.

---

## 5. Generics

Lowercase identifiers in a type position are type variables. Uppercase
identifiers are concrete types. There is no introducing keyword for a type
variable; the *first* lowercase use in a signature or type declaration
binds it, and subsequent uses refer to the same variable.

```i
type List a
    Empty
    Cons
        head : a
        tail : List a
```

`a` is bound by its first appearance and used twice; the type checker
understands this without ceremony. Multiple type parameters are
comma-separated after the type name:

```i
type Result a, e
    Ok    : a
    Error : e
```

Generic functions follow the same rule. The identity function works for
any type; written explicitly:

```i
identity : a -> a
identity = x -> x
```

The `a` in the signature is a type variable — the function works for every
type. At each call site, `a` is unified with the argument's type. There is
no "value of type `a`" in the body — generics are parametric, meaning the
body cannot inspect or branch on the type. If you want behavior that
varies by type, use a trait (see § 7).

A function that uses *both* `a` and `b` constrains them independently:
`zip : List a, List b -> List (Pair a b)` (from [`Std.List`](stdlib.md);
`Pair` is the record type `Std.Pair` since v1 has no tuples) takes
two lists whose element types are decided independently. Same letter means
same type across the signature; different letters mean independently
inferred.

For trait-constrained generics — `a` such that `Eq a`, for example — see
§ 7.

---

## 6. Newtypes

A `type` with a single case wrapping a single value is the idiomatic
newtype. The shortest form is the single-line equation:

```i
type UserId  = Int
type OrderId = Int
```

`UserId` and `OrderId` are *distinct types*, even though both wrap `Int`.
A function expecting a `UserId` will reject an `OrderId`, an `Int`, and
anything else. This is the type system's mechanism for tagging meaning
onto a representation:

```i
lookupUser : UserId -> Maybe User
lookupUser uid = ...

# accidentally pass an OrderId — compile error.
lookupUser someOrderId
```

`i`'s implementation is expected to compile newtypes to zero-cost
wrappers — no boxing, no allocation for a newtype that wraps a primitive
— though the spec does not formally pin the runtime representation.

The block form is the long-hand equivalent and is useful when you want to
add methods to the wrapper:

```i
type UserId
    value : Int
    toString = -> Std.Int.toString self.value
```

This is the same nominal mechanism — `UserId` is still distinct from
`Int` — with room for a `toString` method or other operations.

---

## 7. Traits

Traits are how `i` does ad-hoc polymorphism: behavior that depends on the
type of a value. A trait declaration names methods; an `impl` provides
those methods for a specific type. Generic functions that need
type-dependent behavior write a constraint such as `Eq a => ...` (see
§ 5 for the parametric base case).

```i
trait Eq a
    eq : a, a -> Bool
    ne : a, a -> Bool
```

An implementation is `impl Trait Type` followed by the method bodies:

```i
impl Eq Point
    eq = a, b -> a.x == b.x and a.y == b.y
    ne = a, b -> not (Eq.eq a, b)
```

**Operator desugaring.** Most operators are sugar for trait method calls.
`a + b` is `Add.add a, b`; `a == b` is `Eq.eq a, b`; `a < b` is
`Ord.lt a, b`. The full table lives in [syntax.md § 11](syntax.md). To
make a type usable with `+`, write `impl Add Type`; to make it
comparable, `impl Eq Type`. The prelude pre-imports `Eq`, `Ord`, `Add`,
`Sub`, `Mul`, `Div`, `Neg`, `Pow`, and `Show`, so operators on primitives
just work. `Show.show : a -> String` is the conversion that `print!` and
`++` rely on. See [stdlib.md § Prelude traits](stdlib.md) for each
trait's signature.

**Coherence.** `i` follows Haskell-style global coherence: at most one
`impl Trait Type` exists in the entire program for each `(trait, type)`
pair. There is no orphan-instance escape hatch. Every call site
dispatches to the same implementation regardless of import path — at the
cost of being unable to add an impl for a trait *and* a type both
defined elsewhere. For a small ecosystem this is the right trade for
"fits in your head."

A type can implement any number of traits. There is no inheritance and
no trait extension; the convention is that related impls (`Ord` and
`Eq`) are provided together.

---

## 8. Totality

Every function in `i` is total: it terminates on every input and produces
a value of its declared return type. Three rules together enforce this:

1. **Exhaustive pattern matching.** A `match` that misses a case fails to
   compile (see § 4). A function whose body is a `match` therefore handles
   every possible input.
2. **No partial standard library functions.** `List.head` returns
   `Maybe a`, not `a`; the empty case has nowhere to crash.
3. **Termination checking on recursion.** A recursive function whose
   recursive calls are made on *structurally smaller* arguments — a
   sub-list of the input list, the inner of a `Some`, etc. — is accepted.
   General recursion (e.g., `f n -> f (n + 1)` or recursion on a value the
   compiler can't see shrinks) requires an explicit `corecursive`
   annotation.

   **Note:** v1 does not yet specify the `corecursive` annotation in
   detail. Structural recursion is what v1 accepts; the form and exact
   semantics of the explicit escape hatch are TBD. See
   [limitations.md](limitations.md).

The payoff is large. A total function's type signature is its complete
specification: given an input of the argument type, the function produces
an output of the result type, full stop — no third "or it crashes"
outcome.

The cost is also real. Algorithms that genuinely need general recursion
(some graph traversals, some search procedures) will need a
`corecursive` escape hatch (TBD); until then, expressing them requires
restructuring as structural recursion. The form, exact name, and
semantics of the escape hatch are a known specification gap, not a
roadmap commitment.

---

## 9. No null, no exceptions

The language has no null reference and no exception mechanism. Two types
in the standard library cover the cases those features address:

- `Maybe a` for *absence* — the value might or might not be there. Used
  when a lookup might find nothing, a parse might be empty, an optional
  field is unset.
- `Result a e` for *failure* — the operation might succeed with an `a` or
  fail with an `e`. Used when a parse might fail with a reason, a network
  call might return an error, a precondition might be violated.

Both are ordinary values, not control-flow constructs, because totality
(§ 8) requires a function's type to fully describe its outcomes. An
unchecked exception is a hidden return path no type can express; a null
reference is a hidden inhabitant no type names. Eliminating both makes
the type checker's claims about what a function can return *complete* —
the signature is the whole story.

```i
type Maybe a
    None
    Some : a

type Result a, e
    Ok    : a
    Error : e
```

The compiler treats both as ordinary sum types. There is no special
"unwrap" operation; you `match` on them like any other sum, and the
exhaustiveness checker forces you to handle both arms before you can use
the inner value. This is the safety mechanism that null and unchecked
exceptions in other languages do not have.

For a chain of fallible calls, the `?` early-exit operator collapses the
plumbing. `expr?` returns `Error e` from the enclosing function if `expr`
is `Error e`; otherwise it evaluates to the unwrapped `Ok` value. It only
type-checks inside a function whose return type is `Result _ e` with the
matching error type.

```i
parsePoint = s ->
    parts = s.split ","
    parts match
        [xs, ys]  -> Ok Point(x = parseFloat xs?, y = parseFloat ys?)
        _         -> Error WrongShape
```

`?` is sugar — the compiler still checks every error path. See
[tour.md § 6](tour.md) for the worked example and [syntax.md § 9](syntax.md)
for the surface form.

---

## 10. Type signatures: when to write them

Inference covers most cases. The defensible places to write a signature
are the boundaries — where the type should be the contract rather than
a side effect of the body.

**Write a signature for:**

- Module-exposed values. The signature pins the interface; later body
  changes that would widen or narrow the type become errors at the
  signature site rather than silent shifts in the exposed contract.
- Trait method declarations. These *are* signatures.
- Record field and variant payload declarations. No body to infer from.
- Functions whose inferred type is more general than intended. Pinning
  `add : Int, Int -> Int` prevents accidental generalization to
  `Add a => a, a -> a`.
- Disambiguation, when inference reports ambiguity.

**Don't write a signature for:**

- Trivially-inferable local bindings. `n = 42` does not benefit from
  `n : Int = 42`.
- Lambda parameters in a context that determines them.
- Short internal helpers whose body is the documentation.

Inference recovers what you would otherwise write down; *writing* a
type pins a contract you want to preserve. Use both: write types where
contracts matter, let inference handle the rest.

---

## See also

- [syntax.md](syntax.md) — every form, every operator, look-up style.
- [tour.md](tour.md) — guided introduction with worked examples.
- [effects.md](effects.md) — the `!` system and how effect rows interact
  with the type system.
- [patterns.md](patterns.md) — every pattern kind and how exhaustiveness
  is checked.
- [limitations.md](limitations.md) — what v1 does not yet specify,
  including the `corecursive` annotation.
