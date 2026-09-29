---
id: 470f2bd5-3f25-4ef4-b127-19e16e0e214f
title: '`[[start]]` items resolve only at new game, and a second start silently wins'
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: modding
---

## What

- modloader.rs ~309: `"start" => defs.start = Some(de!(v)?)`. A second `[[start]]` replaces the first by load order, with no warning (DESIGN.md ~3245 already notes this).
- `start.items` aren't resolved in `finalize` (defs.rs ~3153 resolves only `creature`). They're resolved in `Sim::make` (sim.rs ~92), so a bad item id passes the load check and fails only when a new game starts. Tools that only load mods (mod checks, the crosscheck of a loaded save) never see it.

## Reproduce

Not yet run; verified by reading. A mod whose start lists `thing = "nope"`: `modloader::load` succeeds, and `Sim::new` fails.

## Fix

Resolve start items in `finalize`. Make a second start an error that says to patch `start/...` instead, as a second `[[sky]]` is.

## Acceptance

- [ ] Both are load errors
- [ ] A test that fails before the fix and passes after
