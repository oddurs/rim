---
id: f2a8ffc7-9aa8-46cd-9c78-97c3a28001bd
title: 'Underground: rock is a roof, the cellar keeps the year''s mean, and it''s dark'
type: content
status: backlog
milestone: depth
depends_on:
- e311c029-499c-4764-a4d6-6d1f933f00f9
created: 2026-09-26
updated: 2026-09-26
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

- [ ] A cellar at −1 sits within 3°C of the year's mean through a simulated year (weather plugin on)
- [ ] A large dug hall at −2 counts as sheltered
- [ ] Balance: founder's hours at zero warmth in the first winter with a cellar, recorded here
