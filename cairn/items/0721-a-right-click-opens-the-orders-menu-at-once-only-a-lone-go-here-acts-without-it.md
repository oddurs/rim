---
id: 721
uid: ed906bb5-1a35-45eb-bd2f-f3dce53d599f
title: A right-click opens the orders menu at once; only a lone "go here" acts without it
type: feature
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: s
layer: client
area: ui
api: none
---

## Problem

The user doesn't like holding the right button for the menu. Today, with a colonist selected, a quick right-click does the first safe order, and holding for 0.35 s opens the orders menu (crates/rim_client/src/main.rs:1728, 1776-1797). A dragged right-click pans the map (:1768-1774). A quick click acts without showing what it chose, and the menu hides behind a delay.

## Proposal

As in RimWorld, a right-click that isn't a drag opens the orders menu on release, with no hold. The safe first order is the highlighted row, so right-click then Enter, or a click on it, does it (mods/core/ui/menus.luau:131-148 already has a primary row). When the only order is "go here", the colonist walks without a menu. Right-drag still pans. With a tool active, right-click cancels the tool (e4517995). With nothing selected, it opens the thing's own menu (04b4de19).

This replaces the quick-click behaviour of 9aa55d96, the safe right-click.

## Acceptance criteria

- [ ] A test: with a colonist selected, a right-click over a tree opens the orders menu on release, with the safe order highlighted
- [ ] A right-click on open ground with only "go here" moves the colonist without a menu
- [ ] A right-drag pans and opens no menu
