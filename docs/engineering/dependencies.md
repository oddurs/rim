# Dependencies and how they're configured

How each library is set up, why, and what was measured. Settings live in
`Cargo.toml`, `.cargo/config.toml`, `crates/rim_sim/src/script.rs` (sim Luau VM),
`crates/rim_ui/src/vm.rs` (UI Luau VM), `crates/rim_ui/src/{layout,text,fontcache}.rs`
and `crates/rim_client/src/main.rs` (`conf()`).

Measured on the reference machine (Apple Silicon, macOS), release builds.

## Versions

| Crate | Version | Features |
|---|---|---|
| mlua | 0.12 (Luau 0.736) | `luau` |
| hecs | 0.11 | default |
| toml | 1.1 | default |
| serde_path_to_error | 0.1 | |
| taffy | 0.14 | `std`, `taffy_tree`, `flexbox`, `content_size` |
| cosmic-text | 0.19 | default (`std`, `swash`, `fontconfig`) + `shape-run-cache` |
| macroquad | 0.4.16 | default |

The Rust toolchain is pinned in `rust-toolchain.toml` (1.98.1), and CI installs that
version rather than `stable`.

## Luau (mlua)

Two separate VMs. The sim's is deterministic and shared by every player in
co-op. The UI's is client-only (DESIGN.md §11).

**Sim VM** (`ScriptHost::load`):

- **Libraries.** Only `math`, `string`, `table`, `bit32`, `utf8` and `buffer`.
  - `collectgarbage` and `gcinfo` are removed: they report memory, which differs between machines.
  - `loadstring` is removed: it compiles code at run time.
  - `getfenv`/`setfenv` are removed: they reach other mods' environments and switch off Luau's fast paths.
  - `newproxy` and `os` are removed.
  - `math.random` is removed; scripts use `rim.random` (the world RNG).
- **Read-only.** The standard libraries and the global table are read-only.
  `rim` stays writable: that's how plugins offer APIs to each other
  (`rim.weather`, `rim.register_incident`).
- **Safe environments.** Each script's environment is a Luau *safe env*
  (`Table::set_safeenv`). Without it Luau disables its fast paths: cached
  imports such as `math.floor`, builtin fastcalls, and fast `pairs`/`ipairs`.
  A script loads into its own environment table, which starts out unsafe, so
  every global access was a full lookup. The trade-off: an import chain like
  `rim.weather.register` is resolved when the script loads. Replacing a
  function in `rim` later isn't seen by scripts that loaded before. Plugins add
  to `rim`; they don't monkey-patch it.
- **Compiler.** Optimization level 2 (inlines small local functions, unrolls
  constant loops) and debug level 1 (line numbers in errors).
- **Memory limit, 256 MB.** Hitting it fails the allocating script with an
  error. It costs nothing measurable in the sim.
- **Step budget.** 100 million interrupts (loop back-edges and calls) per hook
  or handler call; past it the call stops with "an endless loop?". It is
  *counted*, never timed, because a wall-clock limit would stop peers at
  different points and desync them. It costs about 11% on a loop-heavy
  benchmark.
- **Measured** (`examples/luau_bench.rs`, script-shaped work): 22.9 ms before,
  13.1 ms after, of which 1.4 ms is the step budget.

**UI VM** (`UiVm::load`):
- Sandboxed (`Lua::sandbox`) with safe module environments.
- Compiler level 2.
- A 250 ms wall-clock deadline per component build or handler, which is fine
  for client-only code.
- **No memory limit.** Measured, mlua's limit made UI rebuilds 45% slower
  (0.84 → 1.25 ms): the UI allocates many small tables. A runaway UI mod only
  hurts one player's client, and the deadline stops endless loops.

**Not used:**
- **`luau-jit`** (native code): on x86-64 it needs AVX and silently falls back
  to the interpreter without it, so peers could run different backends. Our
  script time goes to Rust callbacks and table building, which the JIT
  doesn't speed up.
- **`luau-vector4`**: we don't use Luau vectors.
- **GC tuning**: Luau's incremental collector paces itself, and no pauses
  have shown up in profiling.

## Floating point and lockstep

- **Rust never fuses `a*b + c` on its own and never enables fast-math.** So
  `opt-level`, LTO and codegen units don't change IEEE results. Don't add
  `target-cpu=native`, `target-feature` or `-Cllvm-args` fast-math flags: they
  give each player a different binary.
- **Luau is C++, and clang does fuse on arm64 by default.** It emitted 41
  fused multiply-adds in `libluauvm`: in the `%` operator, `math.lerp`,
  `math.noise` and the vector library. A fused result is rounded once and an
  unfused one twice, so a Mac and a PC could compute `a % b` differently.
  - `.cargo/config.toml` builds C/C++ with `-ffp-contract=off`, and
    `scripts/check-no-fma.sh` (run in CI) checks the result.
  - One fmadd remains, deliberately: clang expands `math.noise`'s
    `fmod(x, 256.0)` as `x - trunc(x/256)*256`. Multiplying by 256 is exact,
    so fused and unfused give the same answer.
- **Still platform-dependent:** `sin`, `cos`, `tan`, `exp`, `log`, `pow`/`^`
  and `atan2` come from each platform's maths library, in Rust and in Luau.
  The sim uses none of them for state (climate curves are fixed-point terms,
  DESIGN.md §4c). Scripts must not either (0147 tracks giving them
  deterministic versions).
- **Iteration order:** `pairs` over string and number keys is the same on
  every machine running the same Luau version (string hashes are unseeded).
  Tables, functions and userdata as keys hash by address, so their order
  differs. Upgrading Luau can change hashing, so treat a Luau bump as a
  determinism event.

