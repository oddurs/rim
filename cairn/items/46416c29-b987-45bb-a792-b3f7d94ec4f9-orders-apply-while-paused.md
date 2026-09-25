---
id: 46416c29-b987-45bb-a792-b3f7d94ec4f9
title: Orders apply while paused
type: feature
status: doing
milestone: colony
assignee: Oddur Sigurdsson
claimed: 2026-09-24
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: none
effort: s
layer: engine
area: sim
---

## Why

Paused, nothing happens when the player designates, builds or orders: a
command applies at the start of the next tick, and a paused game has no
next tick. Planning while paused is how a colony sim is played.

## What

- `Sim::apply_pending` applies queued commands at the current tick
  boundary without advancing time: the same point `step` applies them, so
  the command log and replays are unchanged.
- The client calls it every frame, so designations, plans and orders show
  at once, paused or not; the UI reads the world as it is.

## Acceptance criteria

- [ ] Paused, a designation, a plan and an order show in the world and the UI
- [ ] Commands applied early give the same state hash as applied in `step`
