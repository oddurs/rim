---
id: 623
uid: 5c9e649d-13df-4388-bb58-7c3f94cf90d8
title: Named saves, a quick save, and loading from inside a colony
type: feature
status: backlog
milestone: rimos
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: additive
effort: m
layer: client
area: save
---

## Problem

Saving is automatic only; loading happens only from the title or the command line.

## Proposal

- Save in the system menu makes a named save beside the autosaves, and Ctrl S makes a quick one; a toast confirms both.
- Colonies in a colony loads, saving the running colony first.
- Delete asks through the confirm dialog.

## Acceptance criteria

- [ ] A named save loads back to the same state (the save tests' hash)
- [ ] Loading from a colony saves it first, and the title lists both

## 2026-09-29

PAUSED: not started; calm-forest's when work resumes. Seam agreed with green-forest (DESIGN §11b, docs/modding/ui.md): kind="screen" windows; title-layer acts new_colony/load/continue/delete_save/open/quit and views saves/starts/settings/loading. The engine delivery is green-forest's; the client side (main.rs, title.rs, play.rs) is calm-forest's. Waits on green-forest's title-layer delivery and leave-to-title (play.rs). Rule from the user: no per-frame cost when idle, a fast first frame, settings only in rare cases (#388 trimmed afc3e3b0).
