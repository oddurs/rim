# Benchmarks

Two benches measure rim's speed: the sim bench (`cargo run --release -p
rim_sim --example bench`) and the render bench (`rim --bench-render`). The
render bench plays the DESIGN.md §8 world, a 250×250 map with 30 colonists
and 200 pawns, through the game's own frame and render in ten views.

## Running the render bench on a Mac

Run it with `scripts/bench-front`, with the window in front and the machine
otherwise idle.

- **A hidden window measures nothing.** macOS throttles a window that is in
  the background or covered, so a run started with `--background`, or left
  behind another window, reports the throttle rather than the renderer.
- **One bench at a time.** Two benches running together swing the pawn
  pass by up to 8×. Ask the merger for a bench slot first, and say when
  you're done.
- **Vsync is off for the bench** (`swap_interval` 0), so a frame measures
  its cost rather than the display's refresh. macOS paces frames with the
  display anyway, so on a Mac the `rest` column below includes that wait.

## Reading it

The first table gives each pass's CPU time per view, plus submit (handing
the frame to GL), GPU time (Linux only, from `glFinish`), draw calls and
indices.

The second table is the whole frame: the wall time from one frame's start
to the next. It gives the median, p99 and worst frame, the hitches (frames
over twice the median), and `rest`. `rest` is the mean frame less every
pass, submit and GPU: the present, and any wait the passes don't account
for.

The last lines name the GL renderer and the machine.

## CI's runner classes

GitHub's Linux runners come in two classes, and the same pass differs by up
to 3× between them: the moving light pass costs about 3.8 ms on one and
1.35 ms on the other. The bench prints `machine:` (the CPU and its thread
count) so two runs can be compared.

- **Compare two CI runs only when their `machine:` lines match.**
- **Or compare ratios** against a pass the change didn't touch.
- **Never compare one run against an old one** on an unknown class.

## Budgets

Each bench's `--check` holds it to `budgets.toml` at the repo root, and so
does CI. A budget has a `target`, the goal on the reference machine, and a
`cap`, what CI fails over today. A cap only goes down: lower it in the PR
that earns it, and raising one needs a note saying why. Times swing with a
busy machine; counts of work (`max_nodes`, `nodes_per_search`, `hitches`,
`draw_calls`) don't, and they are held to their caps as they are.

<!-- budgets.toml: begin -->
| Budget | Target | Cap |
|---|---|---|
| `sim.base.max_ms` | 4 | 12 |
| `sim.base.max_nodes` | 330 | 350 |
| `sim.base.mean_ms` | 0.1 | 0.2 |
| `sim.base.nodes_per_search` | 10 | 12 |
| `sim.base.p99_ms` | 1 | 2 |
| `sim.winter.max_ms` | 4 | 120 |
| `sim.winter.max_nodes` | 5000 | 40000 |
| `sim.winter.mean_ms` | 0.1 | 0.3 |
| `sim.winter.nodes_per_search` | 20 | 100 |
| `sim.winter.p99_ms` | 1 | 5 |
| `render.gpu.draw_calls` | 60 | 170 |
| `render.gpu.frame_max_ms` | 33 | 75 |
| `render.gpu.frame_p99_ms` | 16.6 | 70 |
| `render.gpu.hitches` | 0 | 130 |
| `render.gpu.mesh_change_ms` | 1 | 3 |
| `render.gpu.world_ms` | 2 | 4 |
| `render.software.draw_calls` | 60 | 70 |
| `render.software.frame_max_ms` | 80 | 165 |
| `render.software.frame_p99_ms` | 70 | 155 |
| `render.software.hitches` | 0 | 10 |
| `render.software.mesh_change_ms` | 1 | 3 |
| `render.software.world_ms` | 2 | 6 |

On CI, a time (`_ms`) is held to its cap times the runner class's slack: 1 on AMD EPYC 9V74, 3 on any other.
<!-- budgets.toml: end -->
