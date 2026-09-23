//! World state: the ECS, the map, the RNG and the clock.

use crate::defs::*;
use crate::field::Fields;
use crate::map::Map;
use crate::path::{Goal, Pathfinder};
use crate::rng::Rng;
use crate::{IVec, TICKS_PER_DAY};
use hecs::Entity;
use std::collections::HashMap;
use std::sync::Arc;

/// Needs are stored as integers in `0..=NEED_MAX`.
pub const NEED_MAX: i32 = 10_000;
pub const CARRY_CAPACITY: u32 = 75;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Faction {
    #[default]
    Wild,
    Player,
    Hostile,
}

impl Faction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wild" => Some(Faction::Wild),
            "player" => Some(Faction::Player),
            "hostile" => Some(Faction::Hostile),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Faction::Wild => "wild",
            Faction::Player => "player",
            Faction::Hostile => "hostile",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum Job {
    #[default]
    Idle,
    Wander {
        to: IVec,
        until: u64,
    },
    MoveTo {
        to: IVec,
    },
    Harvest {
        target: Entity,
        work: u32,
        /// Work the thing even though nobody designated it: foraging for
        /// food, or a job the player pointed at directly.
        forced: bool,
    },
    Deliver {
        bp: Entity,
        src: Entity,
        want: u32,
        stage: u8,
    },
    Construct {
        bp: Entity,
    },
    Eat {
        src: Entity,
        t: u32,
    },
    Sleep {
        bed: Option<Entity>,
        /// Where to lie down without a bed: somewhere warm if possible.
        spot: IVec,
        stage: u8,
    },
    /// Stand somewhere comfortable until a field need (warmth) recovers.
    Comfort {
        to: IVec,
        need: DefId,
        until: u64,
    },
    Attack {
        target: Entity,
        until: u64,
    },
    Flee {
        to: IVec,
        until: u64,
    },
    /// Walk off the map edge and leave the game.
    Leave {
        to: IVec,
    },
}

impl Job {
    pub fn label(&self) -> &'static str {
        match self {
            Job::Idle => "idle",
            Job::Wander { .. } => "wandering",
            Job::MoveTo { .. } => "moving",
            Job::Harvest { forced: true, .. } => "foraging",
            Job::Harvest { .. } => "harvesting",
            Job::Deliver { .. } => "hauling materials",
            Job::Construct { .. } => "building",
            Job::Eat { .. } => "eating",
            Job::Sleep { stage: 1, .. } => "sleeping",
            Job::Sleep { .. } => "going to sleep",
            Job::Comfort { .. } => "warming up",
            Job::Attack { .. } => "fighting",
            Job::Flee { .. } => "fleeing",
            Job::Leave { .. } => "leaving",
        }
    }
}

/// A creature. One component holds everything the AI touches each tick,
/// so the hot loop does one lookup per pawn.
#[derive(Clone, Debug, Default)]
pub struct Pawn {
    /// False for the placeholder left behind while the AI works on a pawn.
    pub active: bool,
    pub def: DefId,
    pub faction: Faction,
    pub name: String,
    pub founder: bool,
    pub pos: IVec,
    /// Cell currently being stepped into, with progress in ticks.
    pub next: Option<IVec>,
    pub progress: u32,
    pub step_ticks: u32,
    /// Remaining path as a stack (`last()` is the next step).
    pub path: Vec<IVec>,
    pub path_goal: Option<Goal>,
    pub repath_at: u64,
    pub hp: i32,
    pub needs: Vec<(DefId, i32)>,
    pub job: Job,
    pub next_think: u64,
    pub cooldown: u32,
    pub drafted: bool,
    pub carry: Option<(DefId, u32)>,
    pub last_attacker: Option<Entity>,
    pub asleep: bool,
    /// Rest-recovery multiplier in percent while asleep.
    pub sleep_rate: u32,
    pub dead: bool,
    /// Walked off the map; removed at the end of the tick.
    pub left: bool,
    /// Hostiles give up and head for the map edge at this tick.
    pub leave_at: Option<u64>,
}

impl Pawn {
    pub fn moving(&self) -> bool {
        self.next.is_some() || !self.path.is_empty()
    }
    pub fn need(&self, need: DefId) -> Option<i32> {
        self.needs.iter().find(|n| n.0 == need).map(|n| n.1)
    }
}

/// A plant, rock, building, blueprint or item stack.
#[derive(Clone, Debug)]
pub struct Thing {
    pub def: DefId,
    pub pos: IVec,
    pub count: u32,
    pub hp: i32,
}

/// Present on a fixture that is still under construction.
#[derive(Clone, Debug)]
pub struct Blueprint {
    pub delivered: Vec<u32>,
    pub work_left: u32,
}

/// Player has marked this thing or creature for work.
#[derive(Clone, Copy, Debug)]
pub struct Designated(pub DefId);

