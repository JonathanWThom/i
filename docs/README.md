# i — Documentation

End-state documentation for the `i` language. Some of it describes features
the compiler doesn't implement yet; check the Status line at the top of each
doc.

## If you're new, read in this order

1. [Tour](tour.md) — narrative introduction with runnable examples
2. [Building and running](building.md) — install, project layout, CLI
3. [Pattern matching](patterns.md), then [Types](types.md), then [Effects](effects.md)
4. [Modules](modules.md)
5. [Standard library](stdlib.md)

## Reference (random access)

- [Syntax](syntax.md) — every form, every operator
- [Standard library](stdlib.md) — every type and function in v1
- [Limitations](limitations.md) — what v1 doesn't do
- [Name resolution](resolution.md) — what every identifier refers to
- [Type checking](checker.md) — Hindley-Milner inference and exhaustiveness
- [Testing strategy](testing.md) — how the compiler is tested, layer by layer

## Specs and plans

- [Design spec](superpowers/specs/2026-04-27-i-language-design.md)
- [Plans](superpowers/plans/) — implementation roadmap
