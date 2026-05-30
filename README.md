# i

A sparse, statically-typed language in development. Building spec first as a learning exercise.

 The syntax is as small as I could make it without giving up safety. Type and effect inference handle most of what would normally need syntax, leaving a handful of operators on the surface.

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

My primary goal is to learn something, but for the language itself, the
priorities are:

1. **Minimal.** Easy to learn and remember.
2. **Safe.** No surprises.
3. **Joyful.** Fun to write and read.

## Status

Docs are complete; lexer, parser, name resolution, and the type checker —
HM core plus traits and operator dispatch — are complete and tested.
Effects and totality are next. For the live state — phases, current
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
runs `make ci` before every commit. To set up after cloning:

```sh
make setup       # one-time; installs the git hook (npm) and fetches deps
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
