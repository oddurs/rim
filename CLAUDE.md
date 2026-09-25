# rim

A plugin-first 2D colony sim. Read DESIGN.md before changing anything
structural: the engine (`crates/rim_sim`) provides mechanisms only; all
content lives in mods (`mods/core` is the base game). If a feature needs
content ids in engine code, it belongs in a mod instead.

- `cargo run --release -p rim_client` — play
- `cargo test -p rim_sim` — includes the determinism test; it must pass
- Sim changes must stay deterministic: world RNG only, no HashMap iteration,
  all player input through `Command`.
- Dropping an item or moving a milestone's `due` is a person's call:
  `cairn propose <ID> status=dropped --why "..."`, then `cairn proposals`.
  The cairn block below is generated: `cairn agent --view next --write CLAUDE.md`.

<!-- cairn:begin -->
## Roadmap and issues

This project tracks its roadmap and issues with `cairn`. Every item is a Markdown file under `cairn/items`, described by the schema in `cairn.toml`.

**Do not create ad-hoc TODO, PLAN or NOTES files.** Create a cairn item instead, so the work appears on the board and in the generated roadmap.

### The loop

1. `cairn next --view next` — what is ready to start. It excludes anything blocked by unfinished dependencies and puts work already in progress first.
2. `cairn claim <ID>` — take it before you start, so no one duplicates the work. `cairn claim --next --view next` picks and claims the top-ranked unclaimed item in one step, and prints its body so you can begin immediately.
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

Selection uses saved view `next`. Additional filters only narrow it; the view's sort and columns do not change `next` ranking. Over MCP, pass `{"view":"next"}` to `next_items` and to `claim_item` without an id. A direct claim is an explicit assignment outside this selection policy. Regenerate these instructions with `cairn agent --view next --write AGENTS.md`.

Claims coordinate writers in the same item directory, not separate branches, worktrees, or clones. Agree on assignments before splitting work.

Item identities are immutable UUIDv4 strings. Use full `id` values from JSON for durable references; commands also accept unambiguous prefixes of at least 8 hex digits. Store full identities in ID-reference fields, never prefixes. Migrated legacy numbers remain lookup aliases; new items do not receive numbers.

### Schema

- **Types**: `feature`, `bug`, `spike`, `perf`, `content`, `chore`, `docs`, `milestone`, `pillar`
- **Statuses**: `backlog` (open), `planned` (open), `doing` (active), `review` (active), `blocked` (active), `done` (done), `dropped` (dropped)
- **`priority`**: one of p0, p1, p2, p3 — p0 blocks the milestone; p3 is nice to have
- **`effort`**: one of s, m, l, xl — Rough size: s < half a day, m ~ a day or two, l ~ a week, xl needs splitting
- **`layer`**: one of engine, core, plugin, client, tooling — engine = rim_sim mechanisms; core = the core mod; plugin = first-party plugin; client = renderer/UI
- **`area`**: one of sim, ai, pathing, map, modding, scripting, storyteller, combat, needs, building, save, net, render, ui, audio, perf, tests, docs — Subsystem this touches
- **`api`**: one of none, additive, breaking — Effect on the plugin API (defs schema, Luau surface, events). Breaking needs an api version bump.
- **`due`**: date, YYYY-MM-DD — When a milestone is meant to land — **you may read this and not set it**
- **Milestones**: `foundations` (due 2026-09-30), `graphics`, `stone-age`, `castaway` (due 2026-10-15), `interface` (due 2026-10-09), `weather` (due 2026-10-01), `shelter` (due 2026-11-01), `building` (due 2026-11-15), `persistence` (due 2026-11-20), `colony` (due 2026-12-15), `eras` (due 2027-01-10), `plugin-api` (due 2027-02-01), `mood` (due 2027-02-20), `sdk` (due 2027-03-01), `scale` (due 2027-03-15), `platform` (due 2027-07-01), `defense` (due 2027-04-10), `co-op` (due 2027-08-15), `crafting` (due 2027-05-01), `1.0` (due 2027-10-01), `world` (due 2027-06-01)
- **Saved views** (`cairn list --view NAME`): `now`, `next`, `api`, `engine`, `plugins`, `perf`, `decisions`, `triage`

### Rules

1. Before starting work, find or create the item and set it to an active status.
2. Use the fields above rather than inventing new ones; add new fields to `cairn.toml` first.
3. Never hand-edit the generated roadmap file — change items and run `cairn render`.
4. `cairn check` must pass before the work is considered done.

<!-- cairn:end -->
