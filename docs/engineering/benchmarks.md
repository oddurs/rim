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
