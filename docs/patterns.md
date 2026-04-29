# Pattern matching

A focused reference on `match` — the form, the pattern kinds, and the rules
the type checker enforces around them. For surface syntax in lookup form,
see [syntax.md § 7](syntax.md); for the introductory walkthrough, see
[tour.md § 5](tour.md). This manual is the *why*.

---

## 1. The `match` form

`expr match` introduces an indented block of `pattern -> body` arms. The
matched value is the value of `expr`; each arm's pattern is tested against
it in source order, and the first arm whose pattern matches supplies the
body that produces the result of the whole expression.

```i
shape match
    Circle r    -> 3.14159 * r^2
    Rect w, h   -> w * h
```

`match` is an *expression*: every arm yields a value, every arm's body has
the same type, and the type of the `match` is that common type. There is no
fallthrough — only one arm runs — and no implicit default; the compiler
requires the listed arms to cover the matched type's full set of cases
(see § 3).

`match` works on any sum type. Records are sums with one implicit case, so
the same form destructures them — see "Record destructuring" below.

---

## 2. Pattern kinds

A pattern is one of the following. Patterns may nest (§ 4); each kind below
describes the leaf form.

### Literal pattern

A literal value matches values equal to it. The literal's type must be the
type being matched.

```i
n match
    0   -> "zero"
    1   -> "one"
    _   -> "many"
```

`Bool`, `String`, and `Float` literals match the same way. Literal
patterns are not exhaustive on their own — there is no way to enumerate
every `Int` — so a literal-only match almost always needs a wildcard arm.

### Identifier pattern

A lowercase identifier matches *anything* and binds the matched value to
that name within the arm's body. This is how you "name what you matched"
without further destructuring.

```i
x match
    n   -> n + 1
```

An identifier arm is equivalent to a wildcard arm for exhaustiveness
purposes — the difference is whether the value is named.

### Wildcard pattern

`_` matches anything and binds nothing. Use it to express "I don't care
about this position."

```i
result match
    Ok _    -> "success"
    Error _ -> "failure"
```

### Constructor pattern

`Constructor args` matches values built with that variant and destructures
its payload into the listed sub-patterns. Args are comma-separated and may
themselves be any pattern, not just identifiers (see § 4).

```i
shape match
    Circle r    -> 3.14159 * r^2
    Rect w, h   -> w * h
```

Constructor patterns accept either positional or named-field binding.
`Rect w, h` binds `w = width` and `h = height` — fields in declaration
order — while `Rect(width = w, height = h)` names them explicitly. Both
forms are accepted; positional reads cleaner for small variants, named
for many-field ones. See [types.md § 4](types.md) for the field-order
rule.

For a variant declared with the single-payload shorthand `Variant : T`,
the pattern binds the payload directly: `Some v` binds `v : a`, not
`v.value : a`.

### List pattern

Square brackets around comma-separated patterns match a list of *exactly*
that length and destructure its elements.

```i
parts match
    []      -> "empty"
    [a]     -> "one"
    [a, b]  -> "two"
    _       -> "more"
```

v1 has not yet specified rest patterns and guards. They will be added in a
later spec revision. Until then, walking a list of unknown length goes
through the constructor form (`Empty` and `Cons` from `Std.List`) or
through `head`/`tail`/`map`/`fold`.

### Record destructuring

A record is a sum with one implicit case. `match` on a record uses the
constructor name with kwargs, mirroring the construction syntax:

```i
p match
    Point(x = a, y = b) -> a + b
```

The positional form `Point a, b` is also accepted and binds the same
fields in declaration order. Because a record has only one case, this
form is exhaustive without a wildcard.

---

## 3. Exhaustiveness

The compiler rejects any `match` that does not cover every constructor of
the matched type. An omitted case is a *compile-time error*, not a
warning, and the error names the missing constructor. There is no
fallthrough and no implicit default arm.

```i
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

# Compile error — `Rect` is not handled.
area = shape ->
    shape match
        Circle r    -> 3.14159 * r^2
```

The error reads roughly:

```
error[non-exhaustive-match]: missing case `Rect`
   at examples/03-shapes.i:9
       shape match
       ^^^^^
note: `Shape` has variants `Circle` and `Rect`; only `Circle` is matched
```

A wildcard or identifier arm supplies the default when you genuinely want
one. An always-redundant arm — one whose pattern can never match given
earlier arms — is reported as a warning.

Exhaustiveness is the language's most-relied-on safety property. Adding a
variant to a sum type forces the compiler to list every `match` site that
needs an arm for the new case, turning a refactor into a mechanical
checklist. See [types.md § 4](types.md) for the sum-type-design view.

The check operates at the level of constructors, not literal values. A
`match` on `Int` with arms `0`, `1`, `2` is not exhaustive — the compiler
cannot enumerate every `Int` — and requires a wildcard arm. The same
applies to `String`, `Float`, and any type whose value space is not
enumerated.

---

## 4. Nested patterns

Patterns nest. A constructor pattern's arguments may themselves be
constructor, list, literal, identifier, or wildcard patterns; the same
rule applies recursively for any depth.

```i
m match
    Some (Cons head, _) -> head
    Some Empty          -> 0
    None                -> 0
```

Each layer is checked independently for exhaustiveness against the type
at that position. The arms above cover the three cases of the outer
`Maybe (List a)`: a `Some` of `Cons`, a `Some` of `Empty`, and `None`. The
compiler tracks the cross-product as it descends and reports any missing
combination at the outermost site.

Nested patterns can also include literals and lists:

```i
result match
    Ok [x, y]       -> x + y
    Ok _            -> 0
    Error _         -> -1
```

`Ok [x, y]` matches an `Ok` of a two-element list; `Ok _` covers the rest
of the `Ok` cases; `Error _` covers the failure case. With nested literal
or fixed-length-list patterns you almost always need a wildcard at the
outer level, because the inner pattern is not itself exhaustive.

---

## 5. Guards

v1 has not yet specified rest patterns and guards. They will be added in a
later spec revision. Conditional refinement of an arm — "match `Some n`
*and* `n > 0`" — is currently expressed by matching first and branching
inside the arm body with a regular `match` on the condition.

---

## See also

- [syntax.md § 7](syntax.md) — every pattern form in lookup style.
- [tour.md § 5](tour.md) — guided introduction to `match`.
- [types.md § 4](types.md) — sum-type design and the exhaustiveness
  property at the type-system level.
- `examples/06-tree.i` — a recursive `match` over a generic tree, the
  worked example for this manual.
