# Syntax reference

Lookup reference. Every form has an entry. Examples are minimal.

For tutorial introductions, see [tour.md](tour.md). For semantics depth, see
[types.md](types.md), [effects.md](effects.md), [patterns.md](patterns.md),
[modules.md](modules.md).

---

## 1. Lexical

### Comments

Line comments start with `#` and run to the end of the line. No block comments.

```i
# this is a comment
x = 1   # trailing comments are fine
```

### Identifiers

Lowercase identifiers name values and type variables. Uppercase identifiers
name types, variants, traits, and modules. Case is significant.

```i
point : Point       # value : Type
list  : List a      # `a` is a type variable
```

### Whitespace and indentation

Indentation marks blocks. Increasing indentation opens a block; returning to
the outer level closes it. Tabs and spaces must not be mixed within a file.

```i
type Point
    x : Float
    y : Float
```

### Newlines

A newline terminates an expression. The exception is when the line ends with
an operator or an open paren — then the expression continues on the next line.

```i
total = a +
    b + c
```

### Integer literal

Decimal digits, of type `Int` (i64).

```i
n = 42
```

### Float literal

Decimal digits with a `.`, of type `Float` (f64).

```i
pi = 3.14159
```

### String literal

Double-quoted text, of type `String`.

```i
greeting = "hello"
```

### List literal

Square brackets around comma-separated expressions, of type `List a`.

```i
nums = [1, 2, 3]
```

### Unit

The type `Unit` has one value, written `Unit`. Used as the result of effectful
procedures with no meaningful return.

```i
done : Unit
```

---

## 2. Operators (the four)

These are the entire operator surface for binding and structure. Arithmetic
and comparison operators aren't in this list; they desugar to trait methods
(see § 11).

### `:` — has-type

Attaches a type to a name. Appears in signatures, field declarations, and
fused with `=` for annotated bindings.

```i
greeting : String
```

### `=` — bind-value

Binds a name to a value. Inside a `type` block, defines a method or constant
on that type.

```i
double = n -> n * 2
```

### `->` — function

Separates parameters from body in a function value, and arg types from result
type in a function signature. Multiple lambda parameters are space-separated;
multiple argument types in a signature are comma-separated.

```i
add = a b -> a + b
```

### `.` — member access

Selects a field, method, or namespaced name. Used both for instance access
(`p.x`) and module/type qualification (`Std.IO.print`).

```i
p.x
Std.IO.print
```

### Parens — grouping

Round parens group an expression. They're not a function-call operator. You
only need them for nesting, or when writing constructor/update kwargs.

```i
add 3, (mul 4, 5)
```

### `!` — effect marker

Attached to a call (`f!`) marks an effectful call site. Attached in a type
(`! Eff`) names an effect row. See § 8.

```i
print! "hi"
```

---

## 3. Bindings

### Value binding

`name = expr` binds `name` to the value of `expr` in the enclosing scope.
Bindings are immutable. No rebinding, no mutation.

```i
greeting = "hello"
```

### Type annotation alone

`name : Type` declares a name's type without giving its value. Followed by a
later `name = expr` to provide the value.

```i
greeting : String
greeting = "hello"
```

### Annotated binding

`name : Type = expr` declares type and value on one line.

```i
greeting : String = "hello"
```

### Block body

A `name =` followed by an indented block uses the block as the value. The
last expression of the block is the value of the binding.

```i
main =
    print! "hi"
    print! "bye"
```

---

## 4. Functions

### Definition

`args -> body` is a function value. Parameters are space-separated lowercase
identifiers. There's no currying: `a b -> ...` is one two-argument
function, not a chain.

```i
add = a b -> a + b
```

### Lambda expression

The same `args -> body` form is a value anywhere an expression is expected.
Single- and multi-argument lambdas both pass directly inside calls without
extra parens — lambda parameters are space-separated and call arguments are
comma-separated, so the two never collide (see § 5).

```i
nums.map x -> x * 2
nums.fold 0, acc x -> acc + x
```

### Type signature

`name : args -> result`. Multiple argument types are comma-separated. An
effect row appears as `! Eff` between args and result.

```i
add   : Int, Int -> Int
print : String ! IO -> Unit
```

### Zero-argument function (effectful procedure)

A function with no parameters is written `! Eff -> result` in type position
and called with `name!`. Pure zero-argument values are not functions; bind
them with `=` to an expression.

```i
readLine : ! IO -> String
```

### Implicit `self`

A function bound with `=` inside a `type` block receives an implicit `self`
parameter referring to the instance. `self` does not appear in the parameter
list.

