---
id: bfaa0225-9667-4d6f-934f-ab767b13937c
title: 'Two dead fields go: Mark::Offscreen''s entity and ClientView.mouse'
type: chore
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: s
layer: client
area: ui
---

## Problem

Two fields are written and never read: Mark::Offscreen.of in overlay.rs (its only match ignores it) and ClientView.mouse in rim_ui's view.rs (nothing in rim_ui reads it, and it isn't exposed to Luau). Found by BARE METAL's dead-code audit.

## Acceptance criteria

- [x] Both fields are gone, rim_ui's tests keep their pointer in their own state, and scripts/task check and the autotest pass
