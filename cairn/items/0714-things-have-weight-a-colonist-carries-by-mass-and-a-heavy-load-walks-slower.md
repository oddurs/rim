---
id: 714
uid: 7691ffc6-817f-4209-a958-eff4e4e3aa3f
title: 'Things have weight: a colonist carries by mass, and a heavy load walks slower'
type: feature
status: backlog
milestone: carrying
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: engine
area: sim
api: additive
---

## Problem

A colonist carries up to 75 of anything: `CARRY_CAPACITY` (crates/rim_sim/src/world.rs:18, used at ai.rs:1199, 1607, 1724 and 1769). A load of stone and a load of berries weigh the same and walk at the same pace. The user wants heavy things carried slower and in smaller stacks, with berries in bigger ones.

## Proposal

- Item defs get `mass`, in grams per unit, as an integer so the sim stays deterministic.
- A pawn's carrying capacity in grams comes from core's defs, not the engine. A pickup takes the least of the stack, the room at the destination, the stack limit and capacity ÷ mass. `CARRY_CAPACITY` goes.
- Walking slows with the load: full speed up to half the capacity, easing to a floor at full. Core's defs set the curve; the engine reads it.
- The load is worked out from the carried lot when it's needed and never stored, so a load has nothing to rebuild.
- An item without `mass` loads as 1 kg, and `rim check` names it. Core and the first-party mods get real masses, for example 50 g a berry and several kilograms a stone chunk.

This changes every haul, so replays of old logs play differently and the change needs the full cross-platform run and a balance look at a haul-heavy seed.

Doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4.

## Acceptance criteria

- [ ] A test: a colonist hauls berries in a bigger stack than stone, and the stone load walks measurably slower
- [ ] Every item in core and the first-party mods has a mass, and `rim check` is clean
- [ ] Tick time at 250² with 50 colonists is unchanged on the scaling bench
- [ ] A haul-heavy seed's days to fill its stores is reported before and after
