---
id: 6ca1ea2e-42d7-4de5-bffa-2d8733c85669
title: A boar stampede of 7 kills a colony of 9 in two hours
type: bug
status: doing
milestone: defense
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: none
effort: m
layer: plugin
area: combat
---

## What happens

On seed 4 of the crosscheck scenario (every shipped mod, a stone-age colony with a wooden hut), on main b90b069e, wildlife_plus's boar stampede fires at dawn on day 18: 7 hostile boars at 431 points, against 9 colonists with colony strength 31.7. All nine are dead within 4,900 ticks (tick 350,860 to 355,766), about two in-game hours. Three raids in the four days before it did no harm: the raiders fled.

The stampede sizes itself by points alone (`clamp(points / 60, 2, 8)` boars) and never reads strength, which is in its context, so a colony that draws 431 points from its wealth meets nearly the most boars there are. The colonists were undrafted and at work. Drafted colonists don't fight back (f3aa2844), so a player couldn't have saved them either.

Found while fixing 224a5488: the crosscheck now turns random threats off because of this, so it no longer notices it.

## Reproduction

Seed: 4
Mods: every shipped mod
Tick: the stampede at 350,000 (day 18); the last colonist dies at 355,766

1. On main before 224a5488: `cargo run --release -p rim_sim --example crosscheck -- --seed 4 --days 18`
2. Day 18's line shows the colony gone: "Everyone is dead. The colony is lost." in the messages.

## Acceptance criteria

- [ ] A stampede's size reads the colony's strength as well as its points
- [ ] Seed 4's day-18 stampede, fired at the same tick, leaves the colony standing (test)

## 2026-09-29

PAUSED: done: stampede.luau sizes the herd by min(points/60, strength*0.5/6) (a boar's melee on colony_strength's scale), clamped 2..8; wildlife_plus test 'a stampede is no bigger than half what the colony can hit back with' fails on main (7 boars at points 431, strength 31.7) and passes (2; 5 at strength 63.4); crosscheck.rs fires boar_stampede at dawn on day 18 like the day-10 raid. Left: criterion 2. A release crosscheck --days 22 on main's history with the OLD sizing keeps its colony (13 colonists on day 18), so the crosscheck no longer reproduces the wipe; decide whether the day-18 fire stays (it keeps combat in the hash) and whether HERD_SHARE 0.5 is right (no balance run yet), then gate. Branch fix/6ca1ea2e-stampede-strength.
