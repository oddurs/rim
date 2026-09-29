---
id: 127
uid: c31738fc-5861-43ef-b579-6f7f523b96bf
title: Mod-list handshake
type: feature
status: backlog
milestone: co-op
depends_on:
- 152
created: 2026-09-22
updated: 2026-09-27
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

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): the handshake compares the colony only: sim-side mods by their sim hashes (173de74c), colony options and sim picks. Client-side mods (e4b96647) are each player's own, as DESIGN.md §11 promises. Factorio requires every mod to match, so players install a friend's minimap to join; rim shouldn't.
