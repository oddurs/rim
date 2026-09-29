---
id: 151
uid: dbb8f031-f057-4c50-839a-e2330b2e5a78
title: Mod dependencies with version ranges, optional deps and incompatibilities
type: feature
status: backlog
milestone: platform
created: 2026-09-23
updated: 2026-09-27
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

- [ ] `depends = { core = "^0.2" }`, `optional = { ... }`, `breaks = [{ id, versions, why }]`; the list form still accepted
- [ ] `breaks` refuses the pair and shows its `why`; there is no warn-only incompatibility field
- [ ] Resolver picks one version per mod or explains why it can't
- [ ] Load-order rules unchanged: `depends` and `optional` imply `load_after`

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): incompatibility is `breaks` with a required `why`, which refuses the pair. There is no warn-only field: RimWorld's incompatibleWith only warns and gets ignored, and contested slots have the ladder (dbb92ebe). Fabric's split of breaks and conflicts informed this; Factorio's ~ (required without ordering) was considered and left out until a mod needs it.
