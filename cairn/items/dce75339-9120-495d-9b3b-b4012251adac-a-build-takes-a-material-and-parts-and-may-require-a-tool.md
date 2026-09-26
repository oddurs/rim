---
id: dce75339-9120-495d-9b3b-b4012251adac
title: A build takes a material and parts, and may require a tool
type: feature
status: backlog
milestone: crafting
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: building
---

## Why

`build` takes a material (`stuff`) or a fixed `cost`, never both (`defs.rs:1478`), so a plank wall can't be "planks and nails", and builds can't require a tool the way harvests and recipes can. DESIGN.md §4f. The houses work (e7c4a3f6, blueprints as orders) builds on this, so it lands first.

## What

- `build` accepts `stuff` and `cost` together: the material sets the factors, the cost lists the parts. Blueprints deliver both; cancelling refunds both.
- `build.requires`: tool tags the builder must hold, fetched like a harvest's tool.
- The dock's cost line shows both.

## Acceptance criteria

- [ ] A def with stuff and cost builds only when both are delivered (test)
- [ ] Cancelling a half-delivered blueprint refunds both (test)
- [ ] A build that requires `sawing` waits for a sawing tool and says so (test)
- [ ] Determinism test passes
