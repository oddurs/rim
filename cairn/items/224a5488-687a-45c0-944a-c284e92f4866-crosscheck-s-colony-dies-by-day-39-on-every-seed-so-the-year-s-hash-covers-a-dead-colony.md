---
id: 224a5488-687a-45c0-944a-c284e92f4866
title: Crosscheck's colony dies by day 39 on every seed, so the year's hash covers a dead colony
type: bug
status: backlog
milestone: proving-ground
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: tooling
area: tests
---

## What happens

`examples/crosscheck` is the cross-machine determinism proof: every platform runs it, and each must print the same hash each day for a year. Its comment says seed 4 is "a colony that lives all 60 days, long enough for the hash to cover its work orders, building and needs". That's no longer true. The harness checks the colony only on day 5, so nothing notices.

I ran the scenario for 60 days on seeds 1–16, on 2026-09-28, on main at 02805dea (#328; its sim numbers match main's before it). The colony count comes from `World::colonists()` each day, and the causes from the messages.

- Seeds 1, 2, 3, 8, 9 and 13–16 stop at day 5: no hand axe (1, 2, 8, 16), blueprints waiting or no campfire (3, 13, 14, 15), or the colony already gone (9).
- Every seed that gets past day 5 loses the colony later:

| seed | colony gone on day | most colonists | in days 15–24: raids, boar stampedes, wolf packs |
|---|---|---|---|
| 4 (the default) | 39 | 10 | 5, 2, 0 |
| 5 | 30 | 4 | 2, 1, 0 |
| 6 | 29 | 8 | 4, 0, 0 |
| 7 | 24 | 7 | 5, 0, 1 |
| 10 | 34 | 4 | 3, 0, 0 |
| 11 | 29 | 3 | 2, 1, 3 |
| 12 | 29 | 6 | 3, 0, 0 |

Colonists die in raids (five raiders at a time, who flee afterwards), boar stampedes and wolf packs. Seed 7's run ends "Everyone is dead. The colony is lost." None of the deaths I read looked like hunger or cold. From the colony's end to day 60, the hash covers wildlife and weather only: no work orders, building or needs. So a determinism bug in those systems after about day 30 would pass crosscheck on every platform.

Found while measuring a9e6b195 (the sun and moon light the sim). That change moves the day of each loss (13–35 at latitude 45, 11–39 at 30), but the colony was already dying on main.

## What should happen

The harness proves what it says. Either the scenario keeps a colony alive for the whole run (it defends itself, or the storyteller is turned down for this run), or the harness stops claiming it does. Either way, crosscheck says so when the colony is gone before the last day, as it already does on day 5.

Whether an undefended colony should fall to the storyteller by day 30 is a balance question for the storyteller, not for this harness.

## Reproduction

Seed: 4 (and 5, 6, 7, 10, 11, 12)
Mods: every shipped mod (core, crafting, primitive, timber, iron, research, weather, farming, fire, wildlife_plus)
Tick: the colony is gone by day 39 (tick 780000)

1. `cargo run --release -p rim_sim --example crosscheck -- --seed 4`
2. Add `w.colonists().count()` to the daily line: it reaches 0 on day 39 and stays there, while the run exits 0.

## Acceptance criteria

- [ ] crosscheck prints the day the colony is lost, and fails if that's before the last day
- [ ] The default seed and scenario keep a colony alive for all 60 days, measured on main
- [ ] The comment in examples/crosscheck.rs says what the year's hash covers, and it's true
