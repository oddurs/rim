---
id: 426
uid: eaf3496e-649b-44ff-8231-44fc8ae61a7f
title: 'rim story: run a story headless across seeds, print its chronicle and beat statistics'
type: feature
status: backlog
milestone: story
depends_on:
- 412
- 422
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: tooling
area: tests
pillar:
- plugin-first
---

## Acceptance criteria

- [ ] `rim story <premise> --seeds N --days D` with the installed mods
- [ ] Prints one run's chronicle with template text
- [ ] Beat statistics across seeds: fired, declined, expired, and when
- [ ] Parallel across seeds; 100 seeds × 120 days in minutes on the reference machine
