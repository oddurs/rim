---
id: fcb28d0c-35a3-4fca-977c-297d30a8e575
title: 'Refresh tiers: a node says how often it may change'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] A `fast` node changes every frame while the shell's layout is reused
- [x] A size change still relays out
- [x] Budget test with a live panel: median frame under 1 ms

## 2026-09-24

Tiers are per mount (ui.mount opts.refresh = frame | fast | slow), not per node: a component function is the unit that is rebuilt, so that is where a cadence can mean anything. Each mount keeps its own last-built time; input, a client change or a handler still rebuild everything, and windows follow the fast cadence. The layout cache now hashes text by its measured size (or nothing, when the leaf has a fixed w and h), so a readout that changes content at the same width leaves the shell's layout alone; wrapped text still hashes its content because its height depends on the width it gets. The budget harness takes the expected rebuild count, since a frame-tier panel rebuilds at 60 Hz by design.
