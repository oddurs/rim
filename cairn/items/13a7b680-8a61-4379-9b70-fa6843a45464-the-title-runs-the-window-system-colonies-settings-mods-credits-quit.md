---
id: 13a7b680-8a61-4379-9b70-fa6843a45464
title: 'The title runs the window system: Colonies, Settings, Mods, Credits, Quit'
type: feature
status: backlog
milestone: rimos
depends_on:
- 2adfa3ac-b19d-41b3-bbfb-eeb18b7f3191
- 8cfce3bb-b22f-43ec-88f8-21bab9326787
- afc3e3b0-be25-4170-8d79-2f60b4c846b2
created: 2026-09-28
updated: 2026-09-28
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
