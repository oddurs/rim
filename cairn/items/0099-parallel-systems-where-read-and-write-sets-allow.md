---
id: 99
title: Parallel systems where read and write sets allow
type: spike
status: backlog
milestone: scale
created: 2026-09-22
updated: 2026-09-22
priority: p2
api: none
effort: m
layer: engine
area: perf
pillar:
- performance
- determinism
---

## Question

Which systems can run in parallel without breaking determinism?

## Options

- Parallel read phase, serial apply
- Per-chunk parallelism
- Not worth it yet

## Decision


## Acceptance criteria

- [ ] Decision recorded in DESIGN.md
