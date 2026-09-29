---
id: 6db2f064-c93b-40f6-a7df-1031ecb9406d
title: Fixed-point term products can overflow i64
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: modding
---

## What

terms.rs ~366 `acc = acc * v / Q` and ~178 `y0 + (y1 - y0) * (x - x0) / (x1 - x0)` multiply two Q values in i64. `to_q` saturates, so `scale = inf` gives `i64::MAX`, and the first multiply by anything above `Q` overflows. With finite values the limit is a product above about 9.2e10 in real units, such as two stock-field inputs near their ±200000 cap with scale ≥ 10.

## How it fails

Debug panics; release wraps (deterministically) to nonsense values in fields and growth.

## Reproduce

Not yet run; verified by reading. A term with `scale = 1e12` over a field of value 1e3, evaluated.

## Fix

Use i128 intermediates in the products, or have `compile` reject non-finite scales, constants and curve points and bound them.

## Acceptance

- [ ] No term input a load accepts can overflow evaluation
- [ ] A test that fails before the fix and passes after
