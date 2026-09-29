---
id: 652
uid: 93807461-84d8-479e-8f83-eb543e0c92a6
title: Opt-in lighting composes the light once at light resolution, so the multiply reads one texel a pixel
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: render
---

## Why

For the opt-in qualities (medium and above) the multiply reads about 11 textures for every screen pixel at full resolution. The glFinish probe (3c65738f) puts it at 1.4 to 2.9 ms of GPU a frame on the Mac, the one lighting pass that runs every frame.

## What

- Compose the light (sky and body shares, sunlit, firelight, moving, rooms) once into a buffer at light resolution. The screen's multiply then reads one texel, bilinear, plus what the contact shadow needs.

## Acceptance criteria

- [ ] Medium's look unchanged in paired shots
- [ ] The multiply's GPU time falls, on a same-runner CI A/B and on the Mac probe
