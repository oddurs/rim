---
id: 349e1b52-2784-4e98-98e3-73fb36f6fad3
title: A lightning flash lights evenly on its first frame
type: bug
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- ff818bb3-7175-48f0-a3c3-c346c9bc2469
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: s
layer: client
area: render
---

## What happens

A flash's shadows (ff818bb3) take the sun pass in `Light::prepare`, which runs before the world is drawn. The flash itself starts in `Sky::weather`, which runs after the world. So on the frame a flash strikes, prepare hasn't seen it: the multiply lights everything evenly, full brightness and no direction, and the bolt's shadows start a frame late. Found in review of ff818bb3.

## What should happen

The first frame of a flash is lit from where the bolt is, like the rest of it.

## Reproduction

Seed: 7
Mods: core
Tick: any, in a storm (heavy precipitation, wind over 10 m/s)

1. `Sky::strike()`, or wait for a natural flash.
2. On the next frame, `Light::lit_by_flash()` is false while `Sky::flash().strength` is 0.9.

## Acceptance criteria

- [x] On the first frame after a strike, the sun target holds the bolt's shadows (autotest)

## 2026-09-27

Fixed: a flash's timing moved into Sky::update, called before Light::prepare; weather only draws. strike() asks for a flash on the next update, so a tool's flash takes a storm's path. Autotest: one frame after a strike the sun target holds the bolt's shadows. With update moved back after prepare, that check fails (verified: FAIL at night a flash lights the ground from where the bolt is).
