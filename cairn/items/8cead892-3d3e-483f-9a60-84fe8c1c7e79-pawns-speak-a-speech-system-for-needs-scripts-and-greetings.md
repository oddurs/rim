---
id: 8cead892-3d3e-483f-9a60-84fe8c1c7e79
title: 'Pawns speak: a speech system for needs, scripts and greetings'
type: feature
status: doing
milestone: interface
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: ui
pillar:
- plugin-first
---

## Why

Speech bubbles were a greeting and nothing else, hard-wired into core's labels. Pawns should say what matters (hungry, freezing, a raid's leader calling out), mods should be able to make them talk, and a crowd talking must stay readable and cheap.

## What

- `World::say` and a small speech log (64 lines, not saved, read by nothing in the sim).
- `rim.say(id, text, ticks?, priority?)` for sim scripts; a need def's `say = { below, lines, ticks }` speaks once per crossing, from the needs pass, the line picked by pawn and tick (no RNG).
- `view.speech()` for the UI; core's labels show each pawn's most important line and at most six bubbles, wrapping and fading.
- Core's food, rest and warmth speak; a raid's leader calls out.

## Acceptance criteria

- [x] A need speaks once as it crosses its level, not again while it stays low; animals don't
- [x] rim.say works from a sim script and cuts long lines
- [x] Speech changes nothing a save holds
- [x] A crowd of ten speakers shows six bubbles, the most important
- [x] A bubble is centred on its speaker the frame a zoom lands (autotest)
