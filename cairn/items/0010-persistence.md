---
id: c76c29d7-996e-488f-9fe0-ffd1e5a32a1c
key: persistence
title: Persistence
type: milestone
status: done
depends_on:
- d02fb66e-af87-4c3e-93c5-48b9bfc1fb2b
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: none
due: 2026-11-20
---

The log is the save and snapshots are a cache (DESIGN.md §7a): a colony is always saved, loads fast, survives mod changes, and replays exactly.

## 2026-09-24

Shipped 2026-09-25: #60 plan, #63 stable entity ids, #64 script data per mod, #65 autotest fix, #68 snapshots, #69 save file, #71 removed-mod data, #73 migrations, #76 replays, #78 always saved, #74 rim save, #75 title screen. On main at 2f5f86c: 254 tests pass, and the crosscheck example saves and reloads a twin every 5 days, which must match the unsaved game on every platform.
