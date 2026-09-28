---
id: 1eabaea8-1bf2-402e-8925-7e872b26cce6
title: autotest.rs in sections, one file each
type: chore
status: done
milestone: bare-metal
assignee: green-forest
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] No autotest file is over 800 lines
- [x] The same checks pass, in the same count or fewer with each deletion noted

## 2026-09-28

Measured on b90b069e: autotest.rs is 4,156 lines. The harness is about 370 lines (T, its helpers, block_diff, patch_diff, grid_shots, open_square), plus one run() of about 3,740 lines in 40 sections and two trailing helpers.
Decision:
- A pure move first, in one PR: autotest/mod.rs keeps the harness and a run() that calls each section in the same order. The sections go by theme into ui.rs, input.rs, tools.rs, building.rs, replace.rs, weather.rs, light.rs, indoors.rs and chalk.rs, each under 800 lines.
- Sections share a few values (the founder, home, defs, the build site and so on). A small Cx struct carries exactly the ones the compiler says a later section reads, and nothing else.
- Same checks, same order, same count: the autotest's passed total before and after is the proof.
- Deleting duplicates of unit tests comes second, as its own PR, each check named.
Order agreed with lucky-harbor (9553b617): the split lands first, then their flake fixes in the new files.

## 2026-09-28

Deleted 4 checks that only restated a unit test, each verified against it:
1. 'stone planned over the wood wall, which still stands' (replace.rs): rim_sim tests/replace.rs cancelling_a_replacement_leaves_the_old_wall asserts the same after the same Build. If the plan weren't made, the next check (the hatch is drawn) would fail anyway.
2. 'putting the tool down fades the grid out' (chalk.rs): at zoom 8, grid::strength is shown × band(8) = 0 whatever the fade, so it restated grid.rs no_grid_when_cells_are_small. The fade has its own unit test, a_tool_swap_holds_the_grid_up.
3. 'a pawn that has walked faces a way' (tools.rs): T::ticks calls Motion::face as the frame does, so this was motion.rs nothing_turns_while_paused's first assertion.
4. 'a walled hut counts as indoors' (weather.rs): rim_sim tests/rooms.rs hut_with_a_door_is_enclosed builds the same ring, with a door, and asserts indoors. A guard replaces it and fails only if there's no open ground for the hut, since the later 'not inside the hut' check is skipped then.
About 20 near-misses were rejected as integration checks. The audit list is in the PR.
