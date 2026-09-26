---
id: dd73a536-0550-4805-bc99-2855139c4814
title: 'Now: an alert registry and a news feed'
type: feature
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Why

Messages are toasts that scroll away. Standing problems (nobody can cook, a colonist is starving, nothing is stored) have no home, and each mod that wants one draws its own label somewhere.

## What

- A core UI module `@core/ui/alerts`: `alerts.add{ id, severity, check = fn(view) -> text? , on_click? }`. Checks run at the slow refresh (4 Hz), never per frame.
- A right-edge "Now" column: active alerts first, by severity, then the news feed (the message log), newest first.
- Mods register alerts the same way core does; core's own alerts go through the registry too.
- Clicking an alert with a subject centres the camera on it.

## Acceptance criteria

- [ ] Alerts are registered, not hard-coded: core and a mod each add one through `alerts.add`
- [ ] Checks run at the slow refresh, and the budget stays under 1 ms
- [ ] News replaces the toast column; nothing is lost when many arrive
- [ ] docs/modding/ui.md shows how to add an alert
