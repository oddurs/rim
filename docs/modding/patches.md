# Patching defs

A mod changes another mod's defs with `[[patch]]` entries in its own `defs/`
files. Redefining a def with the same kind and id is an error: patch it
instead. Patches apply in load order: a mod's patches run after the mods it
depends on, or loads after.

## Ids

Every def id belongs to the mod that defines it: core's `wall` is
`core:wall`. Inside your own mod a bare id means your own def, and another
mod's def needs its prefix. That applies to patch targets, references in your
defs, and ids your scripts pass to `rim`:

<!-- not a sample -->
```toml
[[creature]]
id = "golem"                                      # my_mod:golem
butcher = [{ thing = "iron", count = 5 },         # my_mod:iron
           { thing = "core:raw_meat", count = 1 }] # core's meat
```

Two mods can both define `iron` without colliding. When a bare id isn't
yours but another mod has it, the error says which prefix you meant.

References in a def resolve in that def's own mod, even when your patch
wrote them. If you append to `core:deer`'s `butcher`, a bare `iron` means
core's iron, so write `my_mod:iron`.

## Patches

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
target = "thing/core:berry_bush"
set = { harvest = { regrow_days = 1.5 } }
```

`set` replaces what it names and merges tables, so `regrow_days` changes and
the rest of `harvest` stays. A list given to `set` replaces the whole list;
to change part of one, use the list operations below.

## Lists

`append` and `remove` take tables shaped like the def, down to the lists:

```toml
[[patch]]
target = "creature/core:deer"
append = { butcher = [{ thing = "wood", count = 2 }], spawn = { terrain = ["dirt"] } }

[[patch]]
target = "creature/core:human"
remove = { needs = ["warmth"] }
```

`remove` takes out every element equal to one you list. For lists of tables,
give only the keys that identify the element: `{ field = "temperature" }`
removes every entry whose `field` is `"temperature"`, whatever else it holds.
A removal that matches nothing is a warning.

To change an element in place, match it by its keys and `set` what changes:

```toml
[[patch]]
target = "thing/core:window"

[[patch.edit]]
list = "boundary"                 # a dotted path, like "spawn.terrain"
match = { field = "temperature" }
set = { leak = 2.0 }
```

Every matching element is edited. If none match, that's a warning. `match`
and `remove` compare values as the target def wrote them: core's walls say
`field = "temperature"`, so match on that, not on `"core:temperature"`.

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

## Your own def kinds

A plugin can offer data for other mods to extend, not just functions. It
declares a kind with a schema, and entries of that kind load, patch and
report conflicts like built-in defs:

<!-- not a sample -->
```toml
# mods/magic/defs/spells.toml
[[kind]]
id = "spell"                 # magic:spell

[kind.fields]                # optional; without it, any fields go
label = "string"
mana = "int"
school = { type = "string", default = "fire" }

[[spell]]                    # the declaring mod writes the bare name
id = "spark"
label = "spark"
mana = 2
```

Another mod that depends on `magic` adds entries as `[[magic.spell]]` and
patches them with `target = "magic:spell/magic:spark"`.

Field types are `string`, `int`, `float` (an int is fine), `bool`,
`table`, `list` and `any`. A missing field without a default, or one of the
wrong type, is a load error. A field the schema doesn't list is a warning.

Scripts read the entries with `rim.defs("spell")` (your own kind) or
`rim.defs("magic:spell")`. Each call returns fresh tables in load order,
ids qualified and defaults filled in. The weather plugin's `type` kind
([`mods/weather/defs/types.toml`](../../mods/weather/defs/types.toml)) is a
worked example.
