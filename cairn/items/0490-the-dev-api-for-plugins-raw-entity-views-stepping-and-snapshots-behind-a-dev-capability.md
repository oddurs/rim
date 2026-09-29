---
id: 490
uid: 6ae1513a-d8d0-4744-90e5-386d6d1d1921
title: 'The dev API for plugins: raw entity views, stepping and snapshots, behind a dev capability'
type: feature
status: backlog
milestone: workbench
depends_on:
- 526
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: engine
area: modding
---

## Problem

The workbench's tools are a plugin (`mods/devtools`, DESIGN.md §11a), so everything they show and do has to come through an API a mod can call. Today a mod's UI sees curated `view` fields and can only change the sim through its own `act.send` handler.

## Proposal

- A `dev` capability a mod declares in its manifest; the API exists only when the game runs with `--dev` and the mod declares it (§11a).
- Read: `dev.entity(id)` (raw components serialized, job and step, reservations, path), `dev.find(query)`, `dev.snapshots()` (this session's), `dev.tick()`.
- Act: `dev.send(DevCommand)` for dev commands, `dev.step(n)`, `dev.run_until(fn, max)`, `dev.branch(snapshot)`, `dev.pause()`.
- Types in `types/ui.d.luau` and a section in `docs/modding/api-ui.md`, generated like the rest.

## Acceptance criteria

- [ ] A mod without the capability, or a game without `--dev`, gets no `dev` table (test)
- [ ] Each call has a test through a small fixture mod
- [ ] The generated types and docs include the API and CI checks they're current
