# Testing a mod

The engine is deterministic and runs without a window, so a mod can test
itself against real games. Build a world from a seed, drive it, advance it
and check what happened. A failing test prints the seed and tick, so it
fails the same way every time you run it.

Put tests in your mod's `tests/` folder:

```luau
-- mods/my_mod/tests/flood.luau
test("a flood soaks the colony", function(t)
	local w = t.world({ seed = 7 })
	w:call("@core/scripts/storyteller", "fire", "my_mod:flood")
	w:run_hours(2)
	t.expect(w:data("my_mod:flooded")).to_be(true)
end)
```

Run them with the game's own binary:

```sh
rim test mods/my_mod                  # one mod
rim test                              # every mod in ./mods with a tests/ folder
rim test mods/my_mod --filter flood   # tests whose name contains "flood"
rim test --junit report.xml           # a JUnit report for CI
```

It exits 1 if any test fails. A failure reads like this:

```text
my_mod/tests/flood.luau: a flood soaks the colony
my_mod/tests/flood.luau:5: expected true, got nil (world seed 7, tick 1667)
```

## Worlds

`t.world({ seed, mods, size })` builds a fresh world, exactly as a new game
would. Every option is optional:

- `seed` defaults to 1.
- `mods` defaults to your mod and everything it depends on.
- `size` defaults to the normal map size. A smaller map builds faster.

Each world has its own sim VM, so tests don't affect each other.

| Method | What it does |
|---|---|
| `w:step(n)`, `w:run_hours(h)`, `w:run_days(d)` | Advance the game |
| `w:tick()`, `w:day()`, `w:seed()`, `w:hash()` | Time, and the state hash (equal hashes, equal worlds) |
| `w:colony_center()` | The colonists' average cell: `x, y` |
| `w:colonists()` | `{ id, name, hp, x, y }` for each colonist |
| `w:count_pawns(faction)` | Living pawns of `"player"`, `"hostile"` or `"wild"` |
| `w:spawn_pawn(creature, faction, x, y)` | Add a creature now; returns its id |
| `w:spawn_item(thing, x, y, count)` | Drop items near a cell; returns how many didn't fit |
| `w:designate(designation, x1, y1, x2, y2)` | A player command, applied at the next tick |
| `w:build(thing, x1, y1, x2, y2, stuff)` | Place blueprints; `x2`, `y2` and `stuff` are optional |
| `w:send(name, data)` | Send your mod's sim scripts an event, as the UI's `act.send` does |
| `w:call(module, fn, ...)` | Call a function a script exports, inside this world |
| `w:data(key)` | Script data (`rim.set_data`) |
| `w:messages()` | The message feed: `{ text, kind, tick }` |
| `w:ambient(field)`, `w:field(field, x, y)` | Field values |
| `w:warnings()` | Load warnings, such as patch conflicts |

`w:call` runs a function the way a hook would, so it can use the whole
`rim` API. Arguments and results are plain data. Core's storyteller exports
`fire(id, overrides)`, which sets off any registered incident now, and
`ids()`, which lists them.

## Expectations

`t.expect(value)` then:

| Check | Passes when |
|---|---|
| `.to_be(x)` | `value == x` |
| `.never_to_be(x)` | `value ~= x` |
| `.to_be_truthy()`, `.to_be_falsy()` | `value` is (not) false or nil |
| `.to_be_at_least(n)`, `.to_be_at_most(n)` | A number on the right side of `n` |
| `.to_be_near(x, within)` | A number within `within` of `x` (default `1e-6`) |
| `.to_contain(x)` | A string containing `x`, or a list with `x` in it |

A Luau error in a test fails it too, with its message and line.

The shipped mods test themselves: see `mods/core/tests`,
`mods/weather/tests` and `mods/wildlife_plus/tests`.
