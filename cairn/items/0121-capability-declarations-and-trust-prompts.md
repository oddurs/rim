---
id: 5f7eb168-588c-46c3-b464-4e3a0e67170d
title: Capability declarations and trust prompts
type: feature
status: backlog
milestone: platform
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: additive
effort: m
layer: engine
area: modding
---

## Why

Players should know what a native or WASM mod can touch.

## Acceptance criteria

- [ ] Capabilities in mod.toml

Scope from DESIGN.md §10: data and Luau mods need no prompt because they can't do I/O. Capabilities apply only to the WASM tier. There is no native-code tier.
