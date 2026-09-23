---
id: 122
title: 'rim new: a GitHub-ready mod repo in one command'
type: chore
status: backlog
milestone: sdk
depends_on:
- 85
- 149
created: 2026-09-22
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

Starting a mod takes one command, and the result is a repo ready to push to GitHub.

## Acceptance criteria

- [ ] Generates mod.toml (with SPDX license), defs, a script and a test
- [ ] Includes `.luaurc` and API types so luau-lsp works immediately
- [ ] Includes the GitHub Action workflow, README and .gitignore
- [ ] `rim check` and `rim test` pass on the fresh output
