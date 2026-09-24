# Scripting rules

Sim scripts (`mods/<mod>/scripts/*.luau`) run in one shared, deterministic
Luau VM. Every player in a co-op game runs the same scripts on the same world
and must get bit-for-bit the same result, so the VM is locked down. UI scripts
(`mods/<mod>/ui/`) run in their own client-only VM; see
[Modding the interface](ui.md).

How it's configured, and why: [docs/engineering/dependencies.md](../engineering/dependencies.md).

## What's there

- **Libraries:** `math`, `string`, `table`, `bit32`, `utf8`, `buffer`, and
  the base functions (`pairs`, `ipairs`, `pcall`, `error`, `tostring`,
  `select`, ...).
- **Not there:**
  - `os`, `io`, `debug` and `coroutine`;
  - `collectgarbage` and `gcinfo`, which report memory that differs between machines;
  - `loadstring`, `getfenv`/`setfenv` and `newproxy`;
  - `math.random`: use `rim.random()` and `rim.random_int(a, b)`, which draw
    from the world's random numbers and replay identically.
- **The game's API** is the `rim` table (`rim.every`, `rim.on`, `rim.spawn_pawn`,
  `rim.push_ambient`, ...), plus whatever other plugins add to it.

## Sharing an API with other mods

`rim` is the one table you can add to:

```lua
rim.my_mod = {}
function rim.my_mod.register(def) ... end
```

Other mods that depend on yours call `rim.my_mod.register`. Add to `rim`;
don't replace another mod's functions. Scripts resolve chains like
`rim.weather.register` when they load (that's what makes them fast), so a
replacement made later isn't seen by scripts that loaded before it. The
standard libraries and the global table are read-only: `math.floor = ...`
is an error.

Each script's own globals live in a private environment, so two mods can both
have a `local function update()` or a global `state` without colliding.

## Limits

- **An endless loop is stopped.** A single hook or handler call may run 100
  million steps (loop iterations and calls); past that it stops with an error
  in the message feed, and the hook runs again next time. The limit is
  counted, not timed, so it stops at the same point on every machine.
- **Memory:** the VM may hold 256 MB. A script that allocates past it fails
  with an error.
- A script error never stops the game: it shows in the message feed, names
  your mod, and every other hook keeps running.

## Staying deterministic

- **Randomness:** only `rim.random` and `rim.random_int`.
- **Maths:** `+ - * /`, `%`, `//`, `math.floor`, `math.ceil`, `math.abs`,
  `math.min`, `math.max`, `math.sqrt` and `math.clamp` give identical results
  everywhere. `math.sin`, `math.cos`, `math.tan`, `math.exp`, `math.log`,
  `math.pow`, `^` and `math.atan2` come from each platform's maths library and
  can differ in the last bit. Don't let them affect the simulation.
  - For smooth curves over time, use a field's terms (`input = "hour"`,
    `input = "year"`, `noise`), which the engine evaluates in fixed point
    ([weather guide](weather.md)).
- **Iteration order:** `pairs` over a table with string or number keys visits
  them in the same order on every machine. Over tables keyed by tables or
  functions it doesn't, so sort first if the order matters.
- **State:** keep anything that must survive a save in script data
  (`rim.set_data`), not in Luau locals. See the [weather guide](weather.md#script-data-and-events).
- **Time:** use `rim.tick()`, `rim.hour()` and `rim.date()`. There is no
  wall clock.
