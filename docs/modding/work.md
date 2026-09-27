# Modding work: who does what

Everything about who does which work is data or a script, and core uses
only what's described here. A mod can add kinds of work, roles, stances and
standing orders, replace how Auto plans, or add a view to the Work screen.

The design and its reasoning are in [DESIGN.md §4d](../../DESIGN.md).

## Who owns what

| Layer | Owns |
|---|---|
| Engine | Levels and how they resolve (plan or role, then pin, then rules), work pools and the walk, planned levels and the two-plan rule, readings and bands, urgent marks |
| `core` | Six work types, the scale's names, the Auto, Hand, Builder and Forager roles, Auto's planner (`scripts/auto.luau`), four stances, three standing orders and the readings behind them (`scripts/readings.luau`), the Work screen and its lenses |
| Your mod | Anything above, added, patched or replaced |

## How a level resolves

A colonist's level for a work type is 1 (first) to the scale's last, or 0
(never). It starts at their role's level, or, in a planned role like
Auto, at their planner's; what a role leaves out is the work type's
default. The player's own setting for that colonist, a pin, beats it.
Then each rule that holds sets it or shifts it, in load order. A shift
stops at the first level and never lifts a 0; only a set overrides never.

`rim.priority_parts(id, work)` returns those steps, and they add up to
`rim.priority(id, work)`:

<!-- not a sample -->
```lua
-- { kind = "default", label = "default", delta = 3 },
-- { kind = "plan", label = "fills Haul: 23 waiting, wants 3, has 2", delta = -1 },
-- { kind = "rule", label = "Food is low", delta = -1 }
```

## Work types

A work type is a column on the board. A colonist who has no setting for it
is at its `priority`, so a mod's new work type shows up on every colonist.

<!-- not a sample -->
```toml
[[work_type]]
id = "tailor"
label = "Tailor"
skill = "crafting"                       # trains it, and is faster with it
priority = 3                             # where colonists start
order = 55                               # the tie-break, left to right
auto = { per_person = 3, weight = 2 }    # what Auto reads
```

`auto.per_person` is how many waiting jobs one person keeps up with, and
`auto.weight` how much a waiting job matters next to others'.

## The scale

Core has four levels, named. A mod that wants more patches both, or empties
the names and the board shows numbers:

<!-- not a sample -->
```toml
[[patch]]
target = "priority_scale/core:core"
set = { levels = 9, labels = [] }
```

A label count that doesn't match `levels` fails to load.

## Work roles

A role is a partial set of levels that colonists belong to, one role each.
The colony keeps its own copy of every role, which the player edits; a copy
the player never touched follows its def when a mod updates it.

<!-- not a sample -->
```toml
[[work_role]]
id = "crafter"
label = "Crafter"
order = 40                                              # the first by order is where colonists start
priorities = { craft = 1, "core:chop" = 2, "core:haul" = 3 }
```

Patch another mod's role by its id:

<!-- not a sample -->
```toml
[[patch]]
target = "work_role/core:builder"

[patch.set.priorities]
"crafting:craft" = 4
```

Scripts read roles with `rim.work_roles()` and `rim.work_role(id)`.

## Auto, and replacing its planner

Auto is a role with a `planner`: a Luau function the engine calls once an
in-game hour with the board, and whose answer sets its members' levels.

<!-- not a sample -->
```toml
[[work_role]]
id = "auto"
label = "Auto"
order = -10
planner = "core:auto"
```

A planner gets the board (the same table `rim.work_board()` returns, with
the role's members marked) and returns levels for its members, each with a
reason the player reads on the board:

<!-- not a sample -->
```lua
rim.planner("careful", function(board)
    local plans = {}
    for _, c in board.colonists do
        if c.member then
            local cells = {}
            for _, w in board.work do
                if c.pins[w.id] == nil then
                    cells[w.id] = { level = if w.waiting > 0 then 2 else 3, reason = w.waiting .. " waiting" }
                end
            end
            plans[c.id] = cells
        end
    end
    return plans
end)
```

The engine keeps the promises, whatever the planner does. A level changes
only when two plans in a row agree, so colonists don't swap jobs every
hour. A work type with no plan yet takes the first. A 0, or a level for a
pinned cell, is refused and named in a message. To replace core's planner,
patch the role, so two mods doing it is a loader conflict:

<!-- not a sample -->
```toml
[[patch]]
target = "work_role/core:auto"
set = { planner = "my_mod:careful" }
```

## Stances and standing orders

A `[[priority_rule]]` holds while its `when` does: hours, seasons, a
stance, a colonist's need, or a colony **reading**. It `set`s or `shift`s
work types (negative is sooner). A stance is a named set of rules the
player switches with one click; rules for a stance name it in `when`.

A reading is a number your script publishes (see
[scripting](scripting.md#readings-and-standing-orders)). A rule on one is a
standing order, with a band so it doesn't flap:

<!-- not a sample -->
```toml
[[priority_rule]]
id = "fuel_low"
label = "Fuel is low"
when = { reading = "my_mod:fuel_days", below = 5, until = 8 }
shift = { "core:chop" = -1 }
```

An order is announced (`rule_started`, `rule_stopped`) when it starts or
stops moving priorities: its band, season, hours and stance together, and
whether the colony has switched it off.

## Urgent marks

The player marks one job urgent: a blueprint, a thing or creature marked
for work, or an order's site. Everyone takes it a level sooner than its
work type, never from 0, and it wins ties in its level. A mod's own work,
posted as an order or a designation, can be marked with no code of its own.

## Lenses on the Work screen

The Work screen shows one lens at a time. A lens reads `view.board()` and
writes only through `act`, so it can't disagree with the board. See
[the UI guide](ui.md#work-lenses).

## A worked example: tailoring

A mod that adds a Tailor work type, a Tailor role, and an order that puts
tailors to work when clothes wear thin, on a reading it publishes.

<!-- example: defs/tailor.toml -->
```toml
[[work_type]]
id = "tailor"
label = "Tailor"
priority = 3
order = 55
auto = { per_person = 3, weight = 2 }

[[work_role]]
id = "tailor"
label = "Tailor"
order = 50
priorities = { tailor = 1, "core:haul" = 2 }

[[priority_rule]]
id = "worn"
label = "Clothes are worn"
when = { reading = "tailor:worn", above = 0.5, until = 0.2 }
shift = { tailor = -1 }
```

<!-- example: scripts/main.luau -->
```lua
-- How worn the colony's clothes are, 0 to 1: here, a day's wear on a
-- ten-day cycle, standing in for a real count.
rim.every(rim.ticks_per_day // 24, function()
    local day = rim.tick() / rim.ticks_per_day
    rim.set_reading("worn", (day % 10) / 10)
end)
```
