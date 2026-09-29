---
id: 156467ce-e0a1-4776-b008-7b21cd6f5579
title: A colonist who walks off the map loses the stack it was carrying
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: s
layer: engine
area: sim
api: none
---

## Problem

Leaving the map despawns the held tool and worn garments (crates/rim_sim/src/systems.rs:186-191), but the carried lot is silently discarded. Nothing in the stock ledger or the news records it.

## Acceptance criteria

- [ ] A test: a colonist carrying a stack walks off the map, and the stack is dropped at the edge cell (or the ledger records it leaving)