```i
type Point
    x : Float
    deltaX = other -> self.x - other.x
```

---

## 5. Calls

### Paren-free call

`f a, b` calls `f` with arguments `a` and `b`. The function is juxtaposed
with its arguments, and commas separate arguments. There's no separate
function-application operator.

```i
add 3, 4
```

### Nested call

When a call appears as an argument to another call, parens group it so its
commas are not read as more arguments to the outer call.

```i
add 3, (mul 4, 5)
```

### Method call

`instance.method args` looks up `method` on the type of `instance`, with
`instance` bound as `self`. Arguments follow the same paren-free rule.

```i
p1.distance p2
```

### Multi-argument lambda inside a call

Multi-argument lambdas pass through call argument lists without extra parens:
lambda parameters are space-separated and call arguments are comma-separated,
so the two separators don't collide.

```i
nums.fold 0, acc x -> acc + x       # acc and x are lambda params; 0 and the lambda are call args
```

### Where a lambda body ends

`->` is greedy on the right. The body extends until something else
in the *enclosing* construct claims the next token. The terminators are:

- a newline that returns to or below the indentation of the binding the
  lambda lives inside,
- a `,` at the enclosing call's argument depth,
- a `)` or `]` at the enclosing depth,
- the next pattern arm in an indented `match` block,
- end of input.

That's why `nums.fold 0, acc x -> acc + x` parses cleanly: the `,`
belongs to the outer call, not the lambda. To extend a body across
multiple lines, use an indented block:

```i
result =
    xs.fold initial, acc x ->
        cleaned = clean x
        acc.append cleaned
```

### Method chaining

`.` always binds to the immediately preceding *atom* — an identifier, a
paren group, a literal, a list literal. It doesn't reach back across a
call's arguments. So:

```i
nums.map double                # OK
nums.map double.filter pred    # parses as nums.map (double.filter pred)
(nums.map double).filter pred  # chain on a call result needs parens
```

The price is one paren pair per chain link. The reading rule pays for
itself: every `.` operates on the thing immediately to its left, and
that's it.

Long chains formatted across lines line up at `.`:

```i
nums
    .map double
    .filter positive
    .fold 0, acc x -> acc + x
```

Layout is convention; the parser only cares about the indentation of
the enclosing binding.

### Construction

`Type(field = val, ...)` constructs an instance of `Type` from keyword
arguments. The parens are required; they aren't just grouping. You have
to supply every field.

```i
p = Point(x = 0, y = 0)
```

### Record update

`instance(field = newVal, ...)` produces a copy of `instance` with the listed
fields replaced. Same surface as construction. The only difference is whether
the left side is a type or a value.

```i
p2 = p1(x = 5)
```

---

## 6. Type definitions

### `type` block

`type Name` followed by an indented block of members. The block is closed:
every field, method, and variant of the type lives in here. Inside the
block, the `Name.` prefix is implicit on every member.

```i
type Point
    x : Float
    y : Float
```

### Record fields

Lowercase identifiers introduced with `:` are fields.

```i
type Point
    x : Float
    y : Float
```

### Methods

Lowercase identifiers introduced with `=` inside a `type` block are methods
(if the value is a function) or constants. Methods receive an implicit
`self`.

```i
type Point
    distance = other -> ...
```

### Sum-type variants

Capitalized identifiers inside a `type` block are variants. A variant may
stand alone (no payload), have its own field block, or use the single-payload
shorthand (next entry).

```i
type Color
    Red
    Green
    Blue
```

### Variant with field block

A variant followed by an indented block lists that variant's fields, same
form as record fields.

```i
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float
```

### Single-payload variant shorthand

`Variant : T` is shorthand for a variant carrying exactly one anonymous
payload of type `T`. The payload is exposed as the variant itself in
patterns (`Some x` binds `x : a`), not as a named field.
The uppercase identifier marks this as a variant; a lowercase name with
`:` would be a field declaration instead.

```i
type Maybe a
    None
    Some : a
```

### Generics

A lowercase identifier in type position is a type variable. First use binds
it; no introducing keyword. Multiple type parameters are comma-separated
after the type name.

```i
type Result a, e
    Ok    : a
    Error : e
```

### Newtype (single-line form)

`type Name = T` declares a one-case wrapper around `T`. Nominally distinct
from `T`: values aren't interchangeable, even when the underlying type is.

```i
type UserId = Int
```

### Newtype (block form)

A `type` block with a single non-variant field is the same idea written
long-form.

```i
type UserId
    value : Int
```

---

## 7. Pattern matching

