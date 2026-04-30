# Standard library reference

Lookup reference for v1's standard library. Every type and function is
listed. Each entry gives the signature, a one-line description, and a small
example only when the shape isn't obvious from the signature.

For the language surface, see [syntax.md](syntax.md). For the type and
effect rules these signatures live inside, see [types.md](types.md) and
[effects.md](effects.md). For a guided walk through the most-used pieces,
see [tour.md](tour.md).

The v1 stdlib is deliberately small. The numeric tower is `Int` (i64) and
`Float` (f64) only. There's no `Std.Parse` module; parsing functions live
on the type they parse into (`Std.Int.parse`, `Std.Float.parse`).

---

## Prelude traits

These traits are auto-imported into every module. You don't write
`use Std.Eq` to use `==`. Operators desugar to trait methods (see
[syntax.md § 11](syntax.md)); to make a user-defined type usable with
the operator, write the matching `impl`.

### `Eq a`

```
eq : a, a -> Bool
ne : a, a -> Bool
```

Equality. Operators `==` and `/=` desugar to `Eq.eq` and `Eq.ne`.

### `Ord a`

```
lt : a, a -> Bool
le : a, a -> Bool
gt : a, a -> Bool
ge : a, a -> Bool
```

Ordering. Operators `<`, `<=`, `>`, `>=` desugar to these. An `Ord` impl
is conventionally provided alongside `Eq`.

### `Add a`, `Sub a`, `Mul a`, `Div a`

```
add : a, a -> a
sub : a, a -> a
mul : a, a -> a
div : a, a -> a
```

Arithmetic. Operators `+`, `-`, `*`, `/` desugar to these. `Int` and
`Float` ship implementations. Mixing the two is a type error: no implicit
conversion (see [types.md § 1](types.md)).

### `Neg a`

```
neg : a -> a
```

Unary minus. The expression `-x` desugars to `Neg.neg x`.

### `Pow a`

```
pow : a, a -> a
```

Exponentiation. The `^` operator desugars to `Pow.pow`. `Float` ships
an impl backed by `Std.Float.pow`; `Int` does not (negative integer
exponents would not return an `Int`). Write `impl Pow T` to opt in
a user-defined numeric type.

### `Show a`

```
show : a -> String
```

Convert a value to its display string. This is what `print!` and the
string-concatenation idiom rely on. Every primitive type and most stdlib
types have a derived `Show` impl.

```i
print! "n = " ++ show n
```

---

## `Std.Bool`

The two-valued boolean type and its functions. `and`, `or`, `not`, `xor`
are ordinary functions, not operators. Call them paren-free like any
other function.

### `Bool`

```
type Bool
    True
    False
```

The boolean type. `True` and `False` are the two variants.

### `and : Bool, Bool -> Bool`

Logical conjunction. Strict in both arguments.

### `or : Bool, Bool -> Bool`

Logical disjunction. Strict in both arguments.

### `not : Bool -> Bool`

Logical negation.

### `xor : Bool, Bool -> Bool`

Exclusive-or.

---

## `Std.Int`

The 64-bit signed integer type. Arithmetic is via the prelude traits
(`Add`, `Sub`, `Mul`, `Div`, `Neg`); the entries below cover what's
specific to `Int`.

### `Int`

```
type Int        # = i64
```

Signed 64-bit integer.

### `compare : Int, Int -> Ordering`

Three-way compare. Returns `LT`, `EQ`, or `GT` (the `Ordering` type
re-exported here).

### `parse : String -> Result Int ParseError`

Parse an integer from a string. The error variant is the `Std.Int`
module's own `ParseError`.

```i
use Std.Int as I
n = I.parse "42"        # Ok 42
```

### `toFloat : Int -> Float`

Widen to `Float`. There's no implicit `Int`-to-`Float` coercion; this is
the explicit conversion.

### `toString : Int -> String`

Render as a decimal string. Equivalent to `show` from the `Show` impl.

---

## `Std.Float`

The 64-bit IEEE-754 floating-point type. Same arithmetic shape as
`Int`; this section lists the math functions and conversions.

### `Float`

```
type Float      # = f64
```

