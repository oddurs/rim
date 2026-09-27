---
id: a207eded-13e8-468d-9b4a-1255cbb38d02
title: Can the planner name the jobs nobody can reach, one by one?
type: spike
status: planned
milestone: chalkline
created: 2026-09-27
updated: 2026-09-27
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

(Write the answer here before closing. If it's cheap, the notch item goes ahead; if not, record why and move the notch out of the milestone.)
