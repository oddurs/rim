---
id: 2cdc7e9d-ca2d-4bbc-98f0-c4c389d83dbd
title: A load checks worn garments as it checks the held tool
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: s
layer: engine
area: save
api: none
---

## Problem

A load runs a consistency pass for held tools (crates/rim_sim/src/snapshot.rs:789-809), but none for `worn`: a garment whose `Worn{by}` and the pawn's `worn` list disagree survives the load.

## Acceptance criteria

- [ ] A test loads a save whose worn list and `Worn` components disagree, and gets one consistent state with a load note