IEEE-754 double-precision float.

### `compare : Float, Float -> Ordering`

Three-way compare. Note: NaN comparison follows IEEE-754, not a total
ordering.

### `parse : String -> Result Float ParseError`

Parse a float from a string.

```i
use Std.Float as F
x = F.parse "3.14"       # Ok 3.14
```

### `toInt : Float -> Int`

Truncate toward zero.

### `toString : Float -> String`

Render as a decimal string. Same as `show`.

### `sqrt : Float -> Float`

Principal square root.

### `pow : Float, Float -> Float`

`pow x, y` is `x ^ y`. The `^` operator desugars to this.

### `sin : Float -> Float`

Sine, radians.

### `cos : Float -> Float`

Cosine, radians.

### `tan : Float -> Float`

Tangent, radians.

### `exp : Float -> Float`

Natural exponential.

### `ln : Float -> Float`

Natural logarithm.

---

## `Std.Char`

Operations on a single Unicode scalar value.

### `Char`

```
type Char
```

A Unicode scalar value.

### `toUpper : Char -> Char`

Upper-case a character. Returns the input unchanged if no upper-case
form exists.

### `toLower : Char -> Char`

Lower-case a character. Returns the input unchanged if no lower-case
form exists.

### `isDigit : Char -> Bool`

True for ASCII digits `0`–`9`.

### `isAlpha : Char -> Bool`

True for ASCII letters `A`–`Z` and `a`–`z`.

---

## `Std.String`

The opaque string type. Strings are immutable sequences of `Char`. The
`++` operator desugars to `Concat.concat` (see [syntax.md § 11](syntax.md)).

### `String`

```
type String
```

An immutable Unicode string.

### `length : String -> Int`

Number of `Char`s in the string.

### `++ : String, String -> String`

Concatenation. The `++` operator on strings desugars to
`Concat.concat`, and the `Concat String` impl returns a new string.

### `split : String, String -> List String`

`split s sep` breaks `s` into pieces at every occurrence of `sep`. Empty
pieces are preserved.

```i
"a,b,,c".split ","      # ["a", "b", "", "c"]
```

### `toChars : String -> List Char`

Decompose into a list of `Char`s.

### `fromChars : List Char -> String`

Inverse of `toChars`.

### `contains : String, String -> Bool`

`s.contains needle` returns `True` if `needle` appears anywhere in `s`.

### `trim : String -> String`

Strip leading and trailing whitespace.

---

## `Std.List`

The singly-linked list type. The rule across the stdlib's collection
operations: never crash, always return `Maybe` or `Result` for partial
functions.

### `List a`

```
type List a
    Empty
    Cons
        head : a
        tail : List a
```

Singly-linked list. The `[a, b, c]` literal desugars to nested `Cons`
ending in `Empty`.

### `length : List a -> Int`

Number of elements. Linear in the length.

### `map : List a, (a -> b) -> List b`

Apply a function to each element, producing a new list. Method-call
form is the idiom:

```i
nums.map x -> x * 2
```

### `filter : List a, (a -> Bool) -> List a`

Keep the elements for which the predicate returns `True`.

### `fold : List a, b, (b, a -> b) -> b`

Left fold. `xs.fold init f` reduces left-to-right with accumulator
`init`. The two-arg lambda must be parenthesized (see
[syntax.md § 5](syntax.md)).

```i
nums.fold 0, (acc, x -> acc + x)
```

### `reverse : List a -> List a`

Reverse the list. Linear time.

### `head : List a -> Maybe a`

First element, or `None` if the list is empty. Total: never crashes.

### `tail : List a -> Maybe (List a)`

Everything after the head, or `None` if the list is empty.

### `take : List a, Int -> List a`

`xs.take n` is the first `n` elements. Returns the whole list if `n` is
larger than the length; returns `Empty` if `n <= 0`.

### `drop : List a, Int -> List a`

`xs.drop n` is everything after the first `n` elements. Returns
`Empty` if `n` exceeds the length.

### `zip : List a, List b -> List (Pair a b)`

Pair elements positionally. The result has the length of the shorter
input. `Pair` is `Std.Pair` (see below); used here because v1 has no
tuples.

