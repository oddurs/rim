---
id: 768b6e39-1e85-4da3-aae1-3b821a496ca2
title: Replay drops commands applied at a log's own tick
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: save
---

## What

`replay_logs` (crates/rim_sim/src/savefile.rs, ~line 448) steps the sim up to each log's tick. It pushes each command before the step that starts at the command's tick:

```rust
while sim.world.tick < log.tick {
    while let Some((_, c)) = cmds.next_if(|c| c.0 == sim.world.tick) { sim.push(c.clone()); }
    sim.step();
}
```

A command applied with `Sim::apply_pending` at tick T, with no step after it, is logged at T in a log whose own tick is T. For example: pause, give an order, quit, and `close` snapshots at T. The loop stops when the tick reaches T, so that command is never pushed. The next log only replays commands after it (`c.0 >= from`), so the command is lost altogether.

## How it fails

- `rim replay` of such a save reports a divergence at T that isn't there.
- `SaveFile::load` replaying from an earlier snapshot (the newest one's record was torn) sees the hash disagree at T. It rolls back to the log before and opens a new epoch, and the player loses the tail.

Found by the save/load review sweep.

## Reproduce

In tests/savefile.rs: record, push a command, `apply_pending`, `save.log`, then `replay(path)`. The report says it diverged at that tick.

## Acceptance

- [ ] Commands applied at a log's own tick are replayed before its hash is checked
- [ ] A test that fails before the fix and passes after
