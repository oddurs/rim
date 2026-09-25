---
id: af316e9d-645f-4f7b-935b-9c9a2b1e8ba7
title: 'Main menu: continue, load a colony, or start a new one'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- 01e86e7b-17ac-4e5a-8920-d72a438037f1
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: additive
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

The game starts straight into a new colony. With saves (DESIGN.md §7a) a
player needs to come back to one: continue the last, pick another, or start
fresh. Split from 01e86e7b, which makes every game save itself; this is the
screen in front of it.

## What

A screen before any world exists: Continue (the newest save), a list of
colonies (name, day, colonists, last played) to load, and New colony. Until
it lands, `rim --continue` and `rim --load FILE` do the same from the
command line.

## Acceptance criteria

- [x] Continue loads the newest save
- [x] The list shows every save with its colony's day and colonists, newest first
- [x] New colony starts a game with a new save
- [x] A save that fails to load says why and doesn't crash the menu

## 2026-09-24

The menu is the UI's new 'title' layer, drawn by mods/core/ui/title.luau: before a colony the UI builds only that layer, over an empty World built from the real defs, and in a game never builds it, so no HUD script changed. view.saves() and act.load/act.new_colony are the surface; Continue is the script loading saves[1]. UI_API_VERSION stays 0.1, following #53, which added members without a bump. A load runs on the UI thread: a long replay freezes the title screen until it's done. Tested: the title component (rim_ui tests/title.rs), the listing (save::list_in), the summary (rim_sim savefile). The window loop itself was only run, not clicked: screen capture isn't permitted here.

## 2026-09-24

UI_API_VERSION is now 0.2, and every shipped mod's ui_api with it: pre-1.0 every minor is breaking, so a mod using view.saves or the title layer is refused by an engine that lacks them.
