---
id: 451
uid: 1f9dbcaa-3d14-46ce-8375-0d0558550e5c
title: A run opened from a terminal pulls the person to another Space
type: bug
status: done
milestone: interface
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: ui
---

## What happens

## What should happen

## Reproduction

Seed:
Mods:
Tick:

1.

miniquad activates the app once it runs
(`activateWithOptions:` ignoring other apps), and macOS moves the person
to the Space the window opened in. Every autotest, render bench and game
started from a terminal pulled them out of what they were doing.

## Acceptance criteria

- [x] `--background` opens the window without activating the app
- [x] The autotest and the render bench always open that way
- [x] The autotest still passes with its window in the background

## 2026-09-27

Verified by polling the frontmost app (lsappinfo front) every 0.2 s through an autotest run: rim never came to the front, and the autotest passed 224 of 224 in 23 s with its window in the background.