## hecs

0.11 caches query preparation automatically. Queries now return components
only, and sites that need the entity ask for `(Entity, ...)`. The larger cost
in the AI isn't hecs: `ai.rs` scans every `Thing` for each pawn looking for
work, O(pawns × things). A per-def or spatial index is the fix, and it
belongs with the building sprint's changes to `ai.rs`.

For save games (0060): hecs' serializers rebuild archetypes, which can
change query order and the entity ids handed out after a load. The save
format needs a "save at tick N, load, run to M, compare with an unbroken run"
test.

## toml and error messages

toml 1.1 kept every API we use. Def deserialization goes through
`serde_path_to_error`, so a bad value names its key path, and the loader's
patch record names the mod whose patch set it:

```
thing/tree_oak (from core/defs/nature.toml): at `hp` (patched by 'bad'): invalid type: string "lots", expected u32
```

TOML parse errors already carry line and column. Exact file lines for
*deserialization* errors would need toml's span-keeping `DeTable` API through
our merge; not worth it yet.

## taffy

Flexbox only: we use no grid, block, float or `calc`, so those features are
off, which gives a smaller `Style` and binary. `layout::Engine` keeps one
`TaffyTree` and clears it per call rather than allocating a tree for every
layout and measurement. A scroll area's content height comes from one layout
of the area (the bottom of taffy's scrollable overflow rectangle), not a
natural-size layout of every child every frame. A test checks a padded scroll
list stops exactly at its last row. Most frames skip layout entirely (it's
cached by tree hash), so frame time is unchanged at about 0.13 ms; the gain
is in allocations and in the scroll measurement.

## cosmic-text and fonts

- **Font list cache** (`fontcache.rs`). `load_system_fonts` parses every
  installed font on every launch: about 220 ms cold, and 33 ms with a warm
  disk here, for 883 faces.
  - We store the face list (file, index, families, style, weight, stretch)
    in the platform cache folder and rebuild the database from it: 2 ms.
  - Trusted while every font file and font directory keeps its size and
    modification time; rebuilt after 30 days. `RIM_CACHE_DIR` overrides the
    location.
  - The client starts the load on a background thread before loading mods,
    so even a first run overlaps it with map generation.
- **`shape-run-cache`**: cosmic-text caches shaping by text and attributes
  independent of wrap width, so re-wrapping a label at a new width doesn't
  reshape it. It is trimmed every frame.
- **Hinting below 1.5× DPI**: glyph advances snap to whole pixels, which
  reads crisper at 1×. At 2× subpixel positions look smoother.
- **Kept:** `Shaping::Advanced`, needed for fallback fonts, kerning and
  ligatures. `fontconfig` only does anything on Linux.

## macroquad

`conf()` returns macroquad's own `Conf` (a plain miniquad `Conf` silently gets
default batch sizes):
- **Batches:** 16,000 vertices and 24,000 indices per draw call. Indices were
  the limit at 5,000: 6 per rectangle, 60 per circle. Vertex capacity must
  stay under 65,536 (u16 indices).
- **Vsync** (`swap_interval: 1`). macOS ignores it and paces frames to the
  display.
- **No blocking event loop:** the sim runs every frame.
- **OpenGL on macOS:** Metal can't compile the GLSL lighting shader. Moving to
  Metal needs an MSL version of `sky.rs`'s shader first.
- **X11 first on Linux:** Wayland is marked unstable upstream.
- **No MSAA:** costly at Retina sizes and in CI's software GL.

**Terrain** is one texture (a texel per cell, per-tile variation baked in,
nearest filtering) drawn as a single quad, rebuilt when the map changes.
Before, it was a rectangle per visible cell. Circles under 6 px radius are
8-sided polygons. Frame time is clamped to 0.1 s so a window drag doesn't
jump the camera or run a burst of ticks.

Measured by the autotest, zoomed all the way out: **155 → 66 draw calls,
5.9 → 2.2 ms CPU per frame.**

**Next, if the renderer needs more:**
- Draw UI rectangles from a white texel in the glyph atlas, so a panel
  batches into one call per clip region.
- Replace `draw_text` for item counts, which breaks the world batch.
- Use sprite quads for plants.
- Chunk the ground texture for maps beyond 250×250 (0098, 0193).

## Build profiles

| Profile | For | Settings |
|---|---|---|
| `dev` | editing | opt-level 1, dependencies at 3; line tables only, no dependency debug info |
| `release` | playing, CI | line tables only; `CARGO_PROFILE_RELEASE_DEBUG=0` in CI |
| `profiling` (`cargo prof`) | samply / Instruments | release + full debug info |
| `dist` (`cargo dist`) | shipped builds | fat LTO, one codegen unit, stripped, packed `.dSYM` on macOS |

- **`dist`, measured:** it builds in about 1.5 minutes. The binary is 6.4 MB
  (8.5 MB for release), passes the autotest, and ends a 20-day run with the
  same state hash as a release build: LTO and one codegen unit don't change
  results.
- **`panic` stays `unwind` everywhere.** mlua turns a panic in a Rust callback
  into a Lua error, so a broken mod reports an error instead of killing the
  game. Tests always build with unwind, so `abort` in release would also
  compile everything twice in CI.
- **Linker:** Linux x86-64 already uses rust-lld by default.
- **Disk:** most of `target/` was dependency debug info. A target directory on
  another volume belongs in `~/.cargo/config.toml` (`[build] target-dir`), not
  in the repo; `/target` in `.gitignore` also covers a symlink.
