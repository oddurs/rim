# Research and the stat pipeline

Research is a plugin (`mods/research`). The engine knows nothing of it: it
gives mods a **stat pipeline**, and research is one reader of it.

## Modifiers

A `[[modifier]]` adds a number to a thing's stat while it is on:

```toml
[[modifier]]
id = "no_campfire_yet"
group = "research:fire_lore"
stat = "buildable"
thing = "core:campfire"
value = -1
reason = "Needs research: Fire lore"
```

- A thing def's stat is what the def says plus every modifier on it that is
  on, in load order: `hp`, `work` and `value` come from the def, and
  `buildable` is 1 for anything with a `build` block.
- A buildable whose `buildable` is 0 or less can't be placed: the build
  order is refused (the preview says it's locked), and the dock shows
  `reason` in place of its cost.
- A modifier is on from the start unless it says `on = false`. A script
  switches its mod's modifiers by group: `rim.set_modifiers("masonry",
  false)` from the mod that declares group `masonry`. The switch is saved
  with the world.
- A group can name another mod, `group = "research:joinery"`: that mod's
  `rim.set_modifiers("joinery", ...)` switches it too. That's how a mod gates
  its own content behind someone else's research.
- A modifier on a thing whose mod isn't installed does nothing, silently,
  as a patch to it would: a mod can gate an optional mod's content. One on a
  thing a loaded mod doesn't have is a load warning.
- Scripts read a stat with `rim.stat_of(thing, stat)`, and every modifier
  with `rim.modifier_defs`.

## Research projects

```toml
[[research.project]]
id = "fire_lore"
label = "Fire lore"
note = "How to keep a fire in."
work = 1200
requires = ["research:joinery"]
```

A project's unlocks are the `buildable` modifiers whose group is
`research:` and the project's name (`research:fire_lore` above), from any
mod; the research plugin's own may leave off the `research:`. The research
script turns the chosen project into work orders of 200 work at every
research desk (Research work, its own skill), counts each one done, and
switches the group off when the work is reached. Its state is script data,
`research:state`: `current`, `progress` and `done` by project id, and the
desks.

The screen (Research in the top bar) picks the project; it sends
`research:choose { project }`. A built desk joins by itself;
`research:desk { site }` adds one placed any other way.
