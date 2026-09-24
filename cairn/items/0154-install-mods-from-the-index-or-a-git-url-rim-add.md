---
id: 76a0bc45-13c1-4725-b5ce-47957109ac41
title: 'Install mods from the index or a git URL: rim add'
type: feature
status: backlog
milestone: platform
depends_on:
- 9e979a26-5dc0-4122-b62d-fc48f14d8488
- b7f5cde1-a8c4-4d7d-b93d-bdf0ba0d523b
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Players never need git; developers can point at any repo or branch.

## Acceptance criteria

- [ ] `rim add <id>` and `rim add github.com/<owner>/<repo>[@tag]` download release archives over HTTPS
- [ ] Resolves dependencies, verifies hashes, updates `mods.lock`
- [ ] `rim update` and `rim remove`
- [ ] The in-game mod manager uses the same code
