---
id: 215
title: Pick the material before you place it
type: feature
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-24
depends_on:
- 212
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: none
effort: s
layer: plugin
area: ui
---

## Why

Choosing a buildable is now two choices, and the second one has to be in
front of the player before they commit, not after.

## What

- The build tool gains a material row: what you have, what it costs, and
  what the result will be.
- The stats that differ are shown as they differ -- a stone wall's hp
  against a wooden one's -- rather than as raw numbers.
- Remembers the last material per buildable, because nobody wants to pick
  wood forty times.
- Lives in core's UI mod, on the public UI API.

## Acceptance criteria

- [ ] Material chosen before the blueprint is placed
- [ ] Materials you have none of are visibly unavailable, not hidden
- [ ] Built in core's UI mod, no client change
