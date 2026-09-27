---
id: 46b7d000-066e-4644-9e87-a3a9097154a8
title: Profiler history, p99 and a spike trap
type: feature
status: backlog
milestone: workbench
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: perf
---

## Problem

F3 shows smoothed averages per system, so a single slow tick disappears into the mean and can't be caught. DESIGN.md §11a.

## Proposal

A history graph per system over the last 600 ticks with p50 and p99 against §8's budgets; a spike trap that pauses and snapshots the world when a tick exceeds a threshold, so it can be replayed under a native profiler; export as a Chrome trace.

## Acceptance criteria

- [ ] A deliberately slow tick is caught by the trap and its snapshot replays to the same slow tick (shown)
