# Modules and imports

A focused reference on the module system. For surface forms, see
[syntax.md § 10](syntax.md); for an introduction, see
[tour.md § 9](tour.md). This manual is the *why*.

The module system in `i` is deliberately small. A file is a module; the
first line says what it exports; `use` brings other modules in. That is
the whole mechanism.

---

## 1. One file is one module

Every `.i` file is exactly one module. There is no syntax for declaring
two modules in one file, or one module across two. The mapping is one-
to-one in both directions.

The module's name is set by the `module` declaration, not by the
filename. Filename and module name are kept in sync by convention
(§ 4); they are not enforced to match.

A module's *path* — the dotted name after `use` — mirrors the directory
structure. A file `src/Std/IO.i` declaring `module Std.IO` is imported
as `use Std.IO`. Compound names like `Std.IO` are flat module names
that happen to contain dots; there is no submodule hierarchy in the
type-system sense — `Std.IO` does not "live inside" `Std` as a value.

---

## 2. `module` and `expose`

The first non-blank, non-comment line of every file declares the module:

```i
module Geometry
    expose Point, distance
```

`module Name` names the module. The indented `expose` clause lists the
names that are visible to importers — types, functions, constants, and
traits. Anything bound or declared elsewhere in the file but not named in
`expose` is private and inaccessible outside this file.

`expose` accepts type names, value names, and trait names. Exposing a
type exposes its variants and constructors with it; v1 has no hidden-
constructor mechanism. Multiple `expose` lines concatenate; order does
not matter.

---

## 3. `use`

`use` is the import form. It has three shapes.

**Whole module.** `use Path` makes names from the module accessible as
`Path.name`:

```i
use Std.IO

main = Std.IO.print! "hello"
```

**Cherry-pick.** `use Path (a, b, ...)` brings specific names into local
scope unqualified:

```i
use Std.IO (print, readLine)
```

**Alias.** `use Path as Alias` renames the module locally. The original
path is unavailable in this file; only the alias is:

```i
use Std.Float as F
x = F.parse "3.14"
```

Cherry-pick and alias do not combine in v1 — `use Path as A (x, y)` is
not valid. Write two `use` lines if you need both. `use` lines go
between the `module` declaration and the rest of the file; convention
groups them contiguously.

---

## 4. Project layout

`i` does not enforce a project layout. The convention used by the
examples and the standard library is:

```
my-project/
    src/
        Main.i           # module Main
        Geometry.i       # module Geometry
        Std/
            IO.i         # module Std.IO
            Float.i      # module Std.Float
```

The entry point is `src/Main.i`, declaring `module Main` and exposing
`main`. Every other file in `src/` is a module whose declared name
matches its path with `/` rewritten as `.` and the `.i` extension
dropped: `src/Geometry.i` declares `module Geometry`, and
`src/Std/IO.i` declares `module Std.IO`.

This is convention, not enforced. The compiler resolves `use Path` by
locating a file whose `module` declaration names that path; the
directory structure is how it finds the file, but the declaration is
the source of truth.

A worked two-file program lives in `examples/08-modules-lib.i` and
`examples/08-modules-app.i`. `Geometry` exposes `Point` and `distance`;
`Main` cherry-picks both and uses them unqualified:

```i
# examples/08-modules-lib.i
module Geometry
    expose Point, distance

type Point
    x : Float
    y : Float

distance = a, b ->
    ((a.x - b.x)^2 + (a.y - b.y)^2)^0.5
```

```i
# examples/08-modules-app.i
module Main
    expose main

use Geometry (Point, distance)
use Std.IO (print)

main =
    p1 = Point(x = 0, y = 0)
    p2 = Point(x = 3, y = 4)
    print! "distance: " ++ show (distance p1, p2)
```

---

## 5. Visibility rules

The visibility rule is a single sentence: **only names listed in
`expose` are accessible from other modules.** Everything else in the
file — helper functions, local types, intermediate constants — is
private.

```i
module Geometry
    expose distance

square = x -> x * x          # private helper

distance = a, b ->
    (square (a.x - b.x) + square (a.y - b.y))^0.5
```

`square` is invisible to importers of `Geometry`. Calling `Geometry.square`
from another module is a compile-time error, and a cherry-pick `use
Geometry (square)` fails for the same reason: the name is not exported.

Visibility is checked at the module boundary, not within the file.
There is no protected, internal, or friend visibility — a name is
either exposed or private. The two-level model is intentional:
refining further would multiply the rules a reader must hold in their
head, and the same discipline can be achieved by splitting modules.

A function that returns a private type cannot itself be exposed — the
type would leak through the signature. The compiler rejects this at
the exposure site.

---

## 6. Circular imports

**Modules form a directed acyclic graph in v1.** If module `A` uses
module `B`, then `B` cannot use `A` directly or transitively. A cycle is
a compile-time error, naming the cycle's members.

The acyclic restriction is a v1 commitment. It keeps the resolver
simple and makes type-checking a strict topological pass. Most programs
structure cleanly as a DAG; cases where two modules genuinely need each
other usually indicate that they should be one module, or that a third
module should hold the shared types.

If you find yourself wanting a cycle:

- **Extract.** Move the shared types into a third module both import.
- **Merge.** If two modules depend on each other's implementation, they
  are one module split unhelpfully across files.
- **Invert.** Pass the dependency in as an argument rather than
  importing it.

A future version may relax this; v1 does not.

---

## See also

- [syntax.md § 10](syntax.md) — every module form, look-up style.
- [tour.md § 9](tour.md) — guided introduction.
- [stdlib.md](stdlib.md) — the standard library, module by module.
- [building.md](building.md) — how the compiler resolves a project.
