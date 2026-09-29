---
id: 8610a466-5792-4b36-b15e-7989cb9e6f6a
title: The dock as a taskbar, and the launcher lists apps
type: feature
status: backlog
milestone: rimos
depends_on:
- 2adfa3ac-b19d-41b3-bbfb-eeb18b7f3191
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: core
area: ui
---

## Problem

Open screens show as top-bar tabs, the dock holds only tools, and a minimised app would have nowhere to go.

## Proposal

The dock gets a middle section of open apps (focused, open, minimised) and a right end with the launcher and the system menu. Clicking the focused app minimises it, and clicking a minimised one restores it. Right-click gives the window menu. The palette lists apps first as "Open …".

## Acceptance criteria

- [ ] Autotest: open Work, minimise it, restore it from the dock; the launcher opens Stores by name
- [ ] The dock's height doesn't change (steady-HUD test)
