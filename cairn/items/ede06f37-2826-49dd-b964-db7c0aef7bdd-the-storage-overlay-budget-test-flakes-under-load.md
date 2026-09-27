---
id: ede06f37-2826-49dd-b964-db7c0aef7bdd
title: The storage overlay budget test flakes under load
type: bug
status: review
milestone: crafting
assignee: Oddur Sigurdsson
claimed: 2026-09-27
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: tests
---

## Why

`draw::tests::the_storage_overlay_over_200_stores_is_cheap` (#196) asserts the mean of 20 overlay passes is under 4 ms. Under a loaded machine or a shared CI runner the mean includes scheduling spikes: it failed once in a full workspace run on main at load ~70 and passed alone (0.11 ms).

## What

- Take the fastest of the runs, as the repo's other budget tests do, and give shared CI runners engine.rs's slack (3x, 6x on Windows).

## Acceptance criteria

- [x] The test asserts the fastest pass against the budget with CI slack
- [x] It passes repeatedly in a full parallel workspace run

## 2026-09-27

Now the fastest of 20 passes against 4 ms times CI slack (3x, 6x on Windows). It passed in two full parallel workspace runs at load ~70-90, where the mean-of-20 version had failed once.
