---
id: fcb28d0c-35a3-4fca-977c-297d30a8e575
title: 'Refresh tiers: a node says how often it may change'
type: feature
status: backlog
milestone: colony
depends_on:
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
effort: m
layer: engine
area: ui
---


## Why

Every number a script reads refreshes at 4 Hz so one readout cannot reflow
the shell every frame. Live demand on the Work Board and needs bars want
faster; the top bar wants slower. One rule for everyone means rebuilding
at 20 Hz because one panel asked.

## What

- `refresh = "frame" | "fast" | "slow"` on a node (default: today's 4 Hz).
- A node whose content changed but whose measured size did not repaints
  without invalidating the layout cache of the tree around it.
- The budget test gains a live panel and must still pass.

## Acceptance criteria

- [ ] A `fast` node changes every frame while the shell's layout is reused
- [ ] A size change still relays out
- [ ] Budget test with a live panel: median frame under 1 ms
