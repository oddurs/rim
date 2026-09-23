---
id: 127
title: Mod-list handshake
type: feature
status: backlog
milestone: co-op
depends_on:
- 152
created: 2026-09-22
updated: 2026-09-23
priority: p0
api: none
effort: s
layer: engine
area: net
pillar:
- plugin-first
- determinism
---

## Why

Peers must run identical mods.

## Acceptance criteria

- [ ] Compare modlist lockfiles (0152): ids, versions, content hashes and conflict choices
- [ ] Offer to install the host's exact modlist
