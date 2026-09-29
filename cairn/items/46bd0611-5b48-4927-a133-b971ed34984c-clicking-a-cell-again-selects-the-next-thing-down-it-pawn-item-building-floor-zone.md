---
id: 46bd0611-5b48-4927-a133-b971ed34984c
title: 'Clicking a cell again selects the next thing down it: pawn, item, building, floor, zone'
type: feature
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: client
area: ui
api: none
---

## Problem

A left-click picks the pawn under the cursor, or else item > fixture > floor (crates/rim_client/src/main.rs:1932-1936, 2080-2093). A zone is picked only when nothing else is there. Floors are things, so a stockpile on a floor, or one with an item on it, can never be selected from the map. The only way in is the Zones tray. Clicking the same cell again doesn't cycle.

## Proposal

As in RimWorld: the first click takes the top thing, and each further click on the same cell takes the next one down, through pawns, items, buildings, the floor and the zone, then wraps. Clicking another cell starts again from the top. The hover readout names what the next click will take.

## Acceptance criteria

- [ ] A test clicks one stockpile cell with a floor and an item five times and gets item, floor, zone, then item again
- [ ] A stockpile on a floor can be selected and inspected from the map
