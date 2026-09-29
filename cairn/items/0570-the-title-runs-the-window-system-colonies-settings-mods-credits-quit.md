---
id: 570
uid: 13a7b680-8a61-4379-9b70-fa6843a45464
title: 'The title runs the window system: Colonies, Settings, Mods, Credits, Quit'
type: feature
status: backlog
milestone: rimos
depends_on:
- 586
- 647
- 669
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: none
effort: m
layer: client
area: ui
---

## Problem

Before a colony, only title-layer mounts are built: no windows, no palette. The title has Continue, New colony and a saves list only, and title.rs drops every other action.

## Proposal

- Windows and the launcher work on the title.
- The title's buttons: Continue (primary), New colony, Colonies…, Settings, Mods, Credits, Quit.
- Colonies is an app with open, delete and open folder; Mods lists the installed mods with their api and errors; Credits is a plain app.

## Acceptance criteria

- [ ] The launcher opens on the title (test)
- [ ] Every title button reaches its app (autotest from the title)

## 2026-09-29

PAUSED: not started; calm-forest's when work resumes. Seam agreed with green-forest (DESIGN §11b, docs/modding/ui.md): kind="screen" windows; title-layer acts new_colony/load/continue/delete_save/open/quit and views saves/starts/settings/loading. The engine delivery is green-forest's; the client side (main.rs, title.rs, play.rs) is calm-forest's. Waits on green-forest's title-layer delivery and leave-to-title (play.rs). Rule from the user: no per-frame cost when idle, a fast first frame, settings only in rare cases (#388 trimmed afc3e3b0).
