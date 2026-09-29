---
id: 614
uid: 48f96f56-f4f5-4a0f-b5be-a5234f6c40d1
title: Keybind editor in Settings
type: feature
status: backlog
milestone: rimos
depends_on:
- 669
created: 2026-09-28
updated: 2026-09-29
priority: p2
api: additive
effort: m
layer: core
area: ui
---

## Problem

The engine can rebind (`Ui::rebind`) and saves keybinds.toml, but only tests call it; there's no UI and no Luau API.

## Proposal

- `ui.rebind(id, key)` and `ui.bindings()` for Luau.
- The Controls page lists every binding, searchable. Rebind by pressing a key; a conflict is named, and there's a reset per key.

## Acceptance criteria

- [ ] Rebinding Work to another key survives a restart (test)
- [ ] A conflict names the other binding and asks which keeps the key

## 2026-09-29

PAUSED: not started; calm-forest's when work resumes. Seam agreed with green-forest (DESIGN §11b, docs/modding/ui.md): kind="screen" windows; title-layer acts new_colony/load/continue/delete_save/open/quit and views saves/starts/settings/loading. The engine delivery is green-forest's; the client side (main.rs, title.rs, play.rs) is calm-forest's. Waits on green-forest's title-layer delivery and leave-to-title (play.rs). Rule from the user: no per-frame cost when idle, a fast first frame, settings only in rare cases (#388 trimmed afc3e3b0).
