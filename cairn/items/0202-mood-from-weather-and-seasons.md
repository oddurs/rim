---
id: 202
title: Mood from weather and seasons
type: feature
status: backlog
milestone: mood
depends_on:
- 182
- 184
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: s
layer: plugin
area: needs
pillar:
- plugin-first
---

## Why

A week of rain should get people down and the first sunny spring day should lift them. It's a good early test that `rim.mood` can read the weather through the public API only.

## What

- Mood thoughts in `rim.mood`: soaked, gloomy week, beautiful day, first snow, long winter.
- Uses `rim.weather()`, `rim.season()` and fields; no engine changes.

## Acceptance criteria

- [ ] Thoughts driven only by the scripting API
- [ ] Balance: weather thoughts are flavour, never a spiral on their own
