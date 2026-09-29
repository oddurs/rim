---
id: 571dca56-98bc-4eed-a067-ed2dab075441
title: 'A pack: worn on the back, it carries more and more kinds'
type: feature
status: backlog
milestone: crafting
depends_on:
- 7691ffc6-817f-4209-a958-eff4e4e3aa3f
- 6b156431-0a93-4bea-864e-ae55a04e63e0
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: m
layer: engine
area: sim
api: additive
---

## Problem

A colonist carries one stack. The user wants backpacks later in the game.

## Proposal

A pack is apparel on a back layer with `pack = {mass, stacks}`. Worn, it adds that much carrying capacity and that many extra stacks, so a colonist can haul several kinds in one trip, and later carry food and medicine away from the colony. The mechanism lives in the engine, read from the def field. The pack itself is crafted content, made at the tailoring bench (6b156431).

Doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4.

## Acceptance criteria

- [ ] A test: a colonist wearing a pack hauls two kinds in one trip, within the pack's mass
- [ ] A craftable pack in the first-party content, with its recipe
