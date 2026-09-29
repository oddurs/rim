---
id: 542
uid: e4b96647-cb31-45ed-a3f3-ff22412465bb
title: 'Mod sides: a mod with only ui/ is the player''s, not the colony''s'
type: feature
status: backlog
milestone: platform
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
- determinism
---

## Why

DESIGN.md §11 promises each co-op player can run different UI mods, but
nothing tells a UI mod apart. The loader, the save's epoch and the planned
sets and lockfile treat every mod alike. So a theme mod starts a new epoch
in every save it touches (173de74c), and a co-op handshake would refuse a
player over a colour.

Fabric declares `environment`, Modrinth packs mark `env.client/server`, and
tModLoader splits ServerSide and ClientSide configs. Factorio requires every
mod to match in multiplayer, so players install a friend's minimap to join.
DESIGN.md §10, "Tension: does every mod belong to the colony?"

## What

- A mod's **side** is derived from its folders; there is no field to get
  wrong.
  - `defs/` or `scripts/` make it **sim-side**. Its `ui/` still loads in the
    client.
  - A mod with only `ui/` (and `tests/`) is **client-side**.
  - `sprites/` count as sim-side content, because defs name them.
  - All seven shipped mods are sim-side.
- `ModManifest` gains `side`, computed in `read_manifest`.
- The sim's loader skips client-side mods; the client's UI VM loads them.
- A sim-side mod's `depends` or `optional` may not name a client-side mod,
  because the sim can't rely on what a player may not have. That is a load
  error naming both. A client-side mod may depend on sim-side mods.
- Everything that identifies a colony reads sim-side mods only: the epoch
  (173de74c), sets (73751f4f), "Open as saved" (d137353c), seed codes
  (47ec1fa0), the lockfile's colony section (0152) and the co-op handshake
  (0127).
- `rim check` prints each mod's side.

## Acceptance criteria

- [ ] `side` is derived for all seven mods (sim) and a fixture UI-only mod (client) (test)
- [ ] A client-side mod loads into the UI VM and not into the sim (test)
- [ ] A sim-side mod naming a client-side mod in `depends` or `optional` is a load error naming both (test)
- [ ] `rim check` prints each mod's side
- [ ] docs/modding/ui.md says what makes a mod client-side and what that promises
