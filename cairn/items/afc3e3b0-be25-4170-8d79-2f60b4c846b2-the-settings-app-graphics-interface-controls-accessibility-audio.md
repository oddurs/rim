---
id: afc3e3b0-be25-4170-8d79-2f60b4c846b2
title: 'The Settings app: Graphics, Interface, Controls, Accessibility, Audio'
type: feature
status: backlog
milestone: rimos
depends_on:
- 2adfa3ac-b19d-41b3-bbfb-eeb18b7f3191
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Problem

Every setting is a palette command or a file key (settings.rs). There is no settings screen, and the title screen ignores setting actions.

## Proposal

One app with a page per area and per mod (`ui.settings_page`), the same on the title and in a colony:
- Graphics: lighting presets, render scale, window mode, vsync.
- Interface: UI scale, reset window positions, toasts.
- Controls: scroll, edge pan.
- Accessibility: reduce motion, text size, colour sets.
- Audio: its slots reserved until rim has sound.

Changes apply at once and write settings.toml as today. The Flat lighting preset has a slot, pending its decision. Overlaps 05033e73 (Settings in 1.0); the user decides whether that item moves here.

## Acceptance criteria

- [ ] Every key in settings.toml is reachable from the app (autotest walks the pages)
- [ ] Changing a setting on the title takes effect (title.rs no longer drops them)
