//! What the renderer remembers about how pawns move (DESIGN.md §6h): the
//! cell each stepped out of, so a turn it's leaving is drawn round, and
//! which way each faces. Presentation only: nothing here reaches the sim,
//! and a fresh `Motion` (after a load) only draws the first turn square.

use crate::worksite::Worksites;
use rim_sim::hecs::Entity;
use rim_sim::world::{Pawn, World};
use rim_sim::IVec;
use rim_ui::view::CameFrom;
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// How fast a drawn pawn turns, in radians a game second, so a U-turn
/// takes a visible moment rather than a frame.
pub const TURN_RATE: f32 = 12.0;

#[derive(Default)]
pub struct Motion {
    /// Each pawn's last cell and the one before it, the second being the
    /// cell it came from. Shared with the UI, which anchors names to the
    /// same curve.
    came_from: Arc<CameFrom>,
    at: HashMap<Entity, IVec>,
    facing: HashMap<Entity, f32>,
    /// The game time of the last `face`, in ticks.
    time: Option<f64>,
}

impl Motion {
    /// After each sim tick: note every pawn that moved on to a new cell.
    pub fn stepped(&mut self, w: &World) {
        let came = Arc::make_mut(&mut self.came_from);
        for &e in &w.pawns {
            let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
            match self.at.insert(e, p.pos) {
                Some(was) if was != p.pos => {
                    came.insert(e, was);
                }
                _ => {}
            }
            // A pawn that stopped has no turn in progress: when it sets
            // off again, sideways or back, it leaves straight. At a
            // corner the sim takes the next step in the tick it arrives,
            // so a turn it's making keeps its cell.
            if p.next.is_none() {
                came.remove(&e);
            }
        }
        if self.at.len() > w.pawns.len() {
            self.at.retain(|&e, _| w.ecs.contains(e));
            came.retain(|&e, _| w.ecs.contains(e));
            self.facing.retain(|&e, _| w.ecs.contains(e));
        }
    }

    /// Once a frame: turn each pawn toward where it's going, or toward
    /// its work, by as much as the game time since the last frame allows.
    /// A paused game has no game time, so nothing turns.
    pub fn face(&mut self, w: &World, sites: &Worksites, frac: f32) {
        let now = w.tick as f64 + frac as f64;
        let dt = self.time.map_or(0.0, |t| (now - t).max(0.0)) as f32;
        self.time = Some(now);
        let most = TURN_RATE / 60.0 * dt;
        for &e in &w.pawns {
            let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
            let want = rim_ui::turns::heading_through(&p, self.from(e), frac).or_else(|| sites.work_heading(&p));
            let Some(want) = want else { continue };
            let f = self.facing.entry(e).or_insert(want);
            *f = turn_toward(*f, want, most);
        }
    }

    /// The cell `e` last stepped out of, while it's still walking. A cell
    /// that isn't a neighbour (after a jump) draws no turn.
    pub fn from(&self, e: Entity) -> Option<IVec> {
        self.came_from.get(&e).copied()
    }

    /// Which way `e` is drawn facing, in radians clockwise from north, once
    /// it has moved or worked.
    pub fn facing(&self, e: Entity) -> Option<f32> {
        self.facing.get(&e).copied()
    }

    /// What the UI anchors names and bubbles with.
    pub fn came_from(&self) -> Arc<CameFrom> {
        self.came_from.clone()
    }
}

/// `from` turned toward `to` by at most `most` radians, the short way round.
pub fn turn_toward(from: f32, to: f32, most: f32) -> f32 {
    let d = (to - from + PI).rem_euclid(TAU) - PI;
    (from + d.clamp(-most, most)).rem_euclid(TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_is_capped_and_goes_the_short_way() {
        assert_eq!(turn_toward(0.0, 1.0, 0.2), 0.2);
        // From just west of north to just east of it: across north, not
        // round the long way.
        let t = turn_toward(TAU - 0.1, 0.1, 0.05);
        assert!((t - (TAU - 0.05)).abs() < 1e-5, "{t}");
        assert!((turn_toward(0.3, 0.35, 1.0) - 0.35).abs() < 1e-6, "arrives without overshooting");
    }

    #[test]
    fn a_pawn_that_stops_forgets_where_it_came_from() {
        let sim = rim_sim::Sim::new(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 1)
            .expect("the mods load");
        let e = sim.world.pawns[0];
        let mut m = Motion::default();
        let start = sim.world.ecs.get::<&Pawn>(e).unwrap().pos;
        let east = IVec::at(start.x + 1, start.y, start.z);
        m.stepped(&sim.world);
        {
            // It stepped east and is on its way further.
            let mut p = sim.world.ecs.get::<&mut Pawn>(e).unwrap();
            p.pos = east;
            p.next = Some(IVec::at(east.x + 1, east.y, east.z));
        }
        m.stepped(&sim.world);
        assert_eq!(m.from(e), Some(start), "walking on, it remembers the cell it left");
        sim.world.ecs.get::<&mut Pawn>(e).unwrap().next = None;
        m.stepped(&sim.world);
        assert_eq!(m.from(e), None, "stopped, it has no turn to round when it sets off again");
    }

    #[test]
    fn nothing_turns_while_paused() {
        let mut sim = rim_sim::Sim::new(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 1)
            .expect("the mods load");
        let e = sim.world.pawns[0];
        {
            let mut p = sim.world.ecs.get::<&mut Pawn>(e).unwrap();
            let pos = p.pos;
            p.next = Some(IVec::at(pos.x + 1, pos.y, pos.z));
            p.step_ticks = 100;
        }
        let (mut m, sites) = (Motion::default(), Worksites::default());
        m.face(&sim.world, &sites, 0.0);
        let east = std::f32::consts::FRAC_PI_2;
        assert!((m.facing(e).unwrap() - east).abs() < 1e-5, "a first sighting faces its way at once");
        // Now it steps north, but no game time passes: it keeps facing east.
        {
            let mut p = sim.world.ecs.get::<&mut Pawn>(e).unwrap();
            let pos = p.pos;
            p.next = Some(IVec::at(pos.x, pos.y - 1, pos.z));
        }
        m.face(&sim.world, &sites, 0.0);
        assert!((m.facing(e).unwrap() - east).abs() < 1e-5, "paused, it doesn't turn");
        // A tenth of a game second (six ticks) turns it 1.2 radians.
        m.face(&sim.world, &sites, 0.0);
        sim.world.tick += 6;
        m.face(&sim.world, &sites, 0.0);
        assert!((m.facing(e).unwrap() - (east - 1.2)).abs() < 1e-4, "{:?}", m.facing(e));
    }
}
