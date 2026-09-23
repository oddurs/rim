---
id: 151
title: Mod dependencies with version ranges, optional deps and incompatibilities
type: feature
status: backlog
milestone: platform
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

`depends = ["core"]` has no versions. With mods updating on their own schedules, a mod must be able to say which versions it works with, and the resolver must pick one version per mod (mods share one world, so npm-style duplicates are impossible).

## Acceptance criteria

- [ ] `depends = { core = "^0.2" }`, `optional = { ... }`, `incompatible = [...]`; the list form still accepted
- [ ] Resolver picks one version per mod or explains why it can't
- [ ] Load-order rules unchanged: `depends` and `optional` imply `load_after`