### `match` form

`expr match` followed by an indented block of `pattern -> body` arms. The
value of the matching arm's body is the value of the whole expression.

```i
shape match
    Circle r    -> ...
    Rect w, h   -> ...
```

### Literal pattern

A literal matches values equal to it.

```i
n match
    0   -> "zero"
    _   -> "nonzero"
```

### Identifier pattern

A lowercase identifier matches anything and binds the matched value to that
name.

```i
x match
    n   -> n + 1
```

### Wildcard pattern

`_` matches anything and binds nothing. Used for "don't care" arms.

```i
result match
    Ok _    -> "success"
    Error _ -> "failure"
```

### Constructor pattern

`Constructor args` matches values built with that constructor and destructures
its payload. Args are comma-separated lowercase identifiers (or further
nested patterns).

```i
shape match
    Circle r    -> ...
    Rect w, h   -> ...
```

### Single-payload constructor

For a variant declared with the `Variant : T` shorthand, the constructor
pattern binds the payload directly.

```i
m match
    None    -> 0
    Some v  -> v
```

### List pattern

Square brackets around comma-separated patterns match a list of exactly that
length and destructure its elements. Rest patterns (e.g., `[head, ...tail]`)
are TBD in v1.

```i
parts match
    [a, b]  -> ...
    _       -> ...
```

### Tuple pattern

Parens around comma-separated patterns match a tuple positionally. The shape
of the pattern matches the shape of the tuple type; `(a, b)` matches any
2-tuple and binds its first and second components. Tuple patterns also work
in lambda parameter position: `swap = (a, b) -> (b, a)`.

```i
pair match
    (a, b)  -> a + b
```

### Record destructuring

A `match` arm on a record uses the constructor name with kwargs, mirroring
construction syntax. (Records are sums with one implicit case.)

```i
p match
    Point(x = a, y = b) -> a + b
```

### Nested patterns

Patterns may contain patterns. Constructor arguments may themselves be
constructor, list, literal, identifier, or wildcard patterns.

```i
m match
    Some (Cons head, _) -> head
    _                   -> 0
```

### Exhaustiveness

The compiler rejects any `match` that doesn't cover every constructor of
the matched type. No fallthrough. A wildcard arm supplies the default when
you actually want one.

---

## 8. Effects

### Effectful call

A `!` after a function name at a call site marks the call as effectful. For
zero-argument procedures this is also how they are invoked (`readLine!`).

```i
print! "hello"
```

### Effect row in a type

`! Eff` appears between argument types and the result type, naming the effect
row. The row is usually inferred; you write it when documenting an interface.

```i
print : String ! IO -> Unit
```

### Effect inference

Effect rows propagate from callee to caller. A function that calls anything
effectful inherits that effect in its inferred type. You rarely write effect
rows by hand.

### v1 effects

`IO` (print, readLine, file ops), `State` (`Std.Ref` operations), and
`Env` (`Std.Env.args`, `Std.Env.var`). No others in v1.

### Explicit-pure callback

`! ()` is the empty effect row. Written as a callback's row, it pins
the callback as pure — no IO, no state, nothing. The implicit form
(no annotation) is effect-polymorphic; this is the one explicit
spelling available in v1.

```i
trait Show a
    show : (a -> String ! ())
```

Use it for trait methods that have to stay pure to make sense, and for
HOFs whose semantics depend on the callback being pure (a hash-keyed
cache, an idempotent retry). User-named row variables are out of v1;
see [effects.md § 7](effects.md).

---

## 9. `?` early-exit

### Form

`expr?` evaluates `expr`; if the result represents failure, the enclosing
function returns the failure immediately; otherwise the expression evaluates
to the unwrapped success value.

```i
n = parseInt s?
```

### Type rule

`?` works on both `Result` and `Maybe`:

- If `expr : Result a e` and the enclosing function returns `Result _ e`
  with the same error type, `expr?` unwraps `Ok v` to `v` and propagates
  `Error e` outward.
- If `expr : Maybe a` and the enclosing function returns `Maybe _`,
  `expr?` unwraps `Some v` to `v` and propagates `None` outward.

The two cases are syntactically identical; the type checker picks the right
interpretation from `expr`'s type and the enclosing function's return type.
The compiler still verifies every error path. `?` is sugar, not an escape
hatch.

```i
parsePoint = s ->
    Ok Point(x = parseFloat xs?, y = parseFloat ys?)

firstEven : List Int -> Maybe Int
firstEven = xs ->
    found = xs.find x -> x % 2 == 0
    Some (found?)
```

---

