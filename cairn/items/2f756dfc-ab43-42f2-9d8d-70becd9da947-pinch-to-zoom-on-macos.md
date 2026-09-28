---
id: 2f756dfc-ab43-42f2-9d8d-70becd9da947
title: Pinch to zoom on macOS
type: feature
status: done
milestone: pointer
assignee: Oddur Sigurdsson
depends_on:
- fda56c8e-7da2-4f92-872f-642647093f13
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A pinch on a Mac trackpad zooms around the fingers
- [x] Nothing changes on other platforms

## 2026-09-26

magnifyWithEvent: is added to miniquad's RenderViewClass at startup through the Objective-C runtime (class_addMethod), summing magnification into an atomic the frame takes. Criterion 1 (a real pinch zooms around the fingers) needs a trackpad: the zoom path is autotested with an injected pinch, the handler's install is checked at startup (it warns when it fails), and the gesture itself is for a person to try.

## 2026-09-27

Confirmed by hand on a Mac trackpad: a pinch zooms around the fingers.
