---
id: cec3efdf-dc49-4fc0-a35e-5d649efe90e6
title: 'Dev commands: Command::Dev, a dev-touched epoch, and the commit as engine version'
type: feature
status: backlog
milestone: workbench
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: engine
area: sim
---

## Problem

There is no way to change the sim from inside the game except through a mod's `act.send` handler, and a direct side door would break replays (DESIGN.md §7). Epochs record `CARGO_PKG_VERSION`, always 0.1.0, so a replay across commits reports a divergence instead of a version change. DESIGN.md §11a.

## Proposal

- `Command::Dev(DevCommand)` with `Spawn`, `SetStat`, `Heal`, `Kill`, `Teleport`, `FireIncident`, `ForceWeather`, `FinishBuild`; applied at a tick boundary like any command, logged in the save.
- The first dev command in an epoch marks it `dev_touched`; `rim save unpack` shows it.
- The epoch's engine version is the git commit (from the build), falling back to the package version.
- `dev.*` in Luau (for the console) and the dev port send these commands.

## Acceptance criteria

- [ ] A game with dev commands replays exactly (`rim replay` passes) and its epoch says `dev_touched` (test)
- [ ] A replay under a different commit reports a version change, not a divergence (test)
- [ ] Each command has a test that applies it and checks the result
- [ ] Determinism test passes
