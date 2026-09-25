//! World state: the ECS, the map, the RNG and the clock.

use crate::data::Data;
use crate::defs::*;
use crate::field::{Clock, Fields};
use crate::map::Map;
use crate::path::{Goal, Pathfinder};
use crate::rng::Rng;
use crate::terms::Q;
use crate::{IVec, TICKS_PER_DAY};
use hecs::Entity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Needs are stored as integers in `0..=NEED_MAX`.
pub const NEED_MAX: i32 = 10_000;
pub const CARRY_CAPACITY: u32 = 75;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Faction {
    #[default]
    Wild,
    Player,
    Hostile,
}

impl Faction {
    /// Every faction, in discriminant order. Per-faction tables index by
    /// `as usize`, so this is also their length.
    pub const ALL: [Faction; 3] = [Faction::Wild, Faction::Player, Faction::Hostile];

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
        /// Work the thing even though nobody designated it: foraging for
        /// food, or a job the player pointed at directly.
        forced: bool,
        /// Which of the thing's harvests.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        harvest: HarvestKey,
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
    /// Take a built thing down and get some of it back.
    Deconstruct {
        target: Entity,
    },
    Eat {
        src: Entity,
        t: u32,
        /// A chair at a table, if one was free: the thing and its spot.
        /// Without one the pawn eats where the food lies.
        seat: Option<(Entity, IVec)>,
        /// 0: going to the food. 1: carrying it to the seat.
        stage: u8,
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
    /// Break through a piece of wall, door or window that is in the way.
    Breach {
        target: Entity,
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
            Job::Deconstruct { .. } => "deconstructing",
            Job::Eat { .. } => "eating",
            Job::Sleep { stage: 1, .. } => "sleeping",
            Job::Sleep { .. } => "going to sleep",
            Job::Comfort { .. } => "warming up",
            Job::Attack { .. } => "fighting",
            Job::Breach { .. } => "breaking in",
            Job::Flee { .. } => "fleeing",
            Job::Leave { .. } => "leaving",
        }
    }
}

/// A creature. One component holds everything the AI touches each tick,
/// so the hot loop does one lookup per pawn.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thing {
    pub def: DefId,
    pub pos: IVec,
    pub count: u32,
    pub hp: i32,
}

/// Present on a fixture that is still under construction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blueprint {
    /// What this one costs, resolved when it was placed. A fixed recipe and
    /// a material choice both land here, so nothing downstream has to know
    /// which it was.
    pub cost: Vec<(DefId, u32)>,
    pub delivered: Vec<u32>,
}

/// Progress on a thing someone has started working: chopping, mining,
/// building or taking it down (DESIGN.md §6b). It lives on the thing, not
/// the job, so it survives the worker leaving, a second worker and a save,
/// and a renderer can draw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    pub done: u32,
    /// Resolved when work starts, already scaled by the material.
    pub total: u32,
    /// The designation this work is for; None for a build. A different
    /// designation starts over rather than inheriting another job's count.
    pub designation: Option<DefId>,
    /// Which side the last unit of work came from.
    pub side: Side,
}

impl Work {
    pub fn finished(&self) -> bool {
        self.done >= self.total
    }
}

/// One of the four sides of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    North,
    East,
    South,
    #[default]
    West,
}

impl Side {
    /// The side of `at` that `from` stands on. A diagonal neighbour counts
    /// as the side along its longer axis, and as east or west on a tie.
    pub fn of(at: IVec, from: IVec) -> Side {
        let (dx, dy) = (from.x - at.x, from.y - at.y);
        if dx.abs() >= dy.abs() && dx != 0 {
            if dx > 0 {
                Side::East
            } else {
                Side::West
            }
        } else if dy > 0 {
            Side::South
        } else {
            Side::North
        }
    }
}

/// How long a site stays a worksite after its last unit of work, so a pawn
/// stepping aside or a raider between swings (core's slowest melee
/// cooldown is 90 ticks) doesn't flicker it in and out of the cache.
pub const WORKSITE_GRACE: u64 = 120;

/// What a built thing is made of. Survives construction, so a finished
/// wall still knows it is stone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MadeOf(pub DefId);

