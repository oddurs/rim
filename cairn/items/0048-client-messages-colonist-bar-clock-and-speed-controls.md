---
id: 48
title: 'Client: messages, colonist bar, clock and speed controls'
type: feature
status: done
milestone: castaway
depends_on:
- 45
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

The player needs to know what happened and when.

## Acceptance criteria

- [x] Pause and 1x/3x/6x
- [x] Day and hour
- [x] Colored message feed

## 2026-09-23

Verified by rim --autotest (crates/rim_client/src/autotest.rs), which drives the real client through the same Actions keyboard and mouse produce, checks state and saves screenshots; 56/56 checks pass, screenshots reviewed by eye. Runs in CI on macOS. Pause/resume and 1x/3x/6x; top bar day, HH:MM and speed; four message kinds in distinct colours; colonist bar click selects; Tab cycles.
