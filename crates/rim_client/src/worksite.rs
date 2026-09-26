//! The moving part of a worksite (DESIGN.md §6b): each blow and each finish.
//!
//! The sim sends no events for these. A strike is the client seeing a
//! site's `done` cross a multiple of its style's `every` (or its hp drop)
//! since the last frame, so a faster worker strikes faster and a paused
//! game freezes mid-swing: every clock here is the sim's tick. An exit is a
//! site the client was following that finished or disappeared. Randomness
//! is a hash of the entity and the strike number, so a replay throws the
//! same chips. Particles live in one pool allocated up front.

use crate::rgb;
use macroquad::prelude::Color;
use rim_sim::defs::{DefId, Exit, Strike, WorkStyleDef};
use rim_sim::hecs::Entity;
use rim_sim::rng::hash2_f;
use rim_sim::world::{Blueprint, Job, MadeOf, Pawn, Thing, Work, World};
use rim_sim::IVec;
use std::collections::BTreeMap;

/// Below this many points a cell, a blow is a flash of the cell's outline
/// and nothing moves.
pub const DETAIL_ZOOM: f32 = 20.0;
const POOL: usize = 1500;
const PER_SITE: usize = 48;
/// At most this many strikes' effects a frame per site, however many
/// crossings a fast-forwarded frame saw.
const BURST: u32 = 2;
const FALL_TICKS: u64 = 40;
const CRUMBLE_TICKS: u64 = 30;
const SETTLE_TICKS: u64 = 20;
const GRAVITY: f32 = 0.004;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Chip,
    Dust,
    Leaf,
}

#[derive(Clone, Copy)]
pub struct Particle {
    pub kind: Kind,
    /// In cells; `h` is height above the ground.
    pub x: f32,
    pub y: f32,
    pub h: f32,
    vx: f32,
    vy: f32,
    vh: f32,
    pub age: u16,
    pub life: u16,
    pub size: f32,
    pub color: Color,
    bounced: bool,
    /// The site that threw it (its entity id), for the per-site cap.
    site: u32,
}

const NONE: Particle = Particle {
    kind: Kind::Dust,
    x: 0.0,
    y: 0.0,
    h: 0.0,
    vx: 0.0,
    vy: 0.0,
    vh: 0.0,
    age: 0,
    life: 0,
    size: 0.0,
    color: Color::new(0.0, 0.0, 0.0, 0.0),
    bounced: false,
    site: u32::MAX,
};

/// A site being followed: what it looked like last frame.
struct Site {
    cell: IVec,
    /// Its footprint, in cells from `cell` right and down.
    size: (f32, f32),
    def: DefId,
    own: Color,
    chip: Color,
    style: Option<DefId>,
    /// Toward the last worker, one step.
    toward: (f32, f32),
    done: u32,
    total: u32,
    /// Work a tick, as last seen: skill and tools set it, so it is measured
    /// rather than assumed.
    pace: f32,
    /// The tick `done` was last read at.
    seen: u64,
    hp: i32,
    plan: bool,
    strikes: u32,
    struck: Option<u64>,
    settled: Option<u64>,
}

pub struct Leaving {
    pub kind: Exit,
    pub cell: IVec,
    pub def: DefId,
    pub own: Color,
    /// The way it goes: a fall's direction, or away from the worked side.
    pub way: (f32, f32),
    pub start: u64,
    burst: bool,
}

/// How a live site is drawn this frame, on top of its stage.
#[derive(Clone, Copy)]
pub struct Tone {
    /// Offset in cells.
    pub shift: (f32, f32),
    pub bright: f32,
    /// About the middle of the cell's bottom edge.
    pub scale: f32,
}

impl Default for Tone {
    fn default() -> Self {
        Tone { shift: (0.0, 0.0), bright: 1.0, scale: 1.0 }
    }
}

pub struct Worksites {
    sites: BTreeMap<Entity, Site>,
    pub parts: Vec<Particle>,
    pub leaving: Vec<Leaving>,
    tick: u64,
}

