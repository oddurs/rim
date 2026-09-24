---
id: ded7881a-1a0d-4ded-86a7-8c638bfe5d10
title: 'Deterministic math in scripts: replace library trig and exp'
type: feature
status: done
milestone: plugin-api
depends_on:
- 05dbb688-66e3-47b0-b105-32651a8ebec0
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: breaking
effort: s
layer: engine
area: scripting
pillar:
- determinism
---

## Why

Luau's `math.sin`, `math.cos`, `math.exp`, `math.log` and `math.pow` call the platform C library, which can differ in the last bit between macOS, Windows and Linux. That is enough to desync co-op or break a replay once a mod uses them (DESIGN.md §10).

## Acceptance criteria

- [x] Those functions replaced in the sandbox by deterministic implementations with documented accuracy
- [x] Test vectors that must match exactly on all CI platforms

## 2026-09-23

Less pressing after the Weather sprint: 20_climate.luau (the one script that needed smooth curves) becomes data in 0183, evaluated in fixed point by the engine. Scripts still need a safe alternative for anything else periodic.

## 2026-09-24

Done in PR #24. The sim VM's math.sin, cos, tan, asin, acos, atan, atan2, exp, log, log10, pow, sinh, cosh and tanh are the libm crate's (a Rust port of musl): same code, same bits on every machine; accuracy under 1 ulp, math.log(x, base) exact for bases 2 and 10 and log(x)/log(base) otherwise (documented in docs/modding/scripting.md). Measured here: the platform library differs from musl in 1-10% of inputs (sin 905/20000, exp 1968/20000, pow 1962/20000). The same functions are disabled as Luau compiler builtins, else safe environments fast-call the C versions and literal calls are constant-folded with the compiler's library (verified: math.tan(0.7) was one bit off without it). The ^ operator calls the platform pow except literal exponents 2, 3 and 0.5; the loader warns about every other use with file and line. tests/scripting.rs: bit-exact vectors through direct, aliased and literal calls, two chosen where Apple's library disagrees, so macOS CI catches a regression; the cross-platform half of the check is 0157's crosscheck job.
