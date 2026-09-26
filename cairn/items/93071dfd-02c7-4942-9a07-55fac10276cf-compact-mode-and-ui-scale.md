---
id: 93071dfd-02c7-4942-9a07-55fac10276cf
title: Compact mode and UI scale
type: feature
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
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

- [ ] At 1280×720 the compact layout keeps the map at least half the screen
- [ ] UI scale steps with a key and survives a restart
- [ ] Hit testing matches drawing at every scale