/// Which faction built this fixture. Doors read it: a door opens for its
/// owner and stands in everyone else's way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner(pub Faction);

/// Player has marked this thing or creature for work.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Designated(pub DefId);

/// Harvests growing back, each named by its key:
/// `harvest` is ready again at `ready_at`, and any others are in `also`.
/// Nearly every regrowing thing has one, its first harvest, which saves
/// exactly as it did before a thing could have several.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Regrow {
    pub ready_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harvest: HarvestKey,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<(HarvestKey, u64)>,
}

impl Regrow {
    pub fn new(harvest: HarvestKey, ready_at: u64) -> Regrow {
        Regrow { ready_at, harvest, also: Vec::new() }
    }

    /// Every harvest growing back, and when it's ready.
    pub fn entries(&self) -> impl Iterator<Item = (HarvestKey, u64)> + '_ {
        std::iter::once((self.harvest, self.ready_at)).chain(self.also.iter().copied())
    }

    /// Whether this harvest is still growing back.
    pub fn growing(&self, harvest: HarvestKey) -> bool {
        self.entries().any(|(h, _)| h == harvest)
    }

    /// Keep the entries `keep` accepts, renamed as it says. False when
    /// none is left growing.
    pub fn retain(&mut self, mut keep: impl FnMut(HarvestKey, u64) -> Option<HarvestKey>) -> bool {
        let mut left: Vec<(HarvestKey, u64)> =
            self.entries().filter_map(|(h, at)| keep(h, at).map(|h| (h, at))).collect();
        let Some(first) = left.first().copied() else { return false };
        left.remove(0);
        (self.harvest, self.ready_at, self.also) = (first.0, first.1, left);
        true
    }

    /// Drop the harvests ready by `tick`. False when none is left growing.
    pub fn ripen(&mut self, tick: u64) -> bool {
        self.retain(|h, at| (at > tick).then_some(h))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub tick: u64,
    pub text: String,
    pub kind: MsgKind,
}

/// Things scripts can listen to with `rim.on(name, fn)`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameEvent {
    PawnJoined {
        id: Entity,
        name: String,
        def: DefId,
    },
    PawnDied {
        id: Entity,
        name: String,
        def: DefId,
        faction: Faction,
        pos: IVec,
        /// The colony's founder: an event for the colony, not its end.
        founder: bool,
    },
    PawnLeft {
        id: Entity,
        name: String,
        def: DefId,
        faction: Faction,
    },
    BuildingComplete {
        id: Entity,
        def: DefId,
        pos: IVec,
    },
    NewDay {
        day: u64,
    },
    /// A new season began (`season` is its name from the calendar).
    SeasonChanged {
        season: String,
        index: u32,
        year: u64,
    },
    /// Sent by a script with `rim.emit(name, data)`.
    Script {
        name: String,
        data: Option<Data>,
    },
    ColonyLost,
}

pub struct World {
    pub defs: Arc<DefDb>,
    pub seed: u64,
    /// Spawn through [`World::spawn`], never `ecs.spawn`: hecs's own
    /// allocator would hand out an id the world is about to use.
    pub ecs: hecs::World,
    /// The id the next entity gets. The world hands out entity ids itself
    /// and never reuses one, so an id means the same entity in a save, the
    /// command log and a script, whatever hecs would have allocated.
    pub(crate) next_entity: u32,
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
    /// Things being worked right now, with where they are and the last tick
    /// they were worked. Renderers draw these live and cache everything
    /// else, so the map is touched when one joins or leaves and at no other
    /// time. Derived: not saved, rebuilt by the next unit of work.
    pub worksites: BTreeMap<Entity, (IVec, u64)>,
    /// The last few notable events (tick, kind, pawn, name) for the UI:
    /// "joined", "died", "left". Not part of the simulation state.
    pub recent_events: Vec<(u64, &'static str, Entity, String)>,
    pub colony_lost: bool,
    /// The room rebuild the boundary sums were last computed for.
    seen_room_rebuilds: u64,
    /// State that scripts keep in the world (`rim.set_data`), by key.
    pub data: BTreeMap<String, Data>,
    /// For each mod whose script data is here but which isn't loaded, the
    /// version it wrote that data with: when it comes back, it migrates
    /// from there (0139).
    pub data_versions: BTreeMap<String, String>,
}

impl World {
    pub fn new(defs: Arc<DefDb>, w: i32, h: i32, seed: u64) -> Self {
        let fields = Fields::new(&defs, (w * h) as usize);
        World {
            defs,
            seed,
            ecs: hecs::World::new(),
            next_entity: 1,
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
            worksites: BTreeMap::new(),
            recent_events: Vec::new(),
            colony_lost: false,
            seen_room_rebuilds: u64::MAX,
            data: BTreeMap::new(),
            data_versions: BTreeMap::new(),
        }
    }

