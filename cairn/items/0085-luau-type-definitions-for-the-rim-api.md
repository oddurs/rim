---
id: 6191b800-bc29-4793-a3dd-f8d73d91cbf5
title: Luau type definitions for the rim API
type: docs
status: backlog
milestone: plugin-api
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: additive
effort: s
layer: tooling
area: docs
pillar:
- plugin-first
---

## Why

Typed API makes modding approachable.

## Acceptance criteria

- [ ] The API is declared once in Rust; rim.d.luau and the reference docs are generated from that declaration
- [ ] Works with luau-lsp, including `require("@mod/...")` aliases
- [ ] CI fails if the checked-in types drift from the host
