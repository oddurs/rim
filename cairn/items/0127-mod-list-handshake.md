---
id: c31738fc-5861-43ef-b579-6f7f523b96bf
title: Mod-list handshake
type: feature
status: backlog
milestone: co-op
depends_on:
- 9e979a26-5dc0-4122-b62d-fc48f14d8488
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
