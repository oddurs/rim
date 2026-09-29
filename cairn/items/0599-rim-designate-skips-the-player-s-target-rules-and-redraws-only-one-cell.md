---
id: 599
uid: 39cf8385-39ae-46ef-9f04-318af4de13aa
title: rim.designate skips the player's target rules and redraws only one cell
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: scripting
---

## What

`rim.designate` (script.rs ~1811-1824) inserts `Designated` on any thing: loose stacks, held tools, another faction's buildings, a designation that targets creatures on a thing. `designate_preview` (command.rs ~263) decides what a player's designation may mark. The binding also touches only `t.pos`, not the footprint, so a multi-cell thing isn't redrawn.

## How it fails

A mod can mark things no job can ever do, so their work shows as waiting forever, and big things draw stale.

## Reproduce

Not yet run; verified by reading. `rim.designate(stack_id, "chop")` succeeds, and `ai::work_waiting` counts it forever.

## Fix

Check the designation's targets and the thing's harvest as `designate_preview` does: a script error, or a false return. Touch the whole footprint.

## Acceptance

- [ ] A script can mark only what the player could
- [ ] A test that fails before the fix and passes after
