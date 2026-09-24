---
id: 7e68156c-8ea0-4d65-90a3-a6f7a03b67b7
title: 'Cargo workspace: headless sim crate and client crate'
type: chore
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: s
layer: tooling
area: sim
pillar:
- determinism
---

## Why

The sim must build and run with no graphics so tests, benchmarks and servers can use it.

## Acceptance criteria

- [ ] rim_sim has no rendering dependencies
- [ ] rim_client depends on rim_sim only through its public API