    // ------------------------------------------------------------ entities

    /// Spawn with the next world-owned id. Every entity comes through here.
    pub fn spawn(&mut self, components: impl hecs::DynamicBundle) -> Entity {
        let id = self.next_entity;
        self.next_entity = id.checked_add(1).expect("entity ids exhausted");
        let e = Entity::from_bits(1 << 32 | id as u64).expect("generation 1 is valid");
        self.ecs.spawn_at(e, components);
        e
    }

    // ------------------------------------------------------------ time

    pub fn day(&self) -> u64 {
        self.tick / TICKS_PER_DAY
    }
    /// Hour of day, 0..24. Tick 0 is 06:00.
    pub fn hour(&self) -> f64 {
        ((self.tick + TICKS_PER_DAY / 4) % TICKS_PER_DAY) as f64 / TICKS_PER_DAY as f64 * 24.0
    }
    /// Ticks since the start of the year the game began in.
    fn year_ticks(&self) -> u64 {
        self.defs.calendar.start_day as u64 * TICKS_PER_DAY + self.tick
    }
    /// The season at another tick (for noticing that one began).
    pub fn season_index_at(&self, tick: u64) -> u32 {
        let c = &self.defs.calendar;
        let doy = (c.start_day as u64 * TICKS_PER_DAY + tick) % self.year_len() / TICKS_PER_DAY;
        (doy * c.seasons.len() as u64 / c.year_days as u64) as u32
    }
    fn year_len(&self) -> u64 {
        self.defs.calendar.year_days as u64 * TICKS_PER_DAY
    }
    /// Day of the year, 0-based.
    pub fn day_of_year(&self) -> u32 {
        ((self.year_ticks() % self.year_len()) / TICKS_PER_DAY) as u32
    }
    /// Years since the game began, 0-based.
    pub fn year(&self) -> u64 {
        self.year_ticks() / self.year_len()
    }
    /// Index of the current season in the calendar.
    pub fn season_index(&self) -> u32 {
        self.season_index_at(self.tick)
    }
    pub fn season(&self) -> &str {
        &self.defs.calendar.seasons[self.season_index() as usize]
    }
    /// Day within the current season, 1-based.
    pub fn day_of_season(&self) -> u32 {
        let c = &self.defs.calendar;
        let n = c.seasons.len() as u32;
        let start = (self.season_index() * c.year_days).div_ceil(n);
        self.day_of_year() - start + 1
    }
    /// The clock as terms see it: fraction of the year and hour of day in
    /// fixed point.
    pub fn clock(&self) -> Clock {
        let yl = self.year_len();
        Clock {
            tick: self.tick,
            year: ((self.year_ticks() % yl) as i128 * Q as i128 / yl as i128) as i64,
            hour: (((self.tick + TICKS_PER_DAY / 4) % TICKS_PER_DAY) as i128 * 24 * Q as i128 / TICKS_PER_DAY as i128)
                as i64,
            seed: self.seed,
        }
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
        let e = self.spawn((p,));
        self.pawns.push(e);
        if faction == Faction::Player {
            self.note_event("joined", e, &name);
            self.events.push(GameEvent::PawnJoined { id: e, name, def });
        }
        e
    }

    /// Remember a notable event for the UI (bounded; not simulation state).
    pub fn note_event(&mut self, kind: &'static str, e: Entity, name: &str) {
        self.recent_events.push((self.tick, kind, e, name.to_string()));
        if self.recent_events.len() > 32 {
            self.recent_events.remove(0);
        }
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
        self.spawn_fixture_of(def, pos, blueprint, None)
    }

