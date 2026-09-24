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
  Every engine function is listed in the [script API reference](api-scripts.md).
- **Def ids** are namespaced by mod: core's wall is `core:wall`. A bare id
  you pass to `rim` means your own mod's def, so write `core:temperature`
  for core's field. Ids come back qualified (`rim.thing_defs[i].id` is
  `"core:wood"`). See [Patching defs](patches.md#ids).

## Editor setup

[`types/rim.d.luau`](../../types/rim.d.luau) declares the `rim` API, and
[`types/ui.d.luau`](../../types/ui.d.luau) declares the UI's `ui`, `act`
and `view`, for [luau-lsp](https://github.com/JohnnyMorganz/luau-lsp), so
your editor can complete names, show the docs and flag a wrong call. Both
are generated from the engine's own declarations, and CI fails if they
drift.

In VS Code, install the Luau Language Server extension and add this to the
workspace's `.vscode/settings.json`:

```json
{
  "luau-lsp.platform.type": "standard",
  "luau-lsp.types.definitionFiles": { "@rim": "types/rim.d.luau", "@ui": "types/ui.d.luau" },
  "luau-lsp.sourcemap.enabled": false
}
```

The root `.luaurc` maps each shipped mod to an alias, so
`require("@core/scripts/storyteller")` and `require("@core/ui/kit")` resolve
the way the game resolves them. It's generated from the mods folder: after
adding a mod, run `RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types`.

The editor loads both files for every script, so it won't flag `rim` in UI
code. To check each kind of script against only its own API, as CI does, run
`LUAU_LSP=path/to/luau-lsp ./scripts/check-luau.sh`.

Scripts are nonstrict by default. `--!strict` at the top of a file is stricter,
but you'll need annotations on tables that start empty or `nil`. Type aliases
from the definition file, like `Faction`, aren't visible in scripts, so spell
out the union instead: `"player" | "hostile" | "wild"`.

## Sharing code: modules

`rim` is the engine's API, and it's read-only: `rim.my_mod = {}` is an error.
Mods share code as **modules**. A script returns what it exports:

```lua
-- mods/my_mod/scripts/api.luau
local api = {}
function api.register(def) ... end
return api
```

and other scripts `require` it:

```lua
local api = require("@my_mod/scripts/api") -- any mod's script, by path
local util = require("./lib/util")         -- your own, relative to this file
```

- **Top-level scripts run at load**, in name order. Scripts in
  subdirectories of `scripts/` are modules only: they run when something
  requires them. Either way a script runs once, and every `require` of it
  gets the same value.
- **Only declared dependencies.** A mod can require its own scripts and
  those of mods in its `depends` or `optional` list (`mod.toml`). Anything
  else is a load error naming both mods. Requiring an `optional` mod that
  isn't installed gives `nil`, so you can check:
  `local weather = require("@weather/scripts/weather")`, then
  `if weather then ... end`.
- **At load time.** Require at the top of a script. A module that hasn't
  loaded can't be required from a hook later, and a require cycle is a load
  error that prints the chain.
- **Exports are frozen** once their mod has loaded. Later scripts in the
  same mod can still add to them, but other mods can only read and call
  them. Keep changing state in locals or in script data (`rim.set_data`),
  not in your exports.

Core exports its storyteller (`require("@core/scripts/storyteller")`, with
`register_incident`), and the weather plugin exports `weather` (see
[Modding climate and weather](weather.md)).

The standard libraries and the global table are read-only too:
`math.floor = ...` is an error. Each script's own globals live in a private
environment, so two mods can both have a `local function update()` or a
global `state` without colliding.

## Events

`rim.on(name, fn)` hears engine events (`pawn_died`, `season_changed`, ...)
and mod events. `pawn_died` carries `id`, `name`, `creature`, `faction`, `x`,
`y` and `founder`, true when the colony's founder fell: a colony event a
storyteller mod can answer, not the end of the game. A mod emits only under
its own name:

```lua
rim.emit("my_mod:flood", { x = 10, y = 20 })
```

Emitting `"other_mod:..."` or a bare engine name is an error. The namespace is
the mod whose code calls `rim.emit`: when your hook calls
weather's `force`, it's the weather plugin that emits `weather:changed`.
Handlers run in load order, then registration order, and payloads are plain
data.

## Limits

- **An endless loop is stopped.** A single hook or handler call may run 100
  million steps (loop iterations and calls). Past that it stops with an error
  in the message feed, and that hook or handler is switched off for the rest
  of the game. The limit is counted, not timed, so it stops at the same point
  on every machine.
- **Slow mods are named.** If a mod's script calls average more than 0.5 ms,
  the profiler's warnings (F3) name it. This is only a warning: wall-clock
  time never changes the game.
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
