---
id: e676f1c1-58aa-4cdd-b5f2-13d48793f444
title: A loading line at launch, and mod failures reach the title
type: feature
status: backlog
milestone: rimos
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Problem

Launch shows nothing until the title, and a failed start draws a raw macroquad error screen outside the UI.

## Proposal

- A loading line ("Loading 5 mods…", "Building the atlas…") with a thin bar.
- A mod that fails to load opens the title with a notification naming it, and a way to Mods.

## Acceptance criteria

- [ ] A broken mod no longer stops the game from reaching the title (test with a scratch mod)