impl Default for Worksites {
    fn default() -> Self {
        Worksites { sites: BTreeMap::new(), parts: Vec::with_capacity(POOL), leaving: Vec::new(), tick: 0 }
    }
}

fn style(w: &World, s: Option<DefId>) -> Option<&WorkStyleDef> {
    s.map(|s| &w.defs.work_styles[s as usize])
}

/// The style of `e`'s work: a plan's is the one builds use.
fn style_of(w: &World, e: Entity, t: &Thing) -> Option<DefId> {
    if w.ecs.get::<&Blueprint>(e).is_ok() {
        return w.defs.build_style;
    }
    crate::wear::style_id(w, e, t)
}

impl Worksites {
    /// Follow the sim to its current tick: move particles, see strikes and
    /// finishes. `detail` is false when zoomed out too far to see them.
    pub fn update(&mut self, w: &World, detail: bool) {
        let steps = w.tick.saturating_sub(self.tick).min(20);
        self.tick = w.tick;
        for _ in 0..steps {
            self.step_particles();
        }
        let now = w.tick;
        for (&e, &(cell, _)) in &w.worksites {
            let Some(t) = w.thing(e) else { continue };
            let work = w.ecs.get::<&Work>(e).ok().map(|k| *k);
            let plan = w.ecs.get::<&Blueprint>(e).is_ok();
            let (done, total) = work.map_or((0, 1), |k| (k.done, k.total));
            let fresh = !self.sites.contains_key(&e);
            if fresh {
                let td = w.defs.thing(t.def);
                let own = match w.ecs.get::<&MadeOf>(e) {
                    Ok(m) => rgb(w.defs.thing(m.0).rgb),
                    Err(_) => rgb(td.rgb),
                };
                let chip = match td.harvest.first().and_then(|h| h.yields_r.first()) {
                    Some(&(y, _)) if !plan && w.ecs.get::<&MadeOf>(e).is_err() => rgb(w.defs.thing(y).rgb),
                    _ => own,
                };
                let site = Site {
                    cell,
                    size: (td.size[0] as f32, td.size[1] as f32),
                    def: t.def,
                    own,
                    chip,
                    style: style_of(w, e, &t),
                    toward: crate::wear::toward(w, e),
                    done,
                    total,
                    pace: 1.0,
                    seen: now,
                    hp: t.hp,
                    plan,
                    strikes: 0,
                    struck: None,
                    settled: None,
                };
                self.sites.insert(e, site);
                continue;
            }
            let Some(mut site) = self.sites.remove(&e) else { continue };
            site.toward = crate::wear::toward(w, e);
            site.style = style_of(w, e, &t).or(site.style);
            let every = style(w, site.style).map_or(u32::MAX, |s| s.every.max(1));
            let crossed = if work.is_some() && done > site.done { done / every - site.done / every } else { 0 };
            let hurt = u32::from(t.hp < site.hp);
            for _ in 0..(crossed + hurt).min(BURST) {
                site.strikes += 1;
                site.struck = Some(now);
                if detail {
                    self.strike(w, e, &mut site);
                }
            }
            if site.plan && !plan {
                site.settled = Some(now);
                if detail {
                    self.settle_dust(&site);
                }
            }
            let ticks = now.saturating_sub(site.seen);
            if ticks > 0 {
                if done > site.done {
                    site.pace = (done - site.done) as f32 / ticks as f32;
                }
                site.seen = now;
            }
            (site.done, site.total, site.hp, site.plan) = (done, total.max(1), t.hp, plan);
            self.sites.insert(e, site);
        }
        // Sites no longer worked: gone means finished (or broken), so it
        // leaves; still standing means the worker stopped, and it goes
        // back to the cache as it is.
        let quiet: Vec<Entity> = self.sites.keys().filter(|e| !w.worksites.contains_key(e)).copied().collect();
        for e in quiet {
            let site = self.sites.remove(&e).expect("listed");
            if w.thing(e).is_some() {
                continue;
            }
            let kind = match style(w, site.style).map(|s| s.exit) {
                Some(k) if k != Exit::None => k,
                // Broken by a raider rather than taken down: it breaks up.
                _ if site.done < site.total => Exit::Crumble,
                _ => continue,
            };
            let way = match kind {
                Exit::Fall => crate::wear::fall_way(w, site.cell, site.toward),
                _ => (-site.toward.0, -site.toward.1),
            };
            if self.leaving.len() < 64 {
                self.leaving.push(Leaving {
                    kind,
                    cell: site.cell,
                    def: site.def,
                    own: site.own,
                    way,
                    start: now,
                    burst: false,
                });
            }
            if detail && kind != Exit::Fall {
                self.burst(e, site.cell, site.chip, 10, 4);
            }
        }
        // A fall lands at its end: leaves and dust where the crown hits.
        let mut landed = Vec::new();
        self.leaving.retain_mut(|l| {
            let age = now.saturating_sub(l.start);
            if l.kind == Exit::Fall && age >= FALL_TICKS && !l.burst {
                l.burst = true;
                landed.push((l.cell, l.way, l.own));
            }
            age < FALL_TICKS.max(CRUMBLE_TICKS) + 2
        });
        for (cell, way, own) in landed.into_iter().filter(|_| detail) {
            let at = (cell.x as f32 + 0.5 + way.0 * 1.45, cell.y as f32 + 0.5 + way.1 * 1.45);
            let seed = (cell.x as i64) << 20 | cell.y as i64;
            for i in 0..14 {
                let r = |k: u64| hash2_f(seed, i, 70 + k) as f32 - 0.5;
                self.spawn(Particle {
                    kind: Kind::Leaf,
                    x: at.0 + r(0) * 0.9,
                    y: at.1 + r(1) * 0.9,
                    h: 0.4 + r(2) * 0.3,
                    vx: r(3) * 0.01,
                    vy: r(4) * 0.006,
                    vh: -0.008,
                    life: 120,
                    size: 0.07,
                    color: shade(own, 0.95 + r(5) * 0.5),
                    ..NONE
                });
            }
            self.dust(at, 3, seed);
        }
    }

