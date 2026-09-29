---
id: 530
uid: d137353c-761f-4d83-b402-7c7b37a2a7c8
title: 'Opening a colony under other mods: say what changed before loading, and open it as saved'
type: feature
status: backlog
milestone: platform
depends_on:
- 542
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: save
pillar:
- determinism
- plugin-first
---

## Why

A save's last epoch records every mod's (id, version) in load order
(`savefile::Epoch::mods`). When that differs from what's installed,
`SaveFile::load`:

- restores the newest snapshot under the installed mods;
- drops the tail of the log that can't replay (`LoadReport::lost`);
- removes things whose defs are gone (`LoadReport::dropped`, e.g.
  "dropped 3 × boars:boar");
- starts a new epoch.

`save::open` then tells the player in notes, after the fact.

There is a way back the client doesn't use. `SaveFile::load` takes an
`enabled` filter, but the client always passes `&|_| true`. When the player
has only added a mod since saving, the save could open under its own mods
with no new epoch and nothing lost.

DESIGN.md §7a, "Tension: say what changed before opening, or after?"

## What

Everything here compares **sim sides** (e4b96647):
a mod with only `ui/` is the player's own and never counts as a change.

**`savefile::preview(path, mods_dir) -> Preview`** reads the save without
opening it for writing. It reports:

- the sim-side mods added, removed and changed;
- the ticks that can't replay (the same arithmetic as `lost`);
- `as_saved`: whether every sim-side mod the save used is installed as it
  was;
- what removed mods take with them. This needs the snapshot restored under
  the installed mods (`restore_noting`). Measure it; over 200 ms on a day-30
  save, compute it only when the player opens Details.

**The title screen** marks a save whose mods differ, in one line under its
name ("Made with Wildlife+, which isn't installed"; "Wildlife+ was added
since"; "Weather 0.1.0 → 0.2.0"). `SaveView` gains the line, read from the
epoch only.

**Opening a changed save** shows a panel before anything loads:

- one row per change;
- what goes, in the player's words: "3 boars, from Wildlife+" (the def's name
  where it's installed; the mod's id where the mod is gone);
- how many ticks are lost;
- **Open as saved** (primary; only when `as_saved`): loads with `enabled` set
  to the save's sim-side mods. No new epoch, nothing lost.
- **Open with these mods**: today's behaviour, a new epoch. A version change
  runs the mod's migrate hook (0139) at the boundary, as today.
- **Cancel**.

A save whose sim sides match opens at once, as now. After opening with
changed mods, the existing notes use the panel's wording.

**Later, not here.** When the mod store lands (ead42976), "Open as saved" also works after an update: the save's versions
stay installed, and a missing one is fetched from the index.

## Acceptance criteria

- [ ] `savefile::preview` reports added, removed and changed sim-side mods and the ticks that can't replay, and leaves the file's bytes unchanged (test)
- [ ] A save with a sim mod installed since opens "as saved": no new epoch, `lost == 0`, and the log replays (test)
- [ ] A client-only mod added or removed since saving shows no difference and opens at once (test)
- [ ] A save whose mod was removed offers only "Open with these mods" and Cancel; opening it drops exactly what the panel listed (test)
- [ ] The title row shows the difference line, computed without restoring the world (test on `SaveView`)
- [ ] The panel names installed defs by name, not id (rim_ui test)
- [ ] Cancel returns to the title with the save untouched
- [ ] The preview's time on a day-30 save is measured and noted on this item; over 200 ms, the dropped list moves behind Details
