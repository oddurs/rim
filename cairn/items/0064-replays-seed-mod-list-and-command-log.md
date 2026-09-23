---
id: 64
title: 'Replays: seed, mod list and command log'
type: feature
status: backlog
milestone: persistence
depends_on:
- 32
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- determinism
---

## Why

Bug reports become exact reproductions.

## Acceptance criteria

- [ ] Replay reaches the same state hash at every checkpoint

Once 0152 lands, the mod list in a replay is the modlist lockfile, so a replay pins exact mod versions and hashes.