    /// The effects of one blow on `e`, from its style, thrown toward the worker.
    fn strike(&mut self, w: &World, e: Entity, site: &mut Site) {
        let Some(st) = style(w, site.style) else { return };
        if self.parts.iter().filter(|p| p.site == e.id()).count() >= PER_SITE {
            return;
        }
        let seed = e.id() as i64 * 4099 + site.strikes as i64;
        let r = |i: i64, k: u64| hash2_f(seed, i, k) as f32;
        let (tx, ty) = site.toward;
        let (hw, hh) = (site.size.0 / 2.0, site.size.1 / 2.0);
        let c = (site.cell.x as f32 + hw, site.cell.y as f32 + hh);
        // Where the blow lands: the worked face, or the top of a rising plan.
        let hit = if site.plan {
            (c.0 + (r(0, 1) - 0.5) * 0.6 * site.size.0, c.1 + hh - site.size.1 * site.done as f32 / site.total as f32)
        } else {
            (c.0 + tx * (hw - 0.08), c.1 + ty * (hh - 0.08))
        };
        let before = self.parts.len();
        for s in &st.strike {
            match s {
                Strike::Chips => {
                    for i in 0..5 {
                        let (fast, spread) = (0.015 + r(i, 2) * 0.03, (r(i, 3) - 0.5) * 0.045);
                        self.spawn(Particle {
                            kind: Kind::Chip,
                            x: hit.0,
                            y: hit.1,
                            h: 0.25,
                            vx: tx * fast - ty * spread,
                            vy: ty * fast + tx * spread,
                            vh: 0.025 + r(i, 4) * 0.03,
                            life: 90 + (r(i, 5) * 40.0) as u16,
                            size: 0.045 + r(i, 6) * 0.04,
                            color: shade(site.chip, 0.8 + r(i, 7) * 0.45),
                            ..NONE
                        });
                    }
                }
                Strike::Dust => self.dust((hit.0 + tx * 0.1, hit.1 + ty * 0.1), 2, seed),
                Strike::Shed => {
                    for i in 0..3 {
                        self.spawn(Particle {
                            kind: Kind::Leaf,
                            x: c.0 + (r(i, 8) - 0.5) * 0.6,
                            y: c.1 + (r(i, 9) - 0.5) * 0.6,
                            h: 0.5 + r(i, 10) * 0.3,
                            vx: (r(i, 11) - 0.5) * 0.01,
                            vy: (r(i, 12) - 0.5) * 0.006,
                            vh: -0.006 - r(i, 13) * 0.004,
                            life: 110 + (r(i, 14) * 50.0) as u16,
                            size: 0.06 + r(i, 15) * 0.03,
                            color: shade(site.own, 0.9 + r(i, 16) * 0.5),
                            ..NONE
                        });
                    }
                }
                Strike::Shake => {}
            }
        }
        for p in &mut self.parts[before..] {
            p.site = e.id();
        }
    }

