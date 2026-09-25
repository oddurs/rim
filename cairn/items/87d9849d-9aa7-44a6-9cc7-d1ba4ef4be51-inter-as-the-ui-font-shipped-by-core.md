---
id: 87d9849d-9aa7-44a6-9cc7-d1ba4ef4be51
title: Inter as the UI font, shipped by core
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
effort: s
layer: core
area: ui
---

## Why

The UI took the system font, and cosmic-text draws SF badly: it applies
the weight axis but never SF's optical size, so every size gets the
Display design, about 10% narrower and thinner than macOS draws 13 pt
text (measured: 626 px against Core Text's 693 on the same line). Linux
got DejaVu and Windows Segoe, so layout and screenshots differed by
machine, CI's included.

## What

- Core ships Inter (SIL OFL 1.1), Regular and SemiBold, under
  `mods/core/ui/fonts/`, and the theme names it.
- rim_ui loads the fonts every mod ships under `ui/fonts/`, so a mod can
  bring its own and a theme can name it.
- A family the theme names that isn't there still falls back to the
  system UI font, and says so.

## Acceptance criteria

- [x] The UI draws in Inter on every platform, CI included
- [x] A mod's font under ui/fonts can be named by a theme

## 2026-09-24

Measured before switching, one line at 13 pt on a 2x screen: Core Text 693 px, rim with SF 626 px (cosmic-text applies wght but never opsz, so every size got SF's Display cut, default opsz 28), rim with a fontTools opsz-17 instance 701 px. Fixing SF would mean patching cosmic-text and would only help macOS; Inter's static cuts render right as they are and measure the same on every platform. Regular and SemiBold, the two weights the theme uses, 832 KB.
