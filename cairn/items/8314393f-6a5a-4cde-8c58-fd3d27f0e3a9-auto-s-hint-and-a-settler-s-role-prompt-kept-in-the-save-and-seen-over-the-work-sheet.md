---
id: 8314393f-6a5a-4cde-8c58-fd3d27f0e3a9
title: 'Auto''s hint and a settler''s role prompt: kept in the save, and seen over the Work sheet'
type: feature
status: backlog
milestone: mood
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: s
layer: core
area: ui
---

## Why

Two gaps in 5dc8f858. The first-day hint's "once" lives in `ui.state`, so reloading a day-one save shows it again. And the settler dialog sits on the float layer, under windows, so a settler who joins while the Work sheet is open is asked where the player can't see.

## What

- Remember the hint's dismissal, and which settlers were asked, in the save through the UI mod's persistent state (or a script data key), not `ui.state`.
- While a sheet is open, the settler prompt shows as a line in the sheet's header, or waits until the sheet closes and then gets its full time.

## Acceptance criteria

- [ ] Dismiss the hint, save and load: it stays dismissed (test)
- [ ] A settler joining while Work is open is asked once Work closes, with the full time to answer (UI test)