    fn dust(&mut self, (x, y): (f32, f32), n: i64, seed: i64) {
        for i in 0..n {
            let r = |k: u64| hash2_f(seed, i, 40 + k) as f32 - 0.5;
            self.spawn(Particle {
                kind: Kind::Dust,
                x: x + r(0) * 0.3,
                y: y + r(1) * 0.3,
                vx: r(2) * 0.006,
                vy: r(3) * 0.006,
                life: 40 + (r(4) * 30.0) as u16,
                size: 0.2 + r(5) * 0.12,
                ..NONE
            });
        }
    }

    fn burst(&mut self, e: Entity, cell: IVec, c: Color, chips: i64, dust: i64) {
        let seed = e.id() as i64 * 7919;
        let at = (cell.x as f32 + 0.5, cell.y as f32 + 0.5);
        for i in 0..chips {
            let r = |k: u64| hash2_f(seed, i, 20 + k) as f32;
            let (a, fast) = (r(0) * std::f32::consts::TAU, 0.02 + r(1) * 0.03);
            self.spawn(Particle {
                kind: Kind::Chip,
                x: at.0,
                y: at.1,
                h: 0.3,
                vx: a.cos() * fast,
                vy: a.sin() * fast,
                vh: 0.03 + r(2) * 0.03,
                life: 100,
                size: 0.05 + r(3) * 0.05,
                color: shade(c, 0.8 + r(4) * 0.45),
                ..NONE
            });
        }
        self.dust(at, dust, seed);
    }

    fn settle_dust(&mut self, site: &Site) {
        let (x, y) = (site.cell.x as f32, site.cell.y as f32 + 0.95);
        for i in 0..6 {
            let r = |k: u64| hash2_f(site.cell.x as i64, site.cell.y as i64 * 8 + i, 90 + k) as f32 - 0.5;
            self.spawn(Particle {
                kind: Kind::Dust,
                x: x + 0.1 + i as f32 * 0.16,
                y: y + r(0) * 0.1,
                vx: (i as f32 - 2.5) * 0.004,
                vy: 0.002,
                life: 50,
                size: 0.15 + r(1) * 0.08,
                ..NONE
            });
        }
    }

    fn spawn(&mut self, p: Particle) {
        if self.parts.len() < POOL {
            self.parts.push(p);
        }
    }

