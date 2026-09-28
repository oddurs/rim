---
id: a207eded-13e8-468d-9b4a-1255cbb38d02
title: Can the planner name the jobs nobody can reach, one by one?
type: spike
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p3
api: none
effort: s
layer: engine
area: ai
---

## Question

The unreachable notch needs a per-job answer: this tree, this blueprint. `ai.rs` notes `Why::Unreachable` per work type and def (`r.note(wt, d, …)`), which is enough for the Work Board but not for the map. Can the per-job answer be had cheaply, without saving it or feeding it back into the sim?

## Hypothesis

The planner already visits the job when it finds no path. Recording the entity in the reasons it keeps for the why panel, capped and cleared each plan, costs little. The client reads it through the same view as the Work Board.

## Timebox

Half a day. Measure the soak benchmark before and after.

## Answer

**Yes, and cheaply, but not from the planner.** The hypothesis was half wrong: `choose_work` checks a job's reachability only when it is nearer than the best found so far, and it stops at the nearest reachable one. So it never visits most of the jobs nobody can reach, and recording its refusals would name a few of them at random.

The per-job answer comes straight from the map's regions instead: `ai::unreachable_jobs(w, a, b)`. It sorts the colonists into the regions they stand in, then asks each job in the cells from `a` to `b` (blueprints, things marked for work, order sites, creatures marked for work) whether one colonist per region can reach it, with `Map::can_reach`, a few cell lookups each. It is read-only, saved nowhere, and fed back into nothing. The regions are refreshed every tick (`sim.rs`), so an answer read between ticks is current.

Measured in release, seed 2, core only, 8 colonists, the whole 192×192 map marked to chop (1,157 jobs), load average around 65: **21 µs** for a 60×36 view and **1.05 ms** for the whole map. The notch can ask for the visible cells every frame, or once a tick.

The soak benchmark wasn't run: no code the sim runs changed. `unreachable_jobs` is new and only the notch and the tests call it.

The notch item (7ffd8d09) goes ahead on this.
