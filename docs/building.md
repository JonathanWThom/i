# Building and running

> **Status:** This doc describes commands that don't exist yet. The Rust
> implementation lands in the next plan. Everything below is the contract
> the implementation gets measured against: what `i` programs will look
> like to run, what the directory layout will be, and what an error
> message from the type-checker should read like.

The CLI for `i` is a single binary, also called `i`, with a small set of
subcommands. Until you have more than one file, `i run main.i` is the
only command that matters.

---

## 1. Installation

Once published, the canonical install is:

```
cargo install i-lang
```

This builds the `i` binary from source and drops it on your `$PATH`. The
stdlib ships inside the binary. There's no separate package and no
version manager. Verify with `i --version`.

Until the implementation lands, none of the commands below actually run.

---

## 2. Project layout

`i` programs come in two sizes. A *single-file* program is one `.i` file
you run directly — no project, no manifest, no directories. This is what
the `examples/` directory contains, and it's the right shape for scripts
and exercises:

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

There's no `i.toml` or equivalent in v1. The project is just the `src/`
directory; the compiler walks it to resolve `use` declarations. If you run
`i run src/Main.i`, the project root is implicitly the parent directory
of `src/`.

A future version may add a manifest for dependencies and metadata. v1 has
no third-party packages, so there's nothing to manifest.

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

Arguments after `--` are passed to the program and surface through
[`Std.Env.args`](stdlib.md). The CLI strips them before parsing its own
flags, so `--` is the boundary: anything after it is the program's, not
the `i` runner's.

### `i check <file>`

Type-checks the program and exits. Does not run it. Useful in editors,
pre-commit hooks, and CI:

```
i check src/Main.i
```

`i check` produces the same diagnostics as `i run` but never reaches the
evaluator. A clean check exits `0`; a type error exits `1` and prints the
errors as described in § 5.

### `i fmt <file>` *(planned for later)*

`i fmt` will format an `.i` file in place to the canonical layout. It's
**not in v1**. Pinning the formatter before the language has been used
tends to lock in awkward choices. Until then, follow the conventions shown
throughout this documentation and the `examples/` directory.

---

## 4. Distributing programs

**v1 can't produce a standalone binary.** The implementation is a
tree-walking interpreter, so the only way to run an `i` program is to
invoke `i run` on the source. To share a program, ship the `.i` files
(and the `src/` tree if it's multi-file) and tell the recipient to
install `i` and run them.

This isn't a permanent state. Two future commands are planned but not in
v1:

### `i build <entry>` *(planned for v2)*

Compiles a program to bytecode. The output is a `.ic` file that runs on
the `i` bytecode VM, which ships inside the same `i` binary that v1's
interpreter does.

```
i build src/Main.i -o my-program.ic   # produces a .ic artifact
i exec my-program.ic                  # runs it (planned alongside i build)
```

A `.ic` file is portable across platforms — the VM is the same everywhere
— but still needs `i` installed to run. The win over `i run` is startup
time and execution speed. The cost is an extra build step.

### `i compile <entry> --target native` *(planned for v3)*

Compiles a program to a self-contained native executable for the host
platform. No `i` runtime needed on the target machine; the binary is the
binary.

```
i compile src/Main.i --target native -o my-program
./my-program
```

The native backend (Cranelift, LLVM, or custom) is undecided — see
[limitations.md § 5](limitations.md) and the Known TBDs at the bottom of
that doc. Until v3 lands, "produce a binary" means "ship the source and
expect the recipient to have `i` installed."

---

## 5. Errors

Type errors are the messages you'll see most often. The shape is
top-down: what went wrong, where, the offending span, and a note that
explains *why* the checker rejected it.

Say someone adapts `examples/04-list-map.i` and forgets that the list
holds strings instead of ints:

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

`?` mismatches are the third common shape. `?` works on both `Result`
and `Maybe`, but the inner expression's shape has to match the
enclosing function's return type. Mixing them produces:

```
error[question-mismatch]: cannot use `?` on a `Result` inside a function returning `Maybe`
   at src/Main.i:10
       Some (parseInt s?)
                       ^
note: enclosing function returns `Maybe Int`, but `parseInt s : Result Int ParseError`
hint: change the function to return `Result _ ParseError`, or `match` on
      the `Result` and convert the `Error` arm to `None` explicitly.
```

The compiler refuses to silently drop the error, since that's the
specific failure mode `?` exists to make visible. The fix is either to
adapt the value at the call site or to handle both arms explicitly.

Diagnostics are stable, scriptable text. No color codes when stdout
isn't a TTY, no progress bars, no spinners.

---

## 6. Examples

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
i run examples/09-effect-map.i   # effect-polymorphic map (pure + ! IO)
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

If any example fails to check, the docs and the implementation have
drifted.

---

## See also

- [tour.md](tour.md) — narrative introduction; start here if you have not
  read any `i` yet.
- [modules.md § 4](modules.md) — the project-layout convention this doc
  builds on.
- [syntax.md](syntax.md) — every form the parser accepts.
- [stdlib.md](stdlib.md) — what `print!`, `readLine!`, and friends do.
