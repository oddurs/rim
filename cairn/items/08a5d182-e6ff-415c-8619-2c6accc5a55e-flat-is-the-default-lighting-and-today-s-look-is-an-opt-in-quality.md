---
id: 08a5d182-e6ff-415c-8619-2c6accc5a55e
title: Flat is the default lighting, and today's look is an opt-in quality
type: feature
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

The user chose option B in the BARE METAL plan (3c65738f): the lighting is slow on the Mac. Medium drops the bench's later views from 120 to about 60 fps, while flat holds 120 in every view. So the default is `flat`, and today's look is a quality the player opts into, knowing its cost.

## What

- `flat` is the default preset: an unset `[lighting]` and a new install run it. It is the sim's `light` field, tinted by the sky and firelight colours, in one multiply (DESIGN.md §6e).
- `low`, `medium`, `high` and `ultra` stay, as opt-in qualities. The settings file, the palette bindings and the docs label each with what it costs, and `auto` keeps its ladder.
- The render bench measures the default, and reruns each view under `medium` beside it, so both stay tracked on one runner.
- The autotest's lighting sections pin the preset they test.
- The sim is untouched: light stays the sim's field.

## Acceptance criteria

- [ ] With no settings file, the game and the bench run `flat` (test)
- [ ] medium, high and ultra are chosen in settings or the palette and labelled with their cost (test)
- [ ] Before and after on a same-runner-class CI A/B, and a 5-minute foreground Mac run of the default, with the user's OK