## 10. Modules and imports

### Module declaration

`module Name` on the first non-blank line of a file declares it as a module.
An indented `expose` clause lists the names visible to importers. Anything
not exposed is private.

```i
module Geometry
    expose Point(..), distance
```

### Type exports

A type name in `expose` comes in two forms. `expose Point` exposes the type
opaquely: outside code can hold `Point` values but cannot construct or
destructure them. `expose Point(..)` exposes the type *and* every constructor
(and, for records, every field). The opaque form enables smart-constructor
invariants — only functions in the defining module can build instances.

```i
expose Point          # type only — opaque
expose Point(..)      # type plus all constructors and fields
```

### Import (whole module)

`use Path` imports a module; its names are then accessed via `Path.name`.

```i
use Std.IO
```

### Import (cherry-pick)

`use Path (a, b)` imports specific names from a module into local scope
unqualified.

```i
use Std.IO (print, readLine)
```

### Import (alias)

`use Path as Alias` renames the imported module locally.

```i
use Std.Float as F
```

### Program entry

A program is a module named `Main` exposing a `main` value. `main`'s type
is `! IO -> Unit` (or whatever effect row its body inherits).

```i
module Main
    expose main
```

---

## 11. Operator desugaring

Arithmetic, comparison, and logical operators aren't built-in syntax. Each
one desugars to a trait method, and the trait dispatches on the operand type.
Every primitive type and most stdlib types implement the standard set.

| Operator | Desugars to     | Trait |
|----------|-----------------|-------|
| `a + b`  | `Add.add a, b`  | `Add` |
| `a - b`  | `Sub.sub a, b`  | `Sub` |
| `a * b`  | `Mul.mul a, b`  | `Mul` |
| `a / b`  | `Div.div a, b`  | `Div` |
| `a ^ b`  | `Pow.pow a, b`  | `Pow` |
| `-a`     | `Neg.neg a`     | `Neg` |
| `a == b` | `Eq.eq a, b`    | `Eq`  |
| `a /= b` | `Eq.ne a, b`    | `Eq`  |
| `a < b`  | `Ord.lt a, b`   | `Ord` |
| `a <= b` | `Ord.le a, b`   | `Ord` |
| `a > b`  | `Ord.gt a, b`   | `Ord` |
| `a >= b` | `Ord.ge a, b`   | `Ord` |
| `a ++ b` | `Concat.concat a, b` | `Concat` |

`and`, `or`, `not`, `xor` are functions in `Std.Bool`, not operators, and
are called paren-free like any other function.

`Show.show : a -> String` is the conversion used by `print!` and string
concatenation idioms (see [stdlib.md § `Show`](stdlib.md)).

### Precedence and associativity

Loose to tight. Operators on the same row have equal precedence.

| Rank | Operators                         | Associativity   |
|------|-----------------------------------|-----------------|
| 1    | `->` (lambda body)                | right; greedy   |
| 2    | `or`                              | function — paren-free call |
| 3    | `and`                             | function — paren-free call |
| 4    | `not`                             | function — paren-free call |
| 5    | `==`, `/=`, `<`, `<=`, `>`, `>=`  | non-associative |
| 6    | `++`                              | right           |
| 7    | binary `+`, `-`                   | left            |
| 8    | `*`, `/`                          | left            |
| 9    | `^`                               | right           |
| 10   | unary `-`                         | prefix          |
| 11   | function call (juxtaposition)     | left            |
| 12   | `.`, postfix `?`, postfix `!`     | left            |
| 13   | atoms — literals, identifiers, `()` | —             |

A few consequences:

- Comparisons don't chain. `a < b < c` is a parse error, not a
  conjunction.
- Arithmetic precedence is the standard one: `a + b * c` is
  `a + (b * c)`.
- Lambda `->` is the lowest-precedence thing in expression position, so
  the body extends as far right as possible until something else in the
  enclosing construct ends it (see § 5 *Where a lambda body ends*).
- `.` is at the very top, so `p.x + 1` is `(p.x) + 1`.

---

## 12. Traits

### Declaration

`trait Name a` followed by an indented block of method signatures. The type
variable after the trait name is the implementing type.

```i
trait Eq a
    eq : a, a -> Bool
```

### Implementation

`impl Trait Type` followed by an indented block of method definitions, one
per method declared in the trait.

```i
impl Eq Point
    eq = a b -> a.x == b.x and a.y == b.y
```

### Coherence

One implementation per `(trait, type)` pair anywhere in the program
(Haskell-style global coherence). A type may implement multiple traits.
There's no inheritance.