---

## `Std.Pair`

A two-element grouping with named fields, used where a tuple would be
natural in other languages.

### `Pair a, b`

```
type Pair a, b
    first  : a
    second : b
```

### `make : a, b -> Pair a b`

Construct a pair: `make x, y` is equivalent to `Pair(first = x, second = y)`.

### `swap : Pair a b -> Pair b a`

Swap the two fields.

---

## `Std.Maybe`

Absence-or-value. The replacement for null. See
[types.md § 9](types.md) for the design rationale.

### `Maybe a`

```
type Maybe a
    None
    Some : a
```

Either nothing (`None`) or a value (`Some x`).

### `withDefault : Maybe a, a -> a`

Unwrap with a fallback. `m.withDefault d` is `x` for `Some x` and `d`
for `None`.

### `map : Maybe a, (a -> b) -> Maybe b`

Apply a function inside the `Some`; pass `None` through unchanged.

### `andThen : Maybe a, (a -> Maybe b) -> Maybe b`

Monadic bind (a.k.a. `flatMap`). Chain a `Maybe`-returning step onto a
`Maybe` value, threading `None` through.

```i
lookup userId .andThen u -> u.email
```

---

## `Std.Result`

Success-or-failure. The replacement for exceptions. See
[types.md § 9](types.md) for the design rationale.

`?` is language sugar, not a stdlib function — it's documented in
[syntax.md § 9](syntax.md). For a worked example, see
`examples/07-result.i`.

### `Result a, e`

```
type Result a, e
    Ok    : a
    Error : e
```

Either a success value (`Ok x`) or a failure value (`Error e`). The
error type `e` is fully user-chosen — usually a small sum type.

### `withDefault : Result a e, a -> a`

Unwrap with a fallback. Discards the error.

### `map : Result a e, (a -> b) -> Result b e`

Apply a function inside `Ok`; pass `Error` through unchanged.

### `mapError : Result a e, (e -> f) -> Result a f`

Transform the error. The usual tool for adapting one error type to
another at a module boundary.

### `andThen : Result a e, (a -> Result b e) -> Result b e`

Monadic bind. Chain a `Result`-returning step; threads `Error` through.
The `?` operator is sugar over an `andThen`-style early exit.

---

## `Std.IO`

The IO effect's stdlib surface. Every operation here carries `! IO`.
Calling any of them gives the calling function `! IO` in its inferred
type. See [effects.md § 4](effects.md).

### `print : String ! IO -> Unit`

Write a string to stdout, no trailing newline.

```i
print! "hello"
```

### `println : String ! IO -> Unit`

Write a string to stdout followed by a newline.

### `readLine : ! IO -> String`

Read one line from stdin. Strips the trailing newline. Zero-argument
procedure — call it `readLine!`.

### `readFile : String ! IO -> String`

Read the entire file at the given path as a string. File-not-found and
similar errors surface through the IO error channel, provisionally typed
`IoError`. The variants aren't pinned in v1; see
[limitations.md](limitations.md).

### `writeFile : String, String ! IO -> Unit`

`writeFile path contents` — write the string to the file, replacing any
existing contents.

---

## `Std.Ref`

Mutable cells. The only `! State` surface in v1. Use sparingly; most
code does not need it. See [effects.md § 5](effects.md) for the
"small effectful core, large pure surround" pattern.

### `Ref a`

```
type Ref a
```

A mutable cell holding an `a`.

### `make : a ! State -> Ref a`

Create a new cell containing the given value.

```i
counter = Std.Ref.make! 0
```

### `get : Ref a ! State -> a`

Read the current value.

### `set : Ref a, a ! State -> Unit`

Replace the cell's contents.

```i
Std.Ref.set! counter, 1
```

---

## See also

- [syntax.md](syntax.md) — operator desugaring and effect-marker forms
- [types.md](types.md) — generics, traits, and the no-null/no-exception design
- [effects.md](effects.md) — `! IO` and `! State`, the only effect labels in v1
- [tour.md](tour.md) — guided walk-through with examples that use the stdlib
- [limitations.md](limitations.md) — what the stdlib does not yet cover in v1
