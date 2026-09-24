---
id: 9e979a26-5dc0-4122-b62d-fc48f14d8488
title: 'Modlist lockfile: exact versions, content hashes and conflict choices'
type: feature
status: backlog
milestone: platform
depends_on:
- dbb8f031-f057-4c50-839a-e2330b2e5a78
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: engine
area: modding
pillar:
- determinism
- plugin-first
---

## Why

One file that says exactly what was loaded: the base of replays, bug reports, crash reports and the co-op handshake. A modpack is a lockfile someone shared (DESIGN.md §10).

## Acceptance criteria

- [ ] `mods.lock` records each mod's id, version, source and content hash
- [ ] Records the player's per-field winners for patch conflicts
- [ ] Loading with a lockfile reproduces the same def database byte-for-byte
- [ ] Replays and crash reports embed it
