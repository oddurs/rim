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
- **Maths:** every `math` function gives identical results on every machine.
  - `+ - * /`, `%`, `//`, `floor`, `ceil`, `abs`, `min`, `max`, `sqrt` and
    `clamp` are exact IEEE operations.
  - `math.sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `log`,
    `log10`, `pow`, `sinh`, `cosh` and `tanh` are rim's own (the `libm`
    crate, a port of musl), not the platform's. Their accuracy is under
    1 ulp. `math.log(x, base)` is exact for bases 2 and 10; other bases
    compute `log(x) / log(base)`, which can be 1 ulp off (`math.log(81, 3)`
    is 4.000000000000001).
  - **The `^` operator** is the exception. It calls the platform's `pow`
    except for the exponents `2`, `3` and `0.5` written as literals, which
    Luau computes exactly. Use `math.pow(x, y)` for anything else. The game
    warns about each `^` that could desync, with its file and line (F3 shows
    load warnings).
  - For smooth curves over time, a field's terms (`input = "hour"`,
    `input = "year"`, `noise`) are often simpler, and the engine evaluates
    them in fixed point ([weather guide](weather.md)).
- **Iteration order:** `pairs` over a table with string or number keys visits
  them in the same order on every machine. Over tables keyed by tables or
  functions it doesn't, so sort first if the order matters.
- **State:** keep anything that must survive a save in script data
  (`rim.set_data`), not in Luau locals. See the [weather guide](weather.md#script-data-and-events).
- **Time:** use `rim.tick()`, `rim.hour()` and `rim.date()`. There is no
  wall clock.
