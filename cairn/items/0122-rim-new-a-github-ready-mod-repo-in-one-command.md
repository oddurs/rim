---
id: 6d24a1ec-85be-4317-a62a-4d6f049f8f2d
title: 'rim new: a GitHub-ready mod repo in one command'
type: chore
status: backlog
milestone: sdk
depends_on:
- 6191b800-bc29-4793-a3dd-f8d73d91cbf5
- be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
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
