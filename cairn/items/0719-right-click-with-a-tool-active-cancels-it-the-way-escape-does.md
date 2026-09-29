---
id: 719
uid: e4517995-cfa3-4a79-9262-0b0d28742a1e
title: Right-click with a tool active cancels it the way Escape does
type: bug
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: s
layer: client
area: ui
api: none
---

## Problem

Right-click with a tool active drops to Select (crates/rim_client/src/main.rs:1751-1756), but it doesn't clear the tray's unfold state or card the way Escape does (mods/core/ui/toolbar.luau:332-352), so the tray comes back. Cancelling with a right-click is the RimWorld habit and the user expects it.

## Acceptance criteria

- [ ] A test: with a build tool active, a right-click leaves Select with the tray folded and no card, the same state Escape leaves
