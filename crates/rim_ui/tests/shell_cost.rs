//! What a frame of core's UI costs, with 30 colonists: holding still, and
//! with the pointer crossing a cell every frame. For measuring, not gating:
//! `cargo test --release -p rim_ui --test shell_cost -- --ignored --nocapture`.

mod common;

use common::*;
use rim_sim::world::Faction;
use rim_ui::Input;
use std::time::Instant;

const FRAMES: usize = 480;

#[test]
#[ignore]
fn holding_still_and_crossing_a_cell() {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 1..30 {
        sim.world.spawn_pawn(human, Faction::Player, c.offset(i % 6, i / 6), None);
    }
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = Some(c);
    let mut t = 0.0;
    let mut run = |cv: &mut rim_ui::view::ClientView, cross: bool| -> [f64; 4] {
        // Per frame: the whole call, and the engine's own build, layout and
        // paint, in ms. Medians, since a busy machine makes means useless.
        let mut cols: [Vec<f64>; 4] = Default::default();
        for i in 0..FRAMES {
            t += 1.0 / 60.0;
            if cross {
                cv.hover_cell = Some(c.offset((i % 24) as i32 - 12, 0));
            }
            let start = Instant::now();
            frame(&mut ui, &sim, cv, Input { mouse: (800.0, 480.0), time: t, ..Default::default() });
            cols[0].push(start.elapsed().as_secs_f64() * 1e3);
            cols[1].push(ui.info.build_us / 1e3);
            cols[2].push(ui.info.layout_us / 1e3);
            cols[3].push(ui.info.paint_us / 1e3);
        }
        cols.map(|mut v| {
            v.sort_by(f64::total_cmp);
            v[FRAMES / 2]
        })
    };
    // Settle: the first frames build everything.
    run(&mut cv, false);
    let still = run(&mut cv, false);
    let crossing = run(&mut cv, true);
    let row = |name: &str, m: [f64; 4]| {
        println!("{name:>9}: {:.3} ms a frame (build {:.3}, layout {:.3}, paint {:.3})", m[0], m[1], m[2], m[3]);
    };
    println!("core UI, 30 colonists, {FRAMES} frames, medians:");
    row("still", still);
    row("crossing", crossing);
    println!("  crossing costs {:.3} ms more than holding still", crossing[0] - still[0]);
}
