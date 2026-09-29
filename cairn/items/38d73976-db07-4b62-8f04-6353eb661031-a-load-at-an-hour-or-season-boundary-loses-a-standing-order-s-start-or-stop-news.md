---
id: 38d73976-db07-4b62-8f04-6353eb661031
title: A load at an hour or season boundary loses a standing order's start or stop news
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
priority: p2
api: none
layer: engine
area: save
---

## What

`World::update_rules` (crates/rim_sim/src/rules.rs) keeps the (hour, season, stance, generation) it last worked the rules out for. When that key moves, a reading rule (one with a band) that starts or stops holding is news: `RuleStarted` or `RuleStopped`. The key is derived and not saved, and `Snapshot::restore` calls `update_rules` with no key, which is the silent first work-out, at the load tick's hour and season.

The live game at tick T still has the key from T-1. Its step at T works the rules out again and emits the news; the loaded game already folded the change in silently at load and emits nothing.

## How it fails

At an hour or season boundary, a band rule that reads the hour or the season diverges between the live and loaded game. Core's `wood_for_winter` has a season; its handler (`readings.luau`) posts a message, so the messages hash differs. Snapshots fall every 5,000 ticks, a quarter day, so every snapshot is on an hour boundary and every day's first is on a season boundary when one starts. A load from such a snapshot replays its log, disagrees at the next log, and rolls back to a new epoch.

Found by the save/load review sweep.

## Reproduce

In tests/standing_orders.rs: hold the reading in wood_for_winter's band until the tick autumn begins, and save exactly there. After one step, the live game's events or messages differ from the loaded game's.

## Acceptance

- [x] A loaded game works its rules out as the live one last did, and announces what the live one announces on the next step
- [x] Older saves still load
- [x] A test that fails before the fix and passes after

## 2026-09-29

PAUSED at the merge freeze: the fix and its test are done, and the test was checked to fail before the fix and pass after. The full gate hasn't run yet. Next: rebase on main, run scripts/task check, and mark the PR ready. Branch fix/38d73976-rules-on-load.
