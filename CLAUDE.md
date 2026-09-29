# rim

A plugin-first 2D colony sim. Read DESIGN.md before changing anything
structural: the engine (`crates/rim_sim`) provides mechanisms only; all
content lives in mods (`mods/core` is the base game). If a feature needs
content ids in engine code, it belongs in a mod instead.

**Speed is the number one feature.** rim runs at bare-metal speed and
scales with the colony, and that comes before any feature or effect. Every
change states its cost in numbers: the same bench, and the same CI runner class
or the Mac with its window in front. The budgets in `budgets.toml` gate CI.
Anything that gets in the way of speed goes: delete it rather than hide it
behind an option. A setting that trades speed for looks is the rare
exception, not the fix. Measure how cost grows with pawns and map size, not
only one scene.

- `cargo run --release -p rim_client` — play
- `scripts/task check` — the gate: what CI's checks and test jobs run. It must
  pass before a push; `scripts/task hooks` makes the pre-push hook run it.
  `scripts/task test -- <filter>` while working; `scripts/task help` for the rest.
- Sim changes must stay deterministic: the world's random streams only (§7b), no HashMap iteration,
  all player input through `Command`.
- ROADMAP.md isn't tracked. `cairn render` writes it (gitignored) for a
  local look, and CI uploads main's as the `roadmap` artifact; there is
  nothing to re-render after a rebase. Where the generated block below says
  `cairn render`, that is all it does.
- New items land in `backlog`. A person plans them; `next` shows only
  planned and started work, so a backlog item is taken by a direct claim.
- Dropping an item or moving a milestone's `due` is a person's call:
  `cairn propose <ID> status=dropped --why "..."`, then `cairn proposals`.
  The cairn block below is generated: `cairn agent --view next --write CLAUDE.md`.

<!-- cairn:begin -->
## Roadmap and issues

This project tracks its roadmap and issues with `cairn`. Every item is a Markdown file under `cairn/items`, described by the schema in `cairn.toml`.

**Do not create ad-hoc TODO, PLAN or NOTES files.** Create a cairn item instead, so the work appears on the board and in the generated roadmap.

### The loop

1. `cairn next --view next` — what is ready to start. It excludes anything blocked by unfinished dependencies and puts work already in progress first.
2. `cairn claim <ID>` — take it before you start, so no one duplicates the work. `cairn claim --next --view next` picks and claims the top-ranked unclaimed item in one step. Then read `cairn prompt <ID>`: the item with everything it rests on — the outcome it serves, what its dependencies concluded, what done means and what earlier runs learned.
3. Do the work. Record what you learn: `cairn set <ID> <field>=<value>` for fields, `cairn note <ID> "<TEXT>"` for anything that needs a sentence — why you chose something, what you tried, what to watch for.
4. `cairn tick <ID> <N>` as each acceptance criterion becomes true — `cairn show <ID> --criteria` lists them numbered. Tick what is true, not what would let you close.
5. `cairn close <ID>` when it is done, or `cairn release <ID>` to hand it back.
6. `cairn check` before you report finished. It must pass.

### Commands

```sh
cairn next --view next --json                 # ready work, ranked
cairn claim --next --view next                # take the next ready item
cairn search <TEXT> --json        # titles, bodies and labels
cairn list --json                 # all open items
cairn list --filter 'blocked=false,priority=p0'
cairn prompt <ID>                 # the item as a prompt, with what it rests on
cairn show <ID> --json            # one item, including its body
cairn new "<TITLE>" --type <TYPE> --milestone <MILESTONE>
cairn set <ID> status=<STATUS>    # also labels+=x, or any field below
cairn note <ID> "<TEXT>"          # append reasoning; never replaces
cairn show <ID> --criteria        # acceptance criteria, numbered
cairn tick <ID> <N>               # tick one; --all for every one
cairn close <ID>
cairn check                       # validate; run before finishing
cairn render                      # regenerate ROADMAP.md
```

Selection uses saved view `next`. Additional filters only narrow it; the view's sort and columns do not change `next` ranking. Over MCP, pass `{"view":"next"}` to `next_items` and to `claim_item` without an id. A direct claim is an explicit assignment outside this selection policy. Regenerate these instructions with `cairn agent --view next --write CLAUDE.md`.

Claims coordinate writers in the same item directory and are seen across the worktrees of this repository: `next` leaves out what another worktree has claimed, `claim` refuses it, and `cairn worktrees` shows what each is doing. Separate clones are not read. Agree on assignments before splitting work across clones.

Items are numbered: `0012`, and commands accept the bare number too. Write the number in `depends_on` and other id references. Each item also carries a `uid` tag; leave it alone. If a merge gives two items one number, `cairn renumber` moves the one that arrived and retargets the references that came with it.

### Schema

- **Types**: `feature`, `bug`, `spike`, `perf`, `content`, `chore`, `docs`, `milestone`, `pillar`
- **Statuses**: `backlog` (open), `planned` (open), `doing` (active), `review` (active), `blocked` (active), `done` (done), `dropped` (dropped)
- **`priority`**: one of p0, p1, p2, p3 — p0 blocks the milestone; p3 is nice to have
- **`effort`**: one of s, m, l, xl — Rough size: s < half a day, m ~ a day or two, l ~ a week, xl needs splitting
- **`layer`**: one of engine, core, plugin, client, tooling — engine = rim_sim mechanisms; core = the core mod; plugin = first-party plugin; client = renderer/UI
- **`area`**: one of sim, ai, pathing, map, modding, scripting, storyteller, combat, needs, building, save, net, render, ui, audio, perf, tests, docs — Subsystem this touches
- **`api`**: one of none, additive, breaking — Effect on the plugin API (defs schema, Luau surface, events). Breaking needs an api version bump.
- **`due`**: date, YYYY-MM-DD — When a milestone is meant to land — **you may read this and not set it**
- **Milestones**: `foundations` (due 2026-09-30), `graphics`, `stone-age`, `lighting`, `work`, `pointer`, `proving-ground`, `chalkline`, `workbench`, `rock-face`, `people`, `rimos`, `bare-metal`, `carrying`, `castaway` (due 2026-10-15), `interface` (due 2026-10-09), `weather` (due 2026-10-01), `shelter` (due 2026-11-01), `building` (due 2026-11-15), `persistence` (due 2026-11-20), `colony` (due 2026-12-15), `eras` (due 2027-01-10), `plugin-api` (due 2027-02-01), `houses`, `mood` (due 2027-02-20), `sdk` (due 2027-03-01), `scale` (due 2027-03-15), `platform` (due 2027-07-01), `story`, `defense` (due 2027-04-10), `co-op` (due 2027-08-15), `depth`, `crafting` (due 2027-05-01), `1.0` (due 2027-10-01), `world` (due 2027-06-01)
- **Saved views** (`cairn list --view NAME`): `now`, `next`, `waiting`, `dependencies`, `api`, `engine`, `plugins`, `perf`, `decisions`, `triage`, `history`

### Rules

1. Before starting work, find or create the item and set it to an active status.
2. Use the fields above rather than inventing new ones; add new fields to `cairn.toml` first.
3. Never hand-edit the generated roadmap file — change items and run `cairn render`.
4. `cairn check` must pass before the work is considered done.

<!-- cairn:end -->
