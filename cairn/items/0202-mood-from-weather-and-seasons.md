---
id: f690b16e-d379-4576-b497-01d66c9e17f8
title: Mood from weather and seasons
type: feature
status: backlog
milestone: mood
depends_on:
- eb3b0ac7-9922-4ab2-b9c1-8171e0b7f05f
- 7c50b502-5e27-4807-a36c-0654fe9aec97
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
