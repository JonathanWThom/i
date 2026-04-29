# Building and running

> **Status:** This doc describes commands that do not yet exist. The Rust
> implementation arrives in the next plan. Everything below is the contract
> the implementation will be measured against — what `i` programs will look
> like to run, what the directory layout will be, and what an error message
> from the type-checker should read like.

The CLI for `i` is a single binary, also called `i`, with a small set of
subcommands. `i run main.i` is the only command that matters until you have
more than one file.

---

## 1. Installation

Once published, the canonical install is:

```
cargo install i-lang
```

This builds the `i` binary from source and drops it on your `$PATH`. The
stdlib ships inside the binary; there is no separate package and no version
manager. Verify with `i --version`.

Until the implementation lands, none of the commands below run.

---

## 2. Project layout

`i` programs come in two sizes. A *single-file* program is one `.i` file you
run directly — no project, no manifest, no directories. This is what the
`examples/` directory contains, and it is the right shape for scripts and
exercises:

```
hello.i
```

A *multi-file* project follows the convention from
[modules.md § 4](modules.md): sources live under `src/`, the entry point is
`src/Main.i`, and every other file's path mirrors its module name.

```
my-project/
    src/
        Main.i           # module Main
        Geometry.i       # module Geometry
        Std/
            IO.i         # module Std.IO
```

There is no `i.toml` or equivalent in v1. The project is just the `src/`
directory; the compiler walks it to resolve `use` declarations. If you run
`i run src/Main.i`, the project root is implicitly the parent directory of
`src/`.

A future version may add a manifest for dependencies and metadata. v1 has
no third-party packages, so there is nothing to manifest.

---

## 3. Commands

### `i run <file>`

Type-checks the program and, if checking succeeds, executes it.

```
i run examples/01-hello.i
```

`i run` is the everyday command. It does the full pipeline — parse,
type-check, lower, evaluate — and prints whatever the program prints. Exit
code is `0` on clean run, `1` on a type error, `2` on a runtime error.

Arguments after `--` are passed to the program (once `i` exposes a `Std.Env`
module to read them; until then `--` is reserved but inert).

### `i check <file>`

Type-checks the program and exits. Does not run it. Useful in editors,
pre-commit hooks, and CI:

```
i check src/Main.i
```

`i check` produces the same diagnostics as `i run` but never reaches the
evaluator. A clean check exits `0`; a type error exits `1` and prints the
errors as described in § 4.

### `i fmt <file>` *(planned for later)*

`i fmt` will format an `.i` file in place to the canonical layout. It is
**not included in v1** — pinning the formatter before the language has been
used tends to lock in awkward choices. Until then, follow the conventions
shown throughout this documentation and the `examples/` directory.

---

## 4. Errors

Type errors are the messages you will see most often. They are designed to
read top-down: what went wrong, where it went wrong, the offending span,
and a note explaining *why* the checker rejected it.

Suppose a programmer adapts `examples/04-list-map.i` and forgets that the
list holds strings rather than ints:

```i
module Main
    expose main

main =
    nums = ["1", "2", "3"]
    doubled = nums.map x -> x * 2
    print! "doubled: " ++ show doubled
```

`i check` reports:

```
error[type-mismatch]: expected `Int`, found `String`
   at examples/04-list-map.i:5
       doubled = nums.map x -> x * 2
                              ^^^^^^
note: `nums` is a `List String`, but `*` requires `List Int`
```

Every type-error diagnostic has the same four parts:

1. A short error code (`type-mismatch`, `unbound-name`, `effect-leak`, …)
   and a one-line summary.
2. A `path:line` location.
3. The source line with a caret span pointing at the offending expression.
4. A `note:` explaining the inference that produced the error — what was
   expected, what was found, and which inference step decided.

Effect errors share the format. Calling `print!` from a function that the
checker has marked pure produces:

```
error[effect-leak]: function `double` is declared pure, but calls `print!`
   at src/Main.i:6
       double = n -> print! (show n); n * 2
                     ^^^^^^^^^^^^^^^^
note: remove the `!` call, or annotate `double` with `! IO`
```

Diagnostics are stable, scriptable text — no colour codes when stdout is
not a TTY, no progress bars, no spinners.

---

## 5. Examples

The `examples/` directory at the repo root holds the programs referenced
throughout this documentation. Each is single-file and self-contained
except for the two-file modules example. To run them once the
implementation lands:

```
i run examples/01-hello.i        # "hello, world"
i run examples/02-greet.i        # readLine + print
i run examples/03-shapes.i       # union types and pattern matching
i run examples/04-list-map.i     # list pipelines
i run examples/05-effects.i      # IO effect propagation
i run examples/06-tree.i         # recursive types
i run examples/07-result.i       # Result and the `?` operator
```

The two-file modules example runs against the project root rather than a
single file:

```
i run examples/08-modules-app.i
```

The compiler resolves `use Geometry` to `examples/08-modules-lib.i` by
walking from the entry file's directory; in a real project this would be
`src/Geometry.i` instead.

To type-check every example without running them:

```
for f in examples/*.i; do i check "$f"; done
```

If any example fails to check, the documentation and the implementation
have drifted.

---

## See also

- [tour.md](tour.md) — narrative introduction; start here if you have not
  read any `i` yet.
- [modules.md § 4](modules.md) — the project-layout convention this doc
  builds on.
- [syntax.md](syntax.md) — every form the parser accepts.
- [stdlib.md](stdlib.md) — what `print!`, `readLine!`, and friends do.
