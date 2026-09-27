---
id: e7c4a3f6-99b4-4587-b53e-b40cb0ea96a0
title: Blueprints as work orders
type: feature
status: dropped
milestone: houses
created: 2026-09-25
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: engine
area: building
---

## Why

Work orders (74b6fa7e) and blueprints are the same job: bring these things to a place, then work there. Construction still has its own Blueprint, Deliver and Construct. One mechanism would give building what orders have (inputs by tag, a tool requirement) and orders what building has (the deliver-first scoring, staged looks).

## What

- A blueprint becomes an `Order` whose completion is the engine's own: it becomes the building.
- `Deliver` and `Construct` fold into `Supply` and `Craft`, with no change in behaviour for building.
- Saves carry forward: `engine:blueprint` is read into orders on load.

## Acceptance criteria

- [ ] Every construction and deconstruction test passes unchanged
- [ ] A blueprint can need a tool, through the same `requires`
- [ ] Old saves with blueprints load

## 2026-09-26

Moved to houses and raised to p1: material tool gates (fff4fb42) and replace-in-place (b3ffbae1) build on builds being orders (DESIGN.md §6c).

## Proposed status: backlog -> dropped (Oddur Sigurdsson, 2026-09-26)

Its purpose was giving building what orders have (inputs by tag, a tool requirement). #165 (dce75339) since gave blueprints build.requires with tool fetching and 'Needs a ... tool' reasons, and the material tool gate (fff4fb42) builds on that by reading the blueprint's MadeOf. What's left is an internal refactor of Deliver/Construct into Supply/Craft that no Houses item needs and that touches ai.rs, which three agents are editing. Suggest dropping it, or parking it in the backlog without a milestone, until a mod needs a build input by tag.

## Accepted status: dropped (Oddur Sigurdsson, 2026-09-27)

Proposed by Oddur Sigurdsson on 2026-09-26.

## 2026-09-27

Dropped on the owner's delegation (2026-09-27, 'you make the decisions'), for the reason in the proposal. If a mod later needs a build input by tag, reopen it then.
