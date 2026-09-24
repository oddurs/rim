# Patching defs

A mod changes another mod's defs with `[[patch]]` entries in its own `defs/`
files. Redefining a def with the same kind and id is an error: patch it
instead. Patches apply in load order: a mod's patches run after the mods it
depends on, or loads after.

Every patch names its target as `kind/id` and does one or more of these, in
this order:

| Key | What it does |
|---|---|
| `set` | Replace fields. Tables merge, so you only write what changes |
| `edit` | Change list elements that match a key (`[[patch.edit]]`) |
| `remove` | `true` removes the def; a table removes list elements |
| `append` | Add elements to the end of lists |

Any other key is an error, so a typo like `sett` doesn't pass silently.

## Fields

```toml
[[patch]]
target = "thing/berry_bush"
set = { harvest = { regrow_days = 1.5 } }
```

`set` replaces what it names and merges tables, so `regrow_days` changes and
the rest of `harvest` stays. A list given to `set` replaces the whole list;
to change part of one, use the list operations below.

## Lists

`append` and `remove` take tables shaped like the def, down to the lists:

```toml
[[patch]]
target = "creature/deer"
append = { butcher = [{ thing = "wood", count = 2 }], spawn = { terrain = ["dirt"] } }

[[patch]]
target = "creature/human"
remove = { needs = ["warmth"] }
```

`remove` takes out every element equal to one you list. For lists of tables,
give only the keys that identify the element: `{ field = "temperature" }`
removes every entry whose `field` is `"temperature"`, whatever else it holds.
A removal that matches nothing is a warning.

To change an element in place, match it by its keys and `set` what changes:

```toml
[[patch]]
target = "thing/window"

[[patch.edit]]
list = "boundary"                 # a dotted path, like "spawn.terrain"
match = { field = "temperature" }
set = { leak = 2.0 }
```

Every matching element is edited. If none match, that's a warning.

## Conflicts

The loader reports conflicts in the load warnings (the F3 profiler shows
them). Load order still decides the result:

- Two mods `set` the same field, including the same field of a matched
  list element.
- A mod `set`s a whole list that another mod appended to, removed from or
  edited, which throws their changes away.

These aren't conflicts, because both mods' changes survive:

- Two mods appending to the same list.
- One mod removing and another appending.

Patching a def that isn't there (say, from an optional mod that isn't
installed) is a warning, and the patch is skipped.
