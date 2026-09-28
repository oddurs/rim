---
id: f2a8ffc7-9aa8-46cd-9c78-97c3a28001bd
title: 'Underground: rock is a roof, the cellar keeps the year''s mean, and it''s dark'
type: content
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- e311c029-499c-4764-a4d6-6d1f933f00f9
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: additive
effort: s
layer: core
area: needs
pillar:
- survival
---

## Why

A reason to dig in the first week that isn't ore: the cellar is the warmest place in winter and the coolest in summer (DESIGN.md §6d).

## What

- Every underground room is roofed through §6c's roof span, since solid terrain is a support. No underground special case.
- A field def may give `below = [terms]` for underground ambient. Core: −1 follows the year's mean with a damped seasonal swing; deeper is steady and a little warmer per level.
- Light below ground is zero. Torches and fires are emitters, as indoors today.

## Acceptance criteria

- [x] A cellar at −1 sits within 3°C of the year's mean through a simulated year (weather plugin on)
- [x] A large dug hall at −2 counts as sheltered
- [x] Balance: founder's hours at zero warmth in the first winter with a cellar, recorded here

## 2026-09-26

Houses' roof span (map.rs spread_cover / cover_window) walks the surface plane only (IVec::new). Below the surface cover stays 0, so an underground room counts every cell as uncovered until this item spreads cover per level. Solid rock's span comes from its thing's support block (granite).

## 2026-09-28

Built: field `below` terms (with a `depth` input) give a field's outdoor value under the surface, 0 without them (light, wind, rain); an underground room leaks toward its own level; comfort-seeking floods through stairs. Core: 10°C at -1, +1°C a level. Weather: -1 is the year's mean (8.2°C) with a fifth of the swing an eighth of a year late, 5.4 to 10.4°C, checked over a sampled year (surface -6 to 19). The cellar criterion is met by sampling the terms across the year plus a live cellar reaching the ground's temperature within a day, not a simulated year (a year of sim is 1.2 M ticks). Balance (examples/balance, 20 seeds, 60 days from core's spring start, 3 colonists with tools, --cellar 2 digs stairs and a 5x5 cellar at -1): no colony in either arm reaches winter; they die to wildlife and raids by autumn. Over the run, without/with the cellar: frozen colonist-hours a run 116.9 / 62.2, founder hours at zero warmth after night one 8.8 / 3.7 (14 / 10 runs), deaths by freezing 49 / 29; the cellar was dug by day 2.2 and colonists spent 173 hours a run below. From a late-autumn start (day 40, 20 days) all colonies freeze either way: 264 / 247 frozen colonist-hours; the cellar holds 8.8°C there against -3 on the surface, but under the warmth need's 10°C comfort, so it slows freezing and doesn't stop it. Also fixed on the way: --start-day's patch target lacked its prefix and no longer loaded.
