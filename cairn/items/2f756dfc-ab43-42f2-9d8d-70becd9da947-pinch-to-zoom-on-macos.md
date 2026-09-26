---
id: 2f756dfc-ab43-42f2-9d8d-70becd9da947
title: Pinch to zoom on macOS
type: feature
status: backlog
milestone: pointer
depends_on:
- fda56c8e-7da2-4f92-872f-642647093f13
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Why

The window layer (miniquad 0.4) never registers macOS's magnify gesture, so a pinch never reaches the game.

## What

- Receive `magnifyWithEvent:` on the game's view and zoom by its magnification around the point between the fingers, continuously.
- Kept to the macOS client, with Cmd+scroll as the zoom where pinch isn't available.

## Acceptance criteria

- [ ] A pinch on a Mac trackpad zooms around the fingers
- [ ] Nothing changes on other platforms