    fn step_particles(&mut self) {
        let mut i = 0;
        while i < self.parts.len() {
            let p = &mut self.parts[i];
            p.age += 1;
            if p.age >= p.life {
                // Order doesn't matter to a particle; this keeps the pool
                // from shuffling.
                self.parts.swap_remove(i);
                continue;
            }
            match p.kind {
                Kind::Dust => {
                    p.x += p.vx;
                    p.y += p.vy;
                }
                _ if p.h > 0.0 || p.vh > 0.0 => {
                    p.x += p.vx;
                    p.y += p.vy;
                    p.h += p.vh;
                    if p.kind == Kind::Chip {
                        p.vh -= GRAVITY;
                    } else {
                        p.x += (p.age as f32 * 0.15 + p.size * 90.0).sin() * 0.004;
                    }
                    if p.h <= 0.0 {
                        p.h = 0.0;
                        if p.kind == Kind::Chip && !p.bounced && p.vh < -0.01 {
                            (p.vh, p.vx, p.vy, p.bounced) = (-p.vh * 0.35, p.vx * 0.45, p.vy * 0.45, true);
                        } else {
                            p.vh = 0.0;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// How a live site's look moves this frame: the shake of the last blow,
    /// the flash of its impact, the settle of a build that just stood.
    pub fn tone(&self, w: &World, e: Entity, detail: bool) -> Tone {
        let Some(site) = self.sites.get(&e).filter(|_| detail) else { return Tone::default() };
        let mut tone = Tone::default();
        if let Some(at) = site.struck {
            let a = self.tick.saturating_sub(at) as f32;
            let shakes = style(w, site.style).is_some_and(|s| s.strike.contains(&Strike::Shake));
            if shakes && a < 16.0 {
                let amp = if site.plan { 0.0 } else { 0.03 };
                let s = amp * (-a / 4.0).exp() * (a * 1.6).cos();
                tone.shift = (-site.toward.0 * s, -site.toward.1 * s);
            }
            if a < 4.0 {
                tone.bright = 1.0 + 0.22 * (1.0 - a / 4.0);
            }
        }
        if let Some(at) = site.settled {
            let a = self.tick.saturating_sub(at) as f32;
            if a < SETTLE_TICKS as f32 {
                tone.scale = 1.0 + 0.08 * (-a / 3.5).exp() * (a * 0.9).cos();
            }
        }
        tone
    }

    /// Ticks since `e` was last struck, if it is followed and has been.
    pub fn since_strike(&self, e: Entity) -> Option<u64> {
        self.sites.get(&e).and_then(|s| s.struck).map(|t| self.tick.saturating_sub(t))
    }

    /// Where a working pawn is drawn, in cells, relative to where it
    /// stands: winding up as the next blow nears, lunging into it, easing
    /// back. The next blow is the next multiple of `every`, or the end of a
    /// swing's cooldown.
    pub fn lunge(&self, w: &World, p: &Pawn) -> (f32, f32) {
        let target = match p.job {
            Job::Harvest { target, .. } | Job::Deconstruct { target } | Job::Breach { target } => target,
            Job::Construct { bp } => bp,
            _ => return (0.0, 0.0),
        };
        let Some(site) = self.sites.get(&target) else { return (0.0, 0.0) };
        let every = style(w, site.style).map_or(0, |s| s.every);
        let a = site.struck.map_or(u64::MAX, |t| self.tick.saturating_sub(t));
        let reach = if a < 8 {
            0.17 * (1.0 - a as f32 / 8.0).powi(2)
        } else {
            let next = if w.ecs.get::<&Work>(target).is_ok() && every > 0 {
                ticks_to_strike(site.done, every, site.pace)
            } else {
                p.cooldown
            };
            if next <= 6 {
                -0.07 * (1.0 - next as f32 / 6.0).powi(2)
            } else {
                0.0
            }
        };
        (-site.toward.0 * reach, -site.toward.1 * reach)
    }

    /// Age of a leaving thing, 0 to 1 through its exit.
    pub fn exit_age(&self, l: &Leaving) -> f32 {
        let span = match l.kind {
            Exit::Fall => FALL_TICKS,
            _ => CRUMBLE_TICKS,
        };
        self.tick.saturating_sub(l.start) as f32 / span as f32
    }

    /// Work a tick on `e`, as last seen.
    pub fn pace(&self, e: Entity) -> f32 {
        self.sites.get(&e).map_or(1.0, |s| s.pace)
    }

    /// Progress of a live site as the readout shows it, and whether it is
    /// damage (hp) rather than work.
    pub fn readout(&self, w: &World, e: Entity) -> Option<(IVec, f32, bool)> {
        let site = self.sites.get(&e)?;
        if w.ecs.get::<&Work>(e).is_ok() {
            return Some((site.cell, site.done as f32 / site.total.max(1) as f32, false));
        }
        let max = w.stat(e, "hp")?;
        Some((site.cell, (site.hp as f32 / max as f32).clamp(0.0, 1.0), true))
    }

    pub fn sites(&self) -> impl Iterator<Item = Entity> + '_ {
        self.sites.keys().copied()
    }
}

/// Ticks until work at `pace` a tick next crosses a multiple of `every`.
pub fn ticks_to_strike(done: u32, every: u32, pace: f32) -> u32 {
    ((every - done % every) as f32 / pace.max(0.01)).ceil() as u32
}

fn shade(c: Color, k: f32) -> Color {
    Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), c.a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_sim::{Command, Sim};

    /// The founder alone, told to fell the nearest oak it can reach. Core
    /// alone, whose oaks need no axe.
    fn chopping(seed: u64) -> (Sim, Entity, u32) {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let mut s = Sim::with_mods(&mods, seed, &|m| m == "core").unwrap();
        s.step();
        let founder = s.world.colonists().next().unwrap();
        for e in s.world.pawns.clone() {
            if e != founder {
                let _ = s.world.ecs.despawn(e);
            }
        }
        s.world.pawns.retain(|&e| e == founder);
        let oak = s.world.defs.thing_id("tree_oak").unwrap();
        let from = s.world.pawn_pos(founder).unwrap();
        s.world.map.ensure_regions();
        let (tree, at) = s
            .world
            .ecs
            .query::<(Entity, &Thing)>()
            .iter()
            .filter(|(_, t)| t.def == oak && s.world.map.can_reach(from, rim_sim::path::Goal::Touch(t.pos)))
            .min_by_key(|(e, t)| (t.pos.octile(from), e.id()))
            .map(|(e, t)| (e, t.pos))
            .unwrap();
        let fell = s.world.defs.thing(oak).harvest.iter().find(|h| h.destroy).unwrap();
        let every =
            s.world.defs.work_styles[s.world.defs.designations[fell.desig_r as usize].style_r.unwrap() as usize].every;
        s.push(Command::Designate { designation: fell.desig_r, a: at, b: at });
        (s, tree, every)
    }

    /// Run until the tree is down, following it every tick as a 60 fps
    /// client at 1x would.
    fn fell(s: &mut Sim, ws: &mut Worksites, tree: Entity, detail: bool) -> (u32, usize) {
        let (mut done, mut most) = (0, 0);
        for _ in 0..20_000 {
            s.step();
            ws.update(&s.world, detail);
            most = most.max(ws.parts.len());
            if let Ok(k) = s.world.ecs.get::<&Work>(tree) {
                done = k.done;
            }
            if s.world.thing(tree).is_none() {
                return (done, most);
            }
        }
        panic!("the tree never came down");
    }

    #[test]
    fn a_strike_lands_each_time_the_work_crosses_every() {
        let (mut s, tree, every) = chopping(21);
        let mut ws = Worksites::default();
        let mut strikes = 0;
        for _ in 0..20_000 {
            s.step();
            ws.update(&s.world, true);
            if let Some(site) = ws.sites.get(&tree) {
                strikes = site.strikes;
            }
            if s.world.thing(tree).is_none() {
                break;
            }
        }
        let total = s
            .world
            .defs
            .thing(s.world.defs.thing_id("tree_oak").unwrap())
            .harvest
            .iter()
            .find(|h| h.destroy)
            .unwrap()
            .work;
        // The last blow fells it, and the site is gone before it is counted.
        assert!(strikes >= total / every - 1 && strikes <= total / every, "{strikes} strikes for {total} work");
    }

    #[test]
    fn the_same_game_throws_the_same_chips() {
        let run = || {
            let (mut s, _, _) = chopping(21);
            let mut ws = Worksites::default();
            let mut after = None;
            for tick in 0..20_000 {
                s.step();
                ws.update(&s.world, true);
                after = after.or((!ws.parts.is_empty()).then_some(tick + 45));
                if after == Some(tick) {
                    break;
                }
            }
            ws.parts.iter().map(|p| (p.x.to_bits(), p.y.to_bits(), p.h.to_bits())).collect::<Vec<_>>()
        };
        let (a, b) = (run(), run());
        assert!(!a.is_empty(), "something was thrown");
        assert_eq!(a, b);
    }

    #[test]
    fn a_skilled_worker_winds_up_for_the_blow_that_is_coming() {
        assert_eq!(ticks_to_strike(0, 30, 1.0), 30);
        assert_eq!(ticks_to_strike(0, 30, 2.0), 15, "twice the pace, half the wait");
        assert_eq!(ticks_to_strike(29, 30, 2.0), 1);
        assert_eq!(ticks_to_strike(10, 30, 0.5), 40);
    }

    #[test]
    fn a_sites_pace_is_measured_not_assumed() {
        let (mut s, _, _) = chopping(21);
        let wall = s.world.defs.thing_id("wall").unwrap();
        let stuff = s.world.defs.materials("structural").first().copied();
        let at = s.world.colony_center().unwrap().offset(3, 3);
        let plan = s.world.spawn_fixture_of(wall, at, true, stuff).expect("a plan");
        let mut ws = Worksites::default();
        // Two work a tick, read every frame at 1x, then every third tick at 3x.
        for step in [1, 1, 1, 3, 3] {
            s.world.tick += step;
            s.world.ecs.get::<&mut Work>(plan).unwrap().done += 2 * step as u32;
            s.world.mark_worksite(plan, at);
            ws.update(&s.world, true);
        }
        assert_eq!(ws.pace(plan), 2.0);
    }

    #[test]
    fn the_pool_never_grows() {
        let (mut s, tree, _) = chopping(21);
        let mut ws = Worksites::default();
        let (_, most) = fell(&mut s, &mut ws, tree, true);
        assert!(most > 0 && most <= POOL);
        assert_eq!(ws.parts.capacity(), POOL, "allocated once, never again");
    }

    #[test]
    fn a_felled_tree_falls() {
        let (mut s, tree, _) = chopping(21);
        let mut ws = Worksites::default();
        fell(&mut s, &mut ws, tree, true);
        let l = ws.leaving.iter().find(|l| l.kind == Exit::Fall).expect("it falls");
        let cutter = s.world.pawn_pos(s.world.pawns[0]).unwrap();
        let (dx, dy) = ((cutter.x - l.cell.x) as f32, (cutter.y - l.cell.y) as f32);
        assert_eq!(l.way.0.abs() + l.way.1.abs(), 1.0, "along one axis: {:?}", l.way);
        assert!(l.way.0 * dx + l.way.1 * dy <= 0.0, "never onto the cutter: {:?} from {dx},{dy}", l.way);
    }

    #[test]
    fn zoomed_out_nothing_is_thrown() {
        let (mut s, tree, _) = chopping(21);
        let mut ws = Worksites::default();
        let (_, most) = fell(&mut s, &mut ws, tree, false);
        assert_eq!(most, 0);
    }

    #[test]
    fn a_fall_goes_around_what_stands_in_its_way() {
        let (mut s, tree, _) = chopping(21);
        let cell = s.world.thing(tree).unwrap().pos;
        let rock = s.world.defs.thing_id("granite").unwrap();
        // Clear the way west, then block it: the fall turns aside.
        let west = [cell.offset(-1, 0), cell.offset(-2, 0)];
        let open =
            |s: &Sim, q: IVec| s.world.map.inb(q) && s.world.map.passable(q) && s.world.map.fixture_at(q).is_none();
        if west.iter().all(|&q| open(&s, q)) {
            assert_eq!(
                crate::wear::fall_way(&s.world, cell, (1.0, 0.0)),
                (-1.0, 0.0),
                "away from a worker in the east"
            );
        }
        let _ = s.world.spawn_fixture(rock, west[1], false);
        let way = crate::wear::fall_way(&s.world, cell, (1.0, 0.0));
        assert_ne!(way, (-1.0, 0.0), "not into the rock");
    }
}
