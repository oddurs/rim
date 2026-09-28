---
id: 87f54c25-c94d-4ed2-877c-e4d25e79a056
title: The stock-field cost test times 0.003 ms under load and flakes
type: bug
status: done
milestone: crafting
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
---

## What happens

`stock_fields::two_stock_fields_on_a_big_map_are_cheap` asserts a timing budget, 0.02 ms a tick. Alone the pass takes about 0.003 ms. With every test running at once on a loaded machine it measured 0.0202 and 0.026 ms, and failed quiet-field's gate and mine.

## What should happen

The test checks what makes the pass cheap, which is the work it does: each tick works out at most one slice of the map (cells over the period's ticks), and a period works out every cell. The time is printed as a report, not asserted, as #244 did for the boundary refresh.

## Acceptance criteria

- [x] The test asserts the cells worked per tick and per period, not a time
- [x] The time is still printed

## 2026-09-27

Fixed as rim-c2 asked, the way rapid-cloud fixed the other timing flakes. A period of the rate-1 counter field counts the cells whose value changed each tick. That must be at most a slice (cells.div_ceil(period)), and after the period every cell has been worked. The timing loop runs apart from the counting, so the printed figure is the bare pass: 0.0027 ms a tick here. The wetness PR's ground test gets the same treatment in its own branch.
