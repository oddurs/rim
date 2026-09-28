---
id: 5a69f9c9-b971-496f-bb3b-e3e1838e2f02
title: 'Moving lights: carried and burning, capped, shadowless past the cap'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 9bd9e8ab-6eef-44dc-b814-0b376ceec1e5
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

A colonist carrying a torch at night, a burning roof, fire arrows: these move or change every frame and can't be baked. They get the same shadows as static lights, within a budget. DESIGN.md §6e.

## What

- Any light the renderer sees change position or appear/disappear often is dynamic: drawn each frame into its own target with the same march, at the preset's rays and step.
- A cap per preset (4 / 8 / 16 / 32), nearest the view centre first. Past the cap a light still glows, without shadows.
- Fire spreading (9bd9e8ab) makes burning cells dynamic lights until they settle, so a spreading fire never rebakes the static target every frame.

## Acceptance criteria

- [x] 64 moving lights at `medium`: 8 cast shadows, all glow, frame inside budget (bench)
- [x] A burning building never triggers more than one static rebake a second (test)

## 2026-09-26

Blocked until something moves while emitting light. Core has no carried light, and burning (9bd9e8ab) is blocked itself, so a dynamic-light path now would have no caller. Everything that emits light today (campfire, stove) is a fixture, which the static bake covers. Unblock when fire spreads or a carried light lands.

## 2026-09-27

People (5d09b04e): a carried torch is a held item on the body's hold socket (6f8be218). That makes it the moving light's first caller, and a reason to unblock this.

## 2026-09-28

Done as: fire (9bd9e8ab) is the first caller: its flames come and go as the fire spreads and colonists beat them out. The static bake now runs at most once a second (SETTLE). Lights it hasn't caught up with are drawn every frame into a moving-light target with the same march, and so is anything given through Light::set_moving. The preset's cap of them cast shadows, nearest the view centre first (low 4, medium 8, high 16, ultra 32, overridable as lighting.moving_shadows); the rest glow without, their strength negated in the vertex so the bake shader skips the trace. Changes to walls and lights that wait for a bake add up and are redone together. A light that goes lingers in the bake for at most a second. Autotest: 40 flames placed a frame apart over 1.4 s baked once, every new flame glowed the frame it appeared (dimmest 0.40), none was left moving once the bake caught up, and 64 moving lights drew 8 with shadows and all 64 glowing. Firelight checks that read the bake now wait for it to settle. Not done here: carried lights. Nothing carries a light yet; Hands (6f8be218) will call set_moving with a torch's position each frame. The bench gains a 'moving' view of 64 lights circling the colony at dusk; criterion 1's frame time comes from CI's bench.

## 2026-09-28

Review fixes: waiting static lights always get their shadows and don't count against the cap, which now shares out among moving lights only, nearest first (pure cap_shadows, unit-tested for order). The every-frame filter runs only while a bake is owed. The bake fetches its materials before it clears what it owes. light_settles waits by the clock and fails a check if the bake hasn't caught up in 3 s. DESIGN says what waits: a change after a quiet second bakes at once, and one within a second of the last waits for the next. A light that goes, and a wall that changes, wait in the bake for a second at most.

## 2026-09-28

Merged in #284. Criterion 1 (bench) is still open: CI's bench runs after the autotest, and main's autotest panics in the camera section (with lucky-harbor), so no bench has run on the moving view yet. Local timing at load 80+ proves nothing. I'll close this once a bench runs.

## 2026-09-28

Bench, from main's CI on e75589b9 (Linux llvmpipe, 100 frames): the 'moving' view (64 moving lights circling the colony at dusk, medium: 8 with shadows) draws the world in 3.586 ms mean, inside the 4 ms budget, and the render check passed ('within budget (moving: 3.586 <= 6.0 ms)'). The moving pass is 2.90 ms of that CPU. llvmpipe rasterises on the CPU, so that's the software renderer's cost; its GPU timer query reads 0.33 ms.