/// Harvested; will be harvestable again at `ready_at`.
#[derive(Clone, Copy, Debug)]
pub struct Regrow {
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MsgKind {
    Info,
    Good,
    Threat,
    Bad,
}

impl MsgKind {
    pub fn parse(s: &str) -> Self {
        match s {
            "good" => MsgKind::Good,
            "threat" => MsgKind::Threat,
            "bad" => MsgKind::Bad,
            _ => MsgKind::Info,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub tick: u64,
    pub text: String,
    pub kind: MsgKind,
}

/// Things scripts can listen to with `rim.on(name, fn)`.
#[derive(Clone, Debug)]
pub enum GameEvent {
    PawnJoined { id: Entity, name: String, def: DefId },
    PawnDied { id: Entity, name: String, def: DefId, faction: Faction, pos: IVec },
    PawnLeft { id: Entity, name: String, def: DefId, faction: Faction },
    BuildingComplete { id: Entity, def: DefId, pos: IVec },
    NewDay { day: u64 },
    ColonyLost,
}

pub struct World {
    pub defs: Arc<DefDb>,
    pub seed: u64,
    pub ecs: hecs::World,
    pub map: Map,
    /// Temperature, light and other field layers over the map.
    pub fields: Fields,
    pub rng: Rng,
    pub tick: u64,
    /// Pawns in spawn order; iteration order is part of determinism.
    pub pawns: Vec<Entity>,
    /// target -> pawn holding it.
    pub reservations: HashMap<Entity, Entity>,
    pub messages: Vec<Message>,
    pub events: Vec<GameEvent>,
    pub pf: Pathfinder,
    /// Cached; recomputed every few hundred ticks.
    pub wealth: f64,
    /// Recent melee hits (pos, tick) for renderers.
    pub hits: Vec<(IVec, u64)>,
    pub colony_lost: bool,
}

impl World {
    pub fn new(defs: Arc<DefDb>, w: i32, h: i32, seed: u64) -> Self {
        let fields = Fields::new(&defs, (w * h) as usize);
        World {
            defs,
            seed,
            ecs: hecs::World::new(),
            fields,
            map: Map::new(w, h),
            rng: Rng::new(seed),
            tick: 0,
            pawns: Vec::new(),
            reservations: HashMap::new(),
            messages: Vec::new(),
            events: Vec::new(),
            pf: Pathfinder::default(),
            wealth: 0.0,
            hits: Vec::new(),
            colony_lost: false,
        }
    }

    // ------------------------------------------------------------ time

    pub fn day(&self) -> u64 {
        self.tick / TICKS_PER_DAY
    }
    /// Hour of day, 0..24. Tick 0 is 06:00.
    pub fn hour(&self) -> f64 {
        ((self.tick + TICKS_PER_DAY / 4) % TICKS_PER_DAY) as f64 / TICKS_PER_DAY as f64 * 24.0
    }
    pub fn is_night(&self) -> bool {
        let h = self.hour();
        !(6.0..21.0).contains(&h)
    }

    pub fn message(&mut self, text: impl Into<String>, kind: MsgKind) {
        self.messages.push(Message { tick: self.tick, text: text.into(), kind });
    }

    // ------------------------------------------------------------ pawns

    pub fn spawn_pawn(&mut self, def: DefId, faction: Faction, pos: IVec, name: Option<String>) -> Entity {
        let defs = self.defs.clone();
        let cd = defs.creature(def);
        let name = match name {
            Some(n) => n,
            None if cd.intelligent && !defs.names.is_empty() => {
                // Avoid names already in use, if the pool allows.
                let taken: Vec<String> =
                    self.pawns.iter().filter_map(|&e| self.ecs.get::<&Pawn>(e).ok().map(|p| p.name.clone())).collect();
                let mut pick = String::new();
                for _ in 0..16 {
                    pick = defs.names[self.rng.below(defs.names.len() as u32) as usize].clone();
                    if !taken.contains(&pick) {
                        break;
                    }
                }
                pick
            }
            None => cd.label.clone(),
        };
        let p = Pawn {
            active: true,
            def,
            faction,
            name: name.clone(),
            pos,
            hp: cd.max_hp,
            needs: cd.needs_r.iter().map(|&n| (n, NEED_MAX * 8 / 10)).collect(),
            next_think: self.tick + self.rng.below(30) as u64,
            ..Default::default()
        };
        let e = self.ecs.spawn((p,));
        self.pawns.push(e);
        if faction == Faction::Player {
            self.events.push(GameEvent::PawnJoined { id: e, name, def });
        }
        e
    }

    pub fn pawn_alive(&self, e: Entity) -> bool {
        self.ecs.get::<&Pawn>(e).map(|p| p.active && !p.dead).unwrap_or(false)
    }

    pub fn pawn_pos(&self, e: Entity) -> Option<IVec> {
        self.ecs.get::<&Pawn>(e).ok().filter(|p| p.active && !p.dead).map(|p| p.pos)
    }

    pub fn colonists(&self) -> impl Iterator<Item = Entity> + '_ {
        self.pawns.iter().copied().filter(|&e| {
            self.ecs.get::<&Pawn>(e).map(|p| p.active && !p.dead && p.faction == Faction::Player).unwrap_or(false)
        })
    }

    pub fn colony_center(&self) -> Option<IVec> {
        let (mut sx, mut sy, mut n) = (0i64, 0i64, 0i64);
        for e in self.colonists() {
            if let Some(p) = self.pawn_pos(e) {
                sx += p.x as i64;
                sy += p.y as i64;
                n += 1;
            }
        }
        (n > 0).then(|| IVec::new((sx / n) as i32, (sy / n) as i32))
    }

    // ------------------------------------------------------------ things

    /// Place a plant/rock/building (or its blueprint) on the fixture layer.
    pub fn spawn_fixture(&mut self, def: DefId, pos: IVec, blueprint: bool) -> Option<Entity> {
        if !self.map.inb(pos) || self.map.fixture_at(pos).is_some() {
            return None;
        }
        let defs = self.defs.clone();
        let td = defs.thing(def);
        let t = Thing { def, pos, count: 1, hp: td.hp as i32 };
        let e = if blueprint {
            let b = td.build.as_ref()?;
            self.ecs.spawn((t, Blueprint { delivered: vec![0; b.cost_r.len()], work_left: b.work.max(1) }))
        } else {
            self.ecs.spawn((t,))
        };
        let (blocks, cost, door) = if blueprint { (false, 0, false) } else { (td.blocks, td.path_cost, td.door) };
        self.map.set_fixture(pos, Some(e), blocks, cost, door);
        if !blueprint {
            self.fields.add_emitters(&defs, &self.map, e, def, pos);
        }
        Some(e)
    }

    /// Drop items near `near`, merging into existing stacks. Returns the
    /// amount that could not be placed.
    pub fn place_item(&mut self, def: DefId, near: IVec, mut count: u32) -> u32 {
        let limit = self.defs.thing(def).stack_limit;
        for r in 0..=8i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if count == 0 {
                        return 0;
                    }
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let p = near.offset(dx, dy);
                    if !self.map.passable(p) {
                        continue;
                    }
                    let i = self.map.idx(p);
                    match self.map.item[i] {
                        None => {
                            let n = count.min(limit);
                            let e = self.ecs.spawn((Thing { def, pos: p, count: n, hp: 100 },));
                            self.map.item[i] = Some(e);
                            count -= n;
                        }
                        Some(e) => {
                            if let Ok(mut t) = self.ecs.get::<&mut Thing>(e) {
                                if t.def == def && t.count < limit {
                                    let n = count.min(limit - t.count);
                                    t.count += n;
                                    count -= n;
                                }
                            }
                        }
                    }
                }
            }
        }
        count
    }

    pub fn despawn_thing(&mut self, e: Entity) {
        let Ok(t) = self.ecs.get::<&Thing>(e).map(|t| (*t).clone()) else { return };
        if self.map.inb(t.pos) {
            let i = self.map.idx(t.pos);
            if self.map.item[i] == Some(e) {
                self.map.item[i] = None;
            }
            if self.map.fixture[i] == Some(e) {
                self.map.set_fixture(t.pos, None, false, 0, false);
            }
        }
        self.reservations.remove(&e);
        self.fields.remove_emitters(e);
        let _ = self.ecs.despawn(e);
    }

    /// Take up to `n` from a stack, despawning it when empty.
    pub fn take_from_stack(&mut self, e: Entity, n: u32) -> u32 {
        let (taken, empty) = match self.ecs.get::<&mut Thing>(e) {
            Ok(mut t) => {
                let k = n.min(t.count);
                t.count -= k;
                (k, t.count == 0)
            }
            Err(_) => return 0,
        };
        if empty {
            self.despawn_thing(e);
        }
        taken
    }

    pub fn thing(&self, e: Entity) -> Option<Thing> {
        self.ecs.get::<&Thing>(e).ok().map(|t| (*t).clone())
    }

    // ------------------------------------------------------------ reservations

    /// Reserve `target` for `by`. False if someone else holds it.
    pub fn reserve(&mut self, target: Entity, by: Entity) -> bool {
        match self.reservations.get(&target) {
            Some(&o) => o == by,
            None => {
                self.reservations.insert(target, by);
                true
            }
        }
    }
    pub fn reserved_by_other(&self, target: Entity, by: Entity) -> bool {
        self.reservations.get(&target).is_some_and(|&o| o != by)
    }
    pub fn release_all(&mut self, by: Entity) {
        self.reservations.retain(|_, v| *v != by);
    }

    /// A fingerprint of simulation state, for determinism tests and
    /// (later) multiplayer desync detection.
    pub fn state_hash(&self) -> u64 {
        let mut h = crate::rng::mix(self.tick ^ self.rng.state());
        for &e in &self.pawns {
            if let Ok(p) = self.ecs.get::<&Pawn>(e) {
                h = crate::rng::mix(h ^ ((p.pos.x as u64) << 32 | p.pos.y as u32 as u64) ^ (p.hp as u64) << 48);
            }
        }
        for (_, t) in self.ecs.query::<&Thing>().iter() {
            // Order-independent combine for things.
            h = h.wrapping_add(crate::rng::mix(
                (t.def as u64) << 40 ^ (t.pos.x as u64) << 20 ^ t.pos.y as u64 ^ (t.count as u64) << 56,
            ));
        }
        h
    }
}
