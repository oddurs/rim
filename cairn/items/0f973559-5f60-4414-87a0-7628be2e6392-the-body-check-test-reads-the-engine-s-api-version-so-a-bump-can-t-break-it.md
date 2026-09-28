---
id: 0f973559-5f60-4414-87a0-7628be2e6392
title: The body check test reads the engine's API version, so a bump can't break it
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
effort: s
layer: engine
area: tests
---

## Problem

`rim_ui::check::tests::a_body_that_would_not_draw_fails_the_check` (from #340) writes `api = "0.6"` into its probe mod. #339 raised the engine API to 0.7 and updated the tests that existed then; #340 merged after it with the literal, so the test fails on main: "mod 'probe' targets api 0.6 but the engine provides 0.7".

## Acceptance criteria

- [x] The test writes the engine's `rim_sim::API_VERSION`, and passes on main
