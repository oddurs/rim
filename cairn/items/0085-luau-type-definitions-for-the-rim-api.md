---
id: 6191b800-bc29-4793-a3dd-f8d73d91cbf5
title: Luau type definitions for the rim API
type: docs
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: additive
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Typed API makes modding approachable.

## Acceptance criteria

- [x] The API is declared once in Rust; rim.d.luau and the reference docs are generated from that declaration
- [x] Works with luau-lsp, including `require("@mod/...")` aliases
- [x] CI fails if the checked-in types drift from the host

## 2026-09-24

Declarations sit next to the registrations in script.rs: the api! macro takes the signature and doc, and plugin-free values (ticks_per_day, creature_defs, on, every, ...) use self.declare. tests/api_types.rs asserts the declared names equal what the engine registers, compiles the types to prove they parse, and compares types/rim.d.luau and docs/modding/api-scripts.md (RIM_UPDATE_TYPES=1 rewrites them). luau-analyze can't load definition files, so the editor and CI path is luau-lsp analyze --definitions:@rim=... (pinned 1.70.0). Nonstrict found one real mismatch in core (a string faction) plus implicit returns in incident execute(); fixed both, behaviour unchanged since the storyteller checks ~= false. Strict mode is noisy on untyped locals, so it's opt-in. Definition-file type aliases aren't visible to scripts. UI globals are split into a follow-up item.