    /// `stuff` is the material chosen for a buildable that takes one. It is
    /// ignored for anything with a fixed recipe.
    pub fn spawn_fixture_of(&mut self, def: DefId, pos: IVec, blueprint: bool, stuff: Option<DefId>) -> Option<Entity> {
        if !self.map.inb(pos) {
            return None;
        }
        let defs = self.defs.clone();
        let td = defs.thing(def);
        // A floor wants its own layer free; anything else, the fixture layer.
        let is_floor = td.category == Category::Floor;
        if (is_floor && self.map.floor_at(pos).is_some()) || (!is_floor && self.map.fixture_at(pos).is_some()) {
            return None;
        }
        let made_of = stuff.filter(|_| td.build.as_ref().is_some_and(|b| b.stuff.is_some()));
        // The material scales what the def says. Nothing here knows which
        // names exist; it asks for two and multiplies by whatever comes back.
        let hp = (td.hp as f64 * defs.factor(made_of, "hp")).round().max(1.0) as i32;
        let t = Thing { def, pos, count: 1, hp };
        let e = if blueprint {
            let b = td.build.as_ref()?;
            let cost = match (&b.stuff, made_of) {
                (Some(sc), Some(m)) => vec![(m, sc.count)],
                (Some(_), None) => return None, // needs a material and was given none
                (None, _) => b.cost_r.clone(),
            };
            let total = (b.work as f64 * defs.factor(made_of, "work")).round().max(1.0) as u32;
            let bp = Blueprint { delivered: vec![0; cost.len()], cost };
            self.spawn((t, bp, Work { done: 0, total, designation: None, side: Side::default() }))
        } else {
            self.spawn((t,))
        };
        if let Some(m) = made_of {
            let _ = self.ecs.insert_one(e, MadeOf(m));
        }
        if is_floor {
            self.map.set_floor(pos, Some(e), if blueprint { 0 } else { td.path_cost });
        } else {
            let (blocks, cost, door) = if blueprint { (false, 0, false) } else { (td.blocks, td.path_cost, td.door) };
            self.map.set_fixture(pos, Some(e), blocks, cost, door);
        }
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
                            let e = self.spawn((Thing { def, pos: p, count: n, hp: 100 },));
                            self.map.set_item(p, Some(e));
                            let defs = self.defs.clone();
                            self.fields.add_emitters(&defs, &self.map, e, def, p);
                            count -= n;
                        }
                        Some(e) => {
                            if let Ok(mut t) = self.ecs.get::<&mut Thing>(e) {
                                if t.def == def && t.count < limit {
                                    let n = count.min(limit - t.count);
                                    t.count += n;
                                    count -= n;
                                    drop(t);
                                    self.map.touch(p);
                                }
                            }
                        }
                    }
                }
            }
        }
        count
    }

    /// Whether a harvest of thing `e`, by key, can be worked: it isn't
    /// growing back.
    pub fn harvest_ready(&self, e: Entity, harvest: HarvestKey) -> bool {
        self.ecs.get::<&Regrow>(e).map_or(true, |r| !r.growing(harvest))
    }

    /// A harvest of thing `e`, by key, grows back until `ready_at`.
    pub fn regrow(&mut self, e: Entity, harvest: HarvestKey, ready_at: u64) {
        if let Ok(mut r) = self.ecs.get::<&mut Regrow>(e) {
            r.also.push((harvest, ready_at));
            return;
        }
        let _ = self.ecs.insert_one(e, Regrow::new(harvest, ready_at));
    }

    pub fn despawn_thing(&mut self, e: Entity) {
        let Ok(t) = self.ecs.get::<&Thing>(e).map(|t| (*t).clone()) else { return };
        if self.map.inb(t.pos) {
            let i = self.map.idx(t.pos);
            if self.map.item[i] == Some(e) {
                self.map.set_item(t.pos, None);
            }
            if self.map.fixture[i] == Some(e) {
                self.map.set_fixture(t.pos, None, false, 0, false);
                self.map.set_owner(t.pos, None);
            }
            if self.map.floor[i] == Some(e) {
                self.map.set_floor(t.pos, None, 0);
            }
        }
        self.reservations.remove(&e);
        self.worksites.remove(&e);
        self.fields.remove_emitters(e);
        let _ = self.ecs.despawn(e);
    }

    // ------------------------------------------------------------ work

    /// One unit of work on `e`, which stands at `at`, by a worker at `from`.
    /// `total` is asked for only when the work starts, or when it was for
    /// another designation. Returns the work after this unit.
    pub fn work_on(
        &mut self,
        e: Entity,
        at: IVec,
        from: IVec,
        designation: Option<DefId>,
        total: impl FnOnce(&World) -> u32,
    ) -> Option<Work> {
        let side = Side::of(at, from);
        let same = match self.ecs.get::<&mut Work>(e) {
            Ok(mut w) if w.designation == designation => {
                w.done = (w.done + 1).min(w.total);
                w.side = side;
                Some(*w)
            }
            _ => None,
        };
        let w = match same {
            Some(w) => w,
            None => {
                let total = total(self).max(1);
                let w = Work { done: 1.min(total), total, designation, side };
                self.ecs.insert_one(e, w).ok()?;
                w
            }
        };
        self.mark_worksite(e, at);
        Some(w)
    }

    /// `e`, at `at`, is being worked this tick. The map is touched only when
    /// it becomes a worksite, so a renderer moves it from its cache to the
    /// live list once, not on every unit of progress.
    pub fn mark_worksite(&mut self, e: Entity, at: IVec) {
        if self.worksites.insert(e, (at, self.tick)).is_none() {
            self.map.touch(at);
        }
    }

    /// Forget sites nobody has worked for `WORKSITE_GRACE` ticks, touching
    /// the map so a renderer caches them again at their current stage.
    pub fn sweep_worksites(&mut self) {
        let (tick, map) = (self.tick, &mut self.map);
        self.worksites.retain(|_, &mut (at, last)| {
            let keep = tick.saturating_sub(last) <= WORKSITE_GRACE;
            if !keep {
                map.touch(at);
            }
            keep
        });
    }

    pub fn is_worksite(&self, e: Entity) -> bool {
        self.worksites.contains_key(&e)
    }

    /// How far along `e` looks, 0..=8: the larger of the work done on it and
    /// the hp it has lost (DESIGN.md §6b). Renderers read this and nothing
    /// finer, so a cached thing is right until the next touch.
    pub fn stage(&self, e: Entity) -> u8 {
        let worked = self.ecs.get::<&Work>(e).map_or(0, |w| (w.done as u64 * 8 / w.total.max(1) as u64) as u8);
        let hurt = match (self.ecs.get::<&Thing>(e), self.stat(e, "hp")) {
            (Ok(t), Some(max)) if max >= 1.0 => {
                let max = max.round() as i64;
                ((max - t.hp as i64).clamp(0, max) * 8 / max) as u8
            }
            _ => 0,
        };
        worked.max(hurt)
    }

    /// Take up to `n` from a stack, despawning it when empty.
    pub fn take_from_stack(&mut self, e: Entity, n: u32) -> u32 {
        let (taken, empty, pos) = match self.ecs.get::<&mut Thing>(e) {
            Ok(mut t) => {
                let k = n.min(t.count);
                t.count -= k;
                (k, t.count == 0, t.pos)
            }
            Err(_) => return 0,
        };
        self.map.touch(pos);
        if empty {
            self.despawn_thing(e);
        }
        taken
    }

    /// Something about how `e` is drawn changed (see `Map::touch`).
    pub fn touch(&mut self, e: Entity) {
        if let Some(t) = self.thing(e) {
            self.map.touch(t.pos);
        }
    }

    pub fn thing(&self, e: Entity) -> Option<Thing> {
        self.ecs.get::<&Thing>(e).ok().map(|t| (*t).clone())
    }

    // ------------------------------------------------------------ boundaries

    /// After a room rebuild: what each room is made of, as two numbers per
    /// field. The map already knows each room's boundary cells; this looks
    /// up what stands in them and what it is made of. Pieces that declare
    /// nothing count as the field's own constant, so a boundary nobody
    /// described behaves exactly as it did before there were boundaries.
    pub fn refresh_boundaries(&mut self) {
        if self.map.room_rebuilds == self.seen_room_rebuilds {
            return;
        }
        self.seen_room_rebuilds = self.map.room_rebuilds;
        let defs = self.defs.clone();
        let rooms = self.map.room_count();
        self.fields.reset_boundaries(rooms);
        for r in 0..rooms {
            let cells = self.map.room_boundary(r as u32 + 1);
            if cells.is_empty() {
                continue;
            }
            for (fi, fd) in defs.fields.iter().enumerate() {
                let (mut leak_sum, mut pass_sum) = (0.0, 0.0);
                for &c in cells {
                    let piece = self.map.fixture[c as usize].and_then(|e| {
                        let t = self.ecs.get::<&Thing>(e).ok()?;
                        let b = defs.thing(t.def).boundary.iter().find(|b| b.field_r as usize == fi)?;
                        let made_of = self.ecs.get::<&MadeOf>(e).ok().map(|m| m.0);
                        Some((b.leak, b.pass, made_of))
                    });
                    match piece {
                        Some((leak, pass, made_of)) => {
                            let f = if fd.boundary_factor.is_empty() {
                                1.0
                            } else {
                                defs.factor(made_of, &fd.boundary_factor)
                            };
                            // A factor of zero would make a piece infinitely leaky.
                            let f = if f > 0.0 { f } else { 1.0 };
                            leak_sum += leak / f;
                            pass_sum += pass * f;
                        }
                        None => leak_sum += 1.0,
                    }
                }
                self.fields.set_boundary(fi, r as u32 + 1, leak_sum / cells.len() as f64, pass_sum.min(1.0));
            }
        }
    }

    /// What a built thing cost, in what it was made of: the material and
    /// count for stuff, the recipe otherwise. None for anything not built.
    pub fn cost_of(&self, e: Entity) -> Option<Vec<(DefId, u32)>> {
        let t = self.ecs.get::<&Thing>(e).ok()?;
        let b = self.defs.thing(t.def).build.as_ref()?;
        Some(match (&b.stuff, self.ecs.get::<&MadeOf>(e).ok().map(|m| m.0)) {
            (Some(sc), Some(m)) => vec![(m, sc.count)],
            (Some(_), None) => Vec::new(), // built of nothing we know: nothing to give back
            (None, _) => b.cost_r.clone(),
        })
    }

    // ------------------------------------------------------------ stats

    /// A thing's stat: the def's base times its material's factor of the
    /// same name. The engine has a base for three names -- `hp`, `work`
    /// and `value` -- and for anything else the factor stands alone, so a
    /// script gets back exactly the number a material declared. None when
    /// neither side has anything to say.
    pub fn stat(&self, e: Entity, name: &str) -> Option<f64> {
        let t = self.ecs.get::<&Thing>(e).ok()?;
        let td = self.defs.thing(t.def);
        let made_of = self.ecs.get::<&MadeOf>(e).ok().map(|m| m.0);
        let base = match name {
            "hp" => Some(td.hp as f64),
            "work" => td.build.as_ref().map(|b| b.work as f64),
            "value" => Some(td.market_value),
            _ => None,
        };
        let factor = self.defs.factor_declared(made_of, name);
        match (base, factor) {
            (None, None) => None,
            (b, f) => Some(b.unwrap_or(1.0) * f.unwrap_or(1.0)),
        }
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
        for t in self.ecs.query::<&Thing>().iter() {
            // Order-independent combine for things.
            h = h.wrapping_add(crate::rng::mix(
                (t.def as u64) << 40 ^ (t.pos.x as u64) << 20 ^ t.pos.y as u64 ^ (t.count as u64) << 56,
            ));
        }
        for a in &self.fields.atmos {
            h = crate::rng::mix(h ^ a.value as u64 ^ a.pin.map_or(0, |p| p as u64 ^ 0x9111));
            for p in &a.pushes {
                h = crate::rng::mix(h ^ p.to as u64 ^ p.start << 1 ^ p.until.unwrap_or(7));
                h = p.key.bytes().fold(h, |h, b| crate::rng::mix(h ^ b as u64));
            }
        }
        for (k, v) in &self.data {
            h = v.hash(k.bytes().fold(h, |h, b| crate::rng::mix(h ^ b as u64)));
        }
        h
    }
}
