---
id: eaf91831-765b-44d0-8ffd-f7c9a49ea44d
title: Headless runner for soak tests and benchmarks
type: chore
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: s
layer: tooling
area: tests
pillar:
- performance
---

## Why

Run N days with no window and print ms per tick, events and colony outcome.

## Acceptance criteria

- [x] cargo run --example headless -- --days N --seed S
- [x] Reports mean and p99 tick time

## 2026-09-22

examples/headless.rs. Seed 42, 4 days: mean 0.002 ms/tick, p99 0.005 ms, max 0.22 ms.
