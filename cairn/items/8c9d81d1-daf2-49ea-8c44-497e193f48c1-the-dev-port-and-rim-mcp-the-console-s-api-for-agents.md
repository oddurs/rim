---
id: 8c9d81d1-daf2-49ea-8c44-497e193f48c1
title: 'The dev port and rim mcp: the console''s API for agents'
type: feature
status: backlog
milestone: workbench
depends_on:
- 6ae1513a-d8d0-4744-90e5-386d6d1d1921
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Problem

An agent that wants to see what a change does must run the whole autotest or drive a window. It can't ask the running game a question. DESIGN.md §11a.

## Proposal

`rim --dev-port <port>` (headless with `--headless`) serves JSON lines on localhost: `step`, `eval`, `dev`, `shot`, `save`, `load`. `rim mcp` exposes the same calls as an MCP server over stdio. Every call that changes the sim is a dev command.

## Acceptance criteria

- [ ] A test drives a headless game over the port: load a scene, step 600 ticks, eval a count, take a shot
- [ ] `rim mcp` lists its tools and answers one call (shown)
