---
id: 551
uid: f2a251f6-4518-4f3f-bd59-5c0c1d3cad2d
title: The mod crater publishes its results to the index, and the game marks known-broken mods
type: feature
status: backlog
milestone: platform
depends_on:
- 153
- 155
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

The mod crater (0155) finds which indexed mods a candidate engine breaks.
That knowledge should reach players before they start a colony, not after.
SMAPI keeps a central compatibility list and safely disables mods known to
be broken. DESIGN.md §10, "Tension: a moving API or mods that rot?"

## What

- After each run, the crater writes
  `compat/results/<mod id>.toml` in the index: per mod version, the engine
  versions tested, pass or fail, and the first engine version that broke it.
- The index snapshot the game fetches (0153) includes the results.
- New colony and the mod manager mark a mod known broken on this engine:
  "Wildlife+ 0.1.0: its tests fail on this engine (since 0.7)". It stays
  loadable; the player decides.

## Acceptance criteria

- [ ] The crater writes result files in the format above (test on a fixture index)
- [ ] The snapshot includes them, and the game reads them without network access when cached
- [ ] A known-broken mod is marked on New colony and in the mod manager, and still loads
