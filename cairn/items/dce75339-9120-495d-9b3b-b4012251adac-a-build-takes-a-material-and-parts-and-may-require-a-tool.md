---
id: dce75339-9120-495d-9b3b-b4012251adac
title: A build takes a material and parts, and may require a tool
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
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

- [x] A def with stuff and cost builds only when both are delivered (test)
- [x] Cancelling a half-delivered blueprint refunds both (test)
- [x] A build that requires `sawing` waits for a sawing tool and says so (test)
- [x] Determinism test passes

## 2026-09-26

build takes stuff and cost together: the blueprint's cost is the material (stuff.count of the chosen one) first, then cost's parts, delivered in that order; cancel refunds what was delivered of each, and cost_of (taking it down) gives back both at the refund rate. build.requires is tool tags like a harvest's: its tags join the tool-tag bits, choose_work fetches a tool with tool_for once the materials are in (the walk counts), Job::Construct gained tool: Option<Entity> with a serde default so older saves load, and a finished build wears the tool. A tool doesn't change build speed (harvests do); left for when a real build asks. work_blocked now answers for a plan whose tool nobody has ('Needs a sawing tool.'), and the right-click menu lists 'Build X' with 'no sawing' when it can't. The dock's cost line already joined material and parts.
