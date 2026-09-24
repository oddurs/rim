---
id: 4504c561-95b3-4d62-8691-3a129260a3c9
title: 'Mod loader: discovery, API version check, deterministic load order'
type: feature
status: done
milestone: foundations
depends_on:
- e15682e1-9a6f-4627-8a59-cea52abcf30a
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
- determinism
---

## Why

Load order must be identical on every machine, and incompatible mods must fail loudly.

## Acceptance criteria

- [x] mod.toml declares id, version, api, depends, load_after
- [x] Topological sort with ties broken by id
- [x] Missing dependency and cycles are reported by name
- [x] 0.x API: minor versions are breaking
