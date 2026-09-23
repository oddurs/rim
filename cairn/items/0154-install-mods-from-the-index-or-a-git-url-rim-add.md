---
id: 154
title: 'Install mods from the index or a git URL: rim add'
type: feature
status: backlog
milestone: platform
depends_on:
- 152
- 153
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
