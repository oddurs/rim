---
id: 67f5cfe1-8500-4053-87ef-370e3bf131db
title: 'HUD rhythm: a type scale with leading and named spacing roles'
type: feature
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Why

The HUD has a 4 px grid and five text sizes but no rules for which gap goes where. The news stack uses the tightest gaps on the grid (4 px inset, 2 px between messages, 1.3 leading on wrapped lines, a button 4 px from the edge), so two messages read as one paragraph. The tile readout uses the loosest padding around the smallest text (12 px around 12 px text, readings strung on three spaces). Section titles look like their contents. Proposal: the HUD Rhythm artifact.

## What

- Theme: spacing roles on the 4 px grid (hair 2, tight 4, item 8, inset 12, panel 8, group 16) beside the old names; a caption size (11); leading for one line (1.3) and for wrapped text (1.4), read by the engine instead of a hard-coded 1.3.
- The gap between stacked panels in a region is the `panel` token (8).
- Kit: `kit.panel` pads 8 × 12 by default; `kit.section(label, right?)` for panel headers; `kit.reading(name, value)` for a caption over a value.
- News: a caption header whose right side links to all news, a stripe per message kind, day and time under each message, 8 between messages.
- People: a caption header with the count on the right; cards per the proposal.
- Tile readout: terrain, coordinates and shelter on one line, readings as name over value.

## Acceptance criteria

- [x] Wrapped text uses the theme's leading_wrap; single lines use leading
- [x] kit.panel, kit.section and kit.reading exist and core's panels use them
- [x] News, people and the tile readout match the proposal in the autotest screenshots
- [x] Existing node ids keep working and the UI budget tests pass

## 2026-09-26

Built per the HUD Rhythm artifact. Leading and tracking are theme sections ([leading] line/wrap, [tracking] caption) the engine reads; cosmic-text 0.19 does letter spacing, so the caption tracking is real. Captions stay at 400 (only Regular and SemiBold ship). A news row needed minw = 0 on its text column or long messages never wrapped; tests/now.rs covers it. view.hover gained values {label, value}; view.messages gained clock.
