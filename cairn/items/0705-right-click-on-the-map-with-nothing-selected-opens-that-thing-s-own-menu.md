---
id: 705
uid: 04b4de19-1a14-44f0-823b-05324623d1fd
title: Right-click on the map with nothing selected opens that thing's own menu
type: feature
status: backlog
milestone: rimos
depends_on:
- 711
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: m
layer: client
area: ui
api: none
---

## Problem

With nothing selected, a right-click does nothing, and a hold opens a tile menu that is usually empty (crates/rim_ui/src/vm.rs:1213-1225). The colonist and zone menus exist (mods/core/ui/people.luau:36-70, zones.luau:31-60), but only from lists and the inspector, never from the map.

## Proposal

With nothing selected, right-click a colonist, a zone or a building on the map to get its own menu: the same one its list row gives. The thing chosen follows the click cycle's order (the next thing down the cell).

## Acceptance criteria

- [ ] With nothing selected, a right-click on a colonist, a stockpile and a workbench opens each one's menu (test)
