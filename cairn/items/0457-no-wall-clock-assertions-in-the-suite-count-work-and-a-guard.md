---
id: 457
uid: 3163979c-4bcc-4b59-a524-a1f1167cb965
title: 'No wall-clock assertions in the suite: count work, and a guard'
type: chore
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
depends_on:
- 362
- 221
- 289
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Problem

Seven places in the default suite assert elapsed time: `rim_sim/tests/boundary.rs:258` (6b605f27), `rim_sim/tests/scripting.rs:401` (e8673d3f), `rim_ui/tests/engine.rs` (the frame budget, e2c56c9a), `rim_ui/tests/tokens.rs`, `rim_ui/tests/stores_sheet.rs`, `rim_ui/tests/fonts.rs` and `rim_client/src/draw.rs`. Three have failed CI on a busy runner. A time budget belongs in a bench (DESIGN.md §8a), not in a gating test.

## Proposal

- The three existing items each fix their own test. This item fixes the other four the same way: assert a count (steps, cells visited, draw calls, cache hits against scans) or move the timing to a bench.
- A guard test in `rim_sim` that scans `crates/*/tests/**/*.rs` and `crates/*/src/**/*.rs` test modules for `Instant::now`, `elapsed()` and `SystemTime` and fails with the file and line, with an allowlist for the benches.

## Acceptance criteria

- [x] None of the seven sites asserts wall-clock time (each listed here with what it counts instead)
- [x] The guard test fails on a deliberately added `Instant::now()` in a test and passes on main
- [x] Each rewritten test still fails when the thing it guards regresses (a deliberate regression shown for each)
- [x] The suite passes three times in a row under `nice` with four parallel builds loading the machine

## 2026-09-27

The seven sites, and what each asserts now in the default suite:
- rim_sim/tests/boundary.rs: pieces looked up by refresh_boundaries (#244, 6b605f27).
- rim_sim/tests/scripting.rs: the slow mod named from a profile the test hands check_mod_budgets (#238, e8673d3f).
- rim_ui/tests/engine.rs frame budget: rebuild counts (#251, e2c56c9a); the ms budget runs alone in CI's budget step.
- rim_ui/tests/tokens.rs, stores_sheet.rs: the rebuild and card behaviour; the ms budgets run alone in the budget step (timing_budgets()).
- rim_ui/tests/fonts.rs: that the second load comes from the cache (FontsFrom::Cache); cache-faster runs only under RIM_BUDGETS.
- rim_client/src/draw.rs storage overlay: the overlay's output; the ms budget runs only under RIM_BUDGETS.
- rim_ui/tests/engine.rs endless-loop test: the 5 s assert is gone; the step budget stops the loop and nextest's timeout catches a hang.
DESIGN §8a (decided after this item was written) keeps timing budgets in one isolated, single-thread CI step rather than benches, so the four UI sites gate on RIM_BUDGETS instead of moving to criterion. Claimed with --force past e2c56c9a: its code is on main; it waits only on ten green main runs.
The local nextest kill goes from 3 to 10 minutes: at load 280 the two slowest sim tests (weather forecast, snapshot carry-on, ~2 min alone) ran past 3 and failed gates that had nothing wrong.

## 2026-09-27

Guard shown failing: a test added to rim_ui/tests/board.rs reading Instant::now failed no_test_reads_the_clock_unless_timed_alone with the file and line; reverted, it passes. Rebased onto main, it also caught ground.rs (#247) and stock_fields.rs (#254), both print-only, now on its list.

## 2026-09-27

Regressions shown: a planted 10 ms sleep in Ui::frame, 500 ms in the font cache's read and 6 ms in storage_view fail all eight budget tests under RIM_BUDGETS=1 (8 failed, 1 passed, the unrelated font test) and pass without it, so the gated asserts still bite in CI's budget step. Under load: scripts/task test passed three times in a row under nice, 729/729 each, at load 130-250 (the gate), 41 and 63.

## 2026-09-27

Rebased onto c5d2161e, the guard caught spoilage.rs (#250, merged today) asserting a spoil pass under 3 ms. spoil() now returns the stacks it worked out; the test asserts each pass works out at most 1.5x its share and a cycle of SPOIL_EVERY passes covers every stack. With the stagger removed it fails: [4000, 4000, 4000, 4000].

## 2026-09-27

A new wall-clock assert landed with stock fields (#231): rim_sim/tests/stock_fields.rs two_stock_fields_on_a_big_map_are_cheap asserts under 0.02 ms a tick (6x slack on CI, none locally). It failed at 0.041 ms in a loaded local run on 2026-09-27. Count it with the others.
