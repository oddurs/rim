---
id: 93071dfd-02c7-4942-9a07-55fac10276cf
title: Compact mode and UI scale
type: feature
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: additive
effort: s
layer: client
area: ui
---

## Why

The same layout has to work from 1280×720 to 4K. Below about 1440 px the columns crowd the map; at 4K everything is tiny.

## What

- `view.compact()` is true below a width threshold; core components pick denser layouts from it.
- A UI scale setting (bindings to step it), saved with the other settings, applied as one factor in layout and hit testing.

## Acceptance criteria

- [x] At 1280×720 the compact layout keeps the map at least half the screen
- [x] UI scale steps with a key and survives a restart
- [x] Hit testing matches drawing at every scale

## 2026-09-26

view.compact is logical width < 1440 (UI scale included); view.screen now returns logical pixels as its doc said (it returned physical; nothing read it). UI scale 0.75-2 via Ctrl+=/-/0, saved as ui_scale in settings.toml. 1280x720 with 10 colonists, a selection and news: panels cover 26%.
