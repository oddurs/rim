---
id: 178
title: Monochrome icon set with mod-supplied icons
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 168
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: additive
effort: m
layer: core
area: ui
---

## Why

Toolbars and tabs read faster with icons, and mods need to add their own.

## What

A small line-icon set as SVG, rasterised at load and tinted by tokens. Mods add icons by id.

## Acceptance criteria

- [ ] Core ships icons for every toolbar action
- [ ] Mods add icons by id; missing icons fall back to a label
