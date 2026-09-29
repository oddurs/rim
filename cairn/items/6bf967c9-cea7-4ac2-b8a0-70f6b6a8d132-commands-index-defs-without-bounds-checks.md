---
id: 6bf967c9-cea7-4ac2-b8a0-70f6b6a8d132
title: Commands index defs without bounds checks
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: sim
---

## What

`command::apply` validates most ids (`SetPriority`, `SetStance`, `PlacePlan`, filters), but:
- `Designate` indexes `defs.designations[designation]` (command.rs ~274).
- `Build` calls `defs.thing(thing)` (~389), and `is_material_for(stuff)`.

An out-of-range id panics the sim.

## How it fails

The UI sends valid ids, and save logs replay only under the same mods, so today this takes a bad client or a hand-edited log. Once commands come from co-op peers, one bad command crashes every peer.

## Reproduce

Verified by reading. `command::apply(w, Command::Build { thing: u16::MAX, .. })` panics.

## Fix

Check every def id a command carries, as `SetPriority` does, and ignore the command when one is out of range.

## Acceptance

- [ ] No command can panic the sim
- [ ] A test that fails before the fix and passes after
