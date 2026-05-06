# i

A sparse, statically-typed language in development. Building spec first as a
learning exercise.

The surface is as sparse as I could make it without giving up safety — full
type and effect inference, with the structural surface stripped down to a
handful of operators. (Four of them carry the binding structure: `:`, `=`,
`->`, `.`. Arithmetic, `!`, `?`, and the rest sit on top.)

```i
type Point
    x : Float
    y : Float
    distance = other ->
        ((self.x - other.x)^2 + (self.y - other.y)^2)^0.5

main =
    p1 = Point(x = 0, y = 0)
    p2 = Point(x = 3, y = 4)
    print! "distance: " ++ show (p1.distance p2)
```

## What this is

A learning project I'm doing in public. I want to understand how a
compiler works by actually building one — with a real spec, not a toy
grammar. I'm working through it slowly, one task at a time; the commits
and the plans under `docs/superpowers/plans/` show what I did and in what
order, if you want to follow along.

Nothing runs yet. The design is opinionated and the spec is committed,
but the implementation is half a lexer and stops there. Don't try to use
it. Not looking for contributors right now.

Priorities, in order:

1. **Aesthetic minimalism.** The program on screen should look like the idea
   you have in your head.
2. **Fits in your head.** Small enough that the whole core stays in working
   memory once you've used it for a while. (Not "afternoon-fast" if you've
   never met type and effect inference before — that's a longer ramp.)
3. **Ergonomic.** Sparse to help, not to puzzle.

## Status

Docs are complete; lexer is complete and snapshot-tested; parser and
everything after it are unwritten. For the live state — phases, current
plan, task-level checkboxes — see
[`docs/superpowers/plans/PROGRESS.md`](docs/superpowers/plans/PROGRESS.md).

## Development

Common commands all sit behind `make`:

```sh
make ci          # what GitHub Actions runs (fmt-check + lint + test)
make dev         # cargo test
make rev         # cargo insta review (snapshot review)
make help        # full list
```

A pre-commit hook (managed by [husky](https://typicode.github.io/husky/))
runs `make ci` before every commit. To wire it up after cloning:

```sh
npm install      # one-time; installs husky and runs `prepare`
```

The Node dependency is *only* for the pre-commit hook — the compiler itself
is pure Rust.

## Documentation

- [Tour](docs/tour.md) — start here
- [Syntax reference](docs/syntax.md)
- [Type system](docs/types.md)
- [Effect system](docs/effects.md)
- [Pattern matching](docs/patterns.md)
- [Standard library](docs/stdlib.md)
- [Modules](docs/modules.md)
- [Building and running](docs/building.md)
- [Limitations](docs/limitations.md)

## License

MIT. See [LICENSE](LICENSE).
