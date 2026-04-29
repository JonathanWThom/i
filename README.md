# i

A tiny, statically typed, compiled-ish language whose surface is as sparse as
possible without giving up safety. Think Roc-grade safety in a syntax pared
down to four operators.

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

A learning project, designed as a real language. The goal is something usable
at the end — not a toy. The design priorities, in order:

1. **Aesthetic minimalism.** A program looks like the idea it expresses.
2. **Fits in your head.** The whole core can be learned in an afternoon.
3. **Ergonomic.** Sparse to *help* you, not to puzzle you.

## Status

**Documentation complete.** The end-state user docs are written; every
example program in `examples/` is a complete, syntactically valid `i`
program — though none of them are runnable yet because the implementation
doesn't exist.

**Implementation: not started.** Plan 2 (lexer + parser) is next.

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
