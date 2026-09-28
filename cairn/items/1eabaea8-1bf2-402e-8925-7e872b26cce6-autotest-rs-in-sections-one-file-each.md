---
id: 1eabaea8-1bf2-402e-8925-7e872b26cce6
title: autotest.rs in sections, one file each
type: chore
status: doing
milestone: bare-metal
assignee: green-forest
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: client
area: tests
---

## Why

autotest.rs is 4,030 lines. Every client PR touches it, which makes it the top source of rebase conflicts, and a slow file to read and build.

## What

Split it into a module per section (lighting, UI, tools, depth and so on), sharing one harness. Delete checks that duplicate a unit test.

## Acceptance criteria

- [ ] No autotest file is over 800 lines
- [ ] The same checks pass, in the same count or fewer with each deletion noted

## 2026-09-28

Measured on b90b069e: autotest.rs is 4,156 lines. The harness is about 370 lines (T, its helpers, block_diff, patch_diff, grid_shots, open_square), plus one run() of about 3,740 lines in 40 sections and two trailing helpers.
Decision:
- A pure move first, in one PR: autotest/mod.rs keeps the harness and a run() that calls each section in the same order. The sections go by theme into ui.rs, input.rs, tools.rs, building.rs, replace.rs, weather.rs, light.rs, indoors.rs and chalk.rs, each under 800 lines.
- Sections share a few values (the founder, home, defs, the build site and so on). A small Cx struct carries exactly the ones the compiler says a later section reads, and nothing else.
- Same checks, same order, same count: the autotest's passed total before and after is the proof.
- Deleting duplicates of unit tests comes second, as its own PR, each check named.
Order agreed with lucky-harbor (9553b617): the split lands first, then their flake fixes in the new files.
