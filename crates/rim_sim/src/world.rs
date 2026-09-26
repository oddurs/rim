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
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
        /// A tool to fetch first, when the harvest needs one the pawn
        /// doesn't hold.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool: Option<Entity>,
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
    /// Bring a work order's input: pick up at `src` (stage 0), carry it to
    /// the site (stage 1) and add it to need `need`.
    Supply {
        site: Entity,
        src: Entity,
        need: u8,
        want: u32,
        stage: u8,
    },
    /// Work a work order whose inputs are all in, fetching its tool first.
    Craft {
        site: Entity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool: Option<Entity>,
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
    /// Carry a stack to a stockpile cell: 0 to fetch it, 1 to bring it.
    Haul {
        src: Entity,
        to: IVec,
        stage: u8,
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
            Job::Supply { .. } => "fetching materials",
            Job::Craft { .. } => "working",
            Job::Deconstruct { .. } => "deconstructing",
            Job::Eat { .. } => "eating",
            Job::Sleep { stage: 1, .. } => "sleeping",
            Job::Sleep { .. } => "going to sleep",
            Job::Comfort { .. } => "warming up",
            Job::Attack { .. } => "fighting",
            Job::Breach { .. } => "breaking in",
            Job::Flee { .. } => "fleeing",
            Job::Leave { .. } => "leaving",
            Job::Haul { .. } => "hauling",
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
    pub carry: Option<Lot>,
    pub last_attacker: Option<Entity>,
    pub asleep: bool,
    /// Rest-recovery multiplier in percent while asleep.
    pub sleep_rate: u32,
    pub dead: bool,
    /// Walked off the map; removed at the end of the tick.
    pub left: bool,
    /// Hostiles give up and head for the map edge at this tick.
    pub leave_at: Option<u64>,
    /// Work priorities the player set, by work type: 1 first, 0 never. A
    /// work type not here is at its def's default (DESIGN.md §4d).
    #[serde(default)]
    pub priorities: Vec<(DefId, u8)>,
    /// The tool it holds, off the map while it's held (DESIGN.md §4e).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand: Option<Entity>,
    /// Experience per skill, by skill def, sorted; a level follows from it.
    #[serde(default)]
    pub skills: Vec<(DefId, u32)>,
    /// Work carried over between ticks, in hundredths of a unit, so a
    /// colonist at 150% does three units every two ticks.
    #[serde(default)]
    pub work_frac: u32,
}

/// The highest skill level.
pub const SKILL_MAX: u32 = 20;

/// Experience a skill level takes: 2000 for the first, each level a
/// thousand more than the last, so level 20 is about six weeks of work.
pub fn skill_xp(level: u32) -> u32 {
    1000 * level * (level + 1)
}

impl Pawn {
    pub fn moving(&self) -> bool {
        self.next.is_some() || !self.path.is_empty()
    }
    pub fn need(&self, need: DefId) -> Option<i32> {
        self.needs.iter().find(|n| n.0 == need).map(|n| n.1)
    }

    /// This pawn's priority for a work type: 1 first, 0 never. A level
    /// saved above a scale a mod has since shrunk counts as the last level.
    pub fn priority(&self, defs: &DefDb, work: DefId) -> u8 {
        let set = self.priorities.iter().find(|p| p.0 == work).map(|p| p.1);
        set.unwrap_or(defs.work_types[work as usize].priority).min(defs.priority_scale.levels)
    }

    /// A skill's level, 0 to `SKILL_MAX`.
    pub fn skill(&self, skill: DefId) -> u32 {
        let xp = self.skills.iter().find(|s| s.0 == skill).map_or(0, |s| s.1);
        (1..=SKILL_MAX).take_while(|&l| skill_xp(l) <= xp).last().unwrap_or(0)
    }

    /// Learn by doing.
    pub fn learn(&mut self, skill: DefId, xp: u32) {
        match self.skills.binary_search_by_key(&skill, |s| s.0) {
            Ok(i) => self.skills[i].1 = self.skills[i].1.saturating_add(xp).min(skill_xp(SKILL_MAX)),
            Err(i) => self.skills.insert(i, (skill, xp.min(skill_xp(SKILL_MAX)))),
        }
    }

    /// This tick's units of work at a skill, and the experience for it:
    /// 60% of normal speed untrained, normal at level 4, up to 260% at 20.
    pub fn work_amount(&mut self, skill: Option<DefId>) -> u32 {
        let Some(s) = skill else { return 1 };
        let pct = self.work_frac + 60 + 10 * self.skill(s);
        self.work_frac = pct % 100;
        self.learn(s, 1);
        pct / 100
    }

    /// Set a priority, keeping the list in work-type order so the state
    /// doesn't depend on the order changes were made in.
    pub fn set_priority(&mut self, work: DefId, level: u8) {
        match self.priorities.binary_search_by_key(&work, |p| p.0) {
            Ok(i) => self.priorities[i].1 = level,
            Err(i) => self.priorities.insert(i, (work, level)),
        }
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
    /// Progress toward another designation, shelved when this work took
    /// its place: gathering branches from a half-felled tree doesn't undo
    /// the felling. Coming back to that work picks it up again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<Shelved>,
}

/// Work put aside for other work on the same thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelved {
    pub designation: Option<DefId>,
    pub done: u32,
    pub total: u32,
}

impl Work {
    pub fn new(total: u32, designation: Option<DefId>) -> Work {
        Work { done: 0, total: total.max(1), designation, side: Side::default(), other: None }
    }

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

/// On a natural thing (grass, a tree, rock) where the player planned a
/// building: it's marked to be cleared, and when it's gone the blueprint
/// goes up in its place.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Planned {
    pub thing: DefId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stuff: Option<DefId>,
}

/// Bring these things to a site, then work there (DESIGN.md §4e). A mod
/// posts one on a site (a station) for anything made, cooked or studied,
/// and hears `order_done` when it's worked through. One at a time per site.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    /// The mod that posted it.
    pub owner: String,
    /// What it makes, for the player ("hand axe").
    pub label: String,
    pub needs: Vec<Need>,
    /// Work ticks at bare hands' pace.
    pub work: u32,
    /// Tool tags the worker must hold (core's shared names).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// Who does it: the work type it's posted under.
    pub work_type: DefId,
    /// Progress, kept on the order and not in `Work`, which on a finished
    /// building means taking it down. `total` is set when work starts,
    /// scaled by the worker's tool.
    #[serde(default)]
    pub done: u32,
    #[serde(default)]
    pub total: u32,
}

/// Some of one thing off the map: in a pawn's hands, or brought to a work
/// order. It keeps what it's made of and its hp, so a flint axe hauled to a
/// stockpile is set down as that flint axe, and flint and bone stacks stay
/// apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "LotRepr")]
pub struct Lot {
    pub def: DefId,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made_of: Option<DefId>,
    /// The hp of the stack it came from. None only in older saves: set down,
    /// it takes its def's hp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
}

impl Lot {
    /// Plain things: no material, the def's own hp.
    pub fn new(def: DefId, count: u32) -> Lot {
        Lot { def, count, made_of: None, hp: None }
    }
}

/// Saves before lots carried `(def, count)`.
#[derive(Deserialize)]
#[serde(untagged)]
enum LotRepr {
    Pair(DefId, u32),
    Lot {
        def: DefId,
        count: u32,
        #[serde(default)]
        made_of: Option<DefId>,
        #[serde(default)]
        hp: Option<i32>,
    },
}

impl From<LotRepr> for Lot {
    fn from(r: LotRepr) -> Lot {
        match r {
            LotRepr::Pair(def, count) => Lot::new(def, count),
            LotRepr::Lot { def, count, made_of, hp } => Lot { def, count, made_of, hp },
        }
    }
}

/// One input of a work order: a thing by def, or anything with a tag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Need {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thing: Option<DefId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub count: u32,
    /// What has arrived so far, one lot per def and material.
    #[serde(default)]
    pub delivered: Vec<Lot>,
}

impl Need {
    pub fn have(&self) -> u32 {
        self.delivered.iter().map(|d| d.count).sum()
    }
    pub fn missing(&self) -> u32 {
        self.count.saturating_sub(self.have())
    }
    /// Whether a thing of `def` meets this need.
    pub fn takes(&self, defs: &DefDb, def: DefId) -> bool {
        match (&self.thing, &self.tag) {
            (Some(t), _) => *t == def,
            (None, Some(tag)) => defs.thing(def).tags.contains(tag),
            (None, None) => false,
        }
    }
}

impl Order {
    /// The first need still short, and by how much.
    pub fn missing(&self) -> Option<(usize, u32)> {
        self.needs.iter().enumerate().find_map(|(i, n)| (n.missing() > 0).then(|| (i, n.missing())))
    }
}

/// On a tool a pawn holds: it is off the map until put down.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Held {
    pub by: Entity,
}

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

/// A line a pawn says (DESIGN.md §11): presentation only, like a message.
#[derive(Clone, Debug, PartialEq)]
pub struct Speech {
    pub tick: u64,
    pub pawn: Entity,
    pub text: String,
    /// How long it stays up.
    pub ticks: u32,
    /// Which line shows when a pawn has several, and which bubbles win a
    /// crowded screen.
    pub priority: i32,
}

/// How many lines the speech log keeps: far more than can show at once.
pub const SPEECH_LOG: usize = 64;
/// A line longer than this is cut: a bubble is a line or two, not a letter.
pub const SPEECH_CHARS: usize = 120;

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
    /// A work order was worked to the end: what went into it, and what
    /// it makes should be made of: the first input that is a material, or
    /// else the first input's own material.
    OrderDone {
        site: Entity,
        owner: String,
        label: String,
        inputs: Vec<Lot>,
        stuff: Option<DefId>,
    },
    /// A work order's site was destroyed before it was done. What had been
    /// brought is back on the ground there.
    OrderLost {
        site: Entity,
        owner: String,
        label: String,
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
    /// Every tool in the world, held or lying about, so what the colony can
    /// do is a few lookups rather than a scan. Derived: rebuilt on load.
    pub tools: BTreeSet<Entity>,
    /// The last few notable events (tick, kind, pawn, name) for the UI:
    /// "joined", "died", "left". Not part of the simulation state.
    pub recent_events: Vec<(u64, &'static str, Entity, String)>,
    /// What pawns are saying, newest last, for renderers: a need's line, or
    /// a script's `rim.say`. Not saved and read by nothing in the sim.
    pub speech: std::collections::VecDeque<Speech>,
    pub colony_lost: bool,
    /// The room rebuild the boundary sums were last computed for.
    seen_room_rebuilds: u64,
    /// State that scripts keep in the world (`rim.set_data`), by key.
    pub data: BTreeMap<String, Data>,
    /// Stockpile zones the player painted.
    pub zones: crate::zone::Zones,
    /// How many times shelter has been worked out: for tests and profiling,
    /// not simulation state.
    pub shelter_recomputes: u64,
    /// The colony's stance: which stance's priority rules hold.
    pub stance: Option<DefId>,
    /// The priority rules that hold colony-wide. Derived: rebuilt on load.
    pub rules: crate::rules::Rules,
    /// For each mod whose script data is here but which isn't loaded, the
    /// version it wrote that data with: when it comes back, it migrates
    /// from there (0139).
    pub data_versions: BTreeMap<String, String>,
}

impl World {
    pub fn new(defs: Arc<DefDb>, w: i32, h: i32, seed: u64) -> Self {
        let fields = Fields::new(&defs, (w * h) as usize);
        let stance = defs.default_stance;
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
            tools: BTreeSet::new(),
            recent_events: Vec::new(),
            speech: std::collections::VecDeque::new(),
            colony_lost: false,
            seen_room_rebuilds: u64::MAX,
            data: BTreeMap::new(),
            zones: crate::zone::Zones::new((w * h) as usize),
            shelter_recomputes: 0,
            stance,
            rules: crate::rules::Rules::default(),
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
        // People arrive knowing a little of everything, some more than others.
        let skills = match cd.intelligent {
            true => (0..defs.skills.len() as DefId).map(|s| (s, skill_xp(self.rng.below(7)))).collect(),
            false => Vec::new(),
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
            skills,
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

    /// `pawn` says `text` for `ticks` ticks. Presentation only: the sim never
    /// reads it back, so it needs no RNG and changes nothing a save holds.
    pub fn say(&mut self, pawn: Entity, text: &str, ticks: u32, priority: i32) {
        let text = match text.char_indices().nth(SPEECH_CHARS) {
            Some((cut, _)) => format!("{}…", text[..cut].trim_end()),
            None => text.to_string(),
        };
        if self.speech.len() == SPEECH_LOG {
            self.speech.pop_front();
        }
        self.speech.push_back(Speech { tick: self.tick, pawn, text, ticks: ticks.max(1), priority });
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
    /// Where a pawn stands to work on a thing: next to any cell it covers.
    pub fn reach_goal(&self, t: &Thing) -> crate::path::Goal {
        match self.defs.thing(t.def).size {
            [1, 1] => crate::path::Goal::Touch(t.pos),
            [w, h] => crate::path::Goal::Area { at: t.pos, size: [w as u8, h as u8] },
        }
    }

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
        // The rest of a bigger thing's footprint must be open ground too.
        let more = td.size != [1, 1];
        if more
            && !td.footprint(pos).all(|c| self.map.inb(c) && self.map.passable(c) && self.map.fixture_at(c).is_none())
        {
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
            self.spawn((t, bp, Work::new(total, None)))
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
            for c in td.footprint(pos) {
                self.map.set_fixture(c, Some(e), blocks, cost, door);
            }
        }
        if !blueprint {
            self.fields.add_emitters(&defs, &self.map, e, def, pos);
        }
        Some(e)
    }

    /// Drop items near `near`, merging into existing stacks. Returns the
    /// amount that could not be placed.
    /// Room a cell has for `def`: a stack's worth if it holds no item, the
    /// rest of the stack if it holds the same one, else none. A cell with a
    /// fixture on it, even a blueprint, has none: a wall would go up over
    /// the stack and nobody could reach it again.
    pub fn room_for(&self, def: DefId, made_of: Option<DefId>, p: IVec) -> u32 {
        if !self.map.inb(p) || !self.map.passable(p) || self.map.fixture_at(p).is_some() {
            return 0;
        }
        let limit = self.defs.thing(def).stack_limit;
        match self.map.item_at(p) {
            None => limit,
            Some(e) => self
                .thing(e)
                .filter(|t| t.def == def && self.made_of(e) == made_of)
                .map_or(0, |t| limit.saturating_sub(t.count)),
        }
    }

    /// What a thing is made of, if it was made of anything.
    pub fn made_of(&self, e: Entity) -> Option<DefId> {
        self.ecs.get::<&MadeOf>(e).ok().map(|m| m.0)
    }

    /// Put as much of a lot as fits on exactly this cell, as a new stack or
    /// on the same one there. Returns how many didn't fit.
    pub fn put_lot(&mut self, lot: Lot, p: IVec) -> u32 {
        let n = lot.count.min(self.room_for(lot.def, lot.made_of, p));
        if n > 0 {
            self.add_to_cell(Lot { count: n, ..lot }, p);
        }
        lot.count - n
    }

    /// Put a lot's worth on a cell that has room for it: onto the stack
    /// there, which keeps its hp, or as a new stack with the lot's.
    fn add_to_cell(&mut self, lot: Lot, p: IVec) {
        if let Some(e) = self.map.item_at(p) {
            if let Ok(mut t) = self.ecs.get::<&mut Thing>(e) {
                t.count += lot.count;
            }
            self.map.touch(p);
            return;
        }
        let td = self.defs.thing(lot.def);
        let hp = lot.hp.unwrap_or_else(|| (td.hp as f64 * self.defs.factor(lot.made_of, "hp")).round().max(1.0) as i32);
        let tool = td.tool.is_some();
        let e = self.spawn((Thing { def: lot.def, pos: p, count: lot.count, hp },));
        if let Some(m) = lot.made_of {
            let _ = self.ecs.insert_one(e, MadeOf(m));
        }
        if tool {
            self.tools.insert(e);
        }
        self.map.set_item(p, Some(e));
        let defs = self.defs.clone();
        self.fields.add_emitters(&defs, &self.map, e, lot.def, p);
    }

    pub fn place_item(&mut self, def: DefId, near: IVec, count: u32) -> u32 {
        self.place_lot(Lot::new(def, count), near)
    }

    /// `place_item`, made of `stuff`: a flint axe, a bone one.
    pub fn place_item_of(&mut self, def: DefId, near: IVec, count: u32, stuff: Option<DefId>) -> u32 {
        self.place_lot(Lot { made_of: stuff, ..Lot::new(def, count) }, near)
    }

    /// Drop a lot near `near`, merging into stacks of the same thing of the
    /// same material. Returns how many didn't fit.
    pub fn place_lot(&mut self, lot: Lot, near: IVec) -> u32 {
        let limit = self.defs.thing(lot.def).stack_limit;
        let mut count = lot.count;
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
                    // Not under a plan, or where one will go once the grass
                    // is cleared: a wall would go up over the stack.
                    let planned = self
                        .map
                        .fixture_at(p)
                        .is_some_and(|f| self.ecs.get::<&Blueprint>(f).is_ok() || self.ecs.get::<&Planned>(f).is_ok());
                    if !self.map.passable(p) || planned {
                        continue;
                    }
                    let room = match self.map.item_at(p) {
                        None => limit,
                        Some(e) => self
                            .thing(e)
                            .filter(|t| t.def == lot.def && self.made_of(e) == lot.made_of)
                            .map_or(0, |t| limit.saturating_sub(t.count)),
                    };
                    let n = count.min(room);
                    if n > 0 {
                        self.add_to_cell(Lot { count: n, ..lot }, p);
                        count -= n;
                    }
                }
            }
        }
        count
    }

    /// Take up to `n` from a stack, as a lot that remembers it.
    pub fn pick_up(&mut self, e: Entity, n: u32) -> Option<Lot> {
        let t = self.thing(e)?;
        let made_of = self.made_of(e);
        let count = self.take_from_stack(e, n);
        (count > 0).then_some(Lot { def: t.def, count, made_of, hp: Some(t.hp) })
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

    /// Plan a building over a natural thing: mark it to be cleared with the
    /// harvest that removes it (gather grass, chop an oak, mine rock), and
    /// the blueprint follows. With no such harvest, something underfoot (a
    /// berry bush, which regrows) is cleared now; something that blocks is
    /// left, as there's no work that would clear it.
    pub fn plan_over(&mut self, e: Entity, thing: DefId, stuff: Option<DefId>) {
        let Some(t) = self.thing(e) else { return };
        let td = self.defs.thing(t.def);
        match td.harvest.iter().find(|h| h.destroy).map(|h| h.desig_r) {
            Some(d) => {
                let _ = self.ecs.insert(e, (Designated(d), Planned { thing, stuff }));
                self.map.touch(t.pos);
            }
            None if !td.blocks => {
                self.despawn_thing(e);
                self.spawn_fixture_of(thing, t.pos, true, stuff);
            }
            None => {}
        }
    }

    pub fn despawn_thing(&mut self, e: Entity) {
        let Ok(t) = self.ecs.get::<&Thing>(e).map(|t| (*t).clone()) else { return };
        let planned = self.ecs.get::<&Planned>(e).ok().map(|p| *p);
        // A site taken down mid-order: what was brought stays, and the mod
        // that posted it hears.
        if let Ok(o) = self.ecs.remove_one::<Order>(e) {
            for &lot in o.needs.iter().flat_map(|n| &n.delivered) {
                self.place_lot(lot, t.pos);
            }
            self.events.push(GameEvent::OrderLost { site: e, owner: o.owner, label: o.label });
        }
        if self.map.inb(t.pos) {
            let i = self.map.idx(t.pos);
            if self.map.item[i] == Some(e) {
                self.map.set_item(t.pos, None);
            }
            if self.map.fixture[i] == Some(e) {
                let defs = self.defs.clone();
                for c in defs.thing(t.def).footprint(t.pos) {
                    if self.map.inb(c) && self.map.fixture_at(c) == Some(e) {
                        self.map.set_fixture(c, None, false, 0, false);
                        self.map.set_owner(c, None);
                    }
                }
            }
            if self.map.floor[i] == Some(e) {
                self.map.set_floor(t.pos, None, 0);
            }
        }
        self.reservations.remove(&e);
        self.worksites.remove(&e);
        self.tools.remove(&e);
        self.fields.remove_emitters(e);
        let _ = self.ecs.despawn(e);
        // Cleared for a building: it goes up in its place.
        if let Some(p) = planned {
            self.spawn_fixture_of(p.thing, t.pos, true, p.stuff);
        }
    }

    // ------------------------------------------------------------ tools

    /// The tags a thing does as a tool; none if it isn't one.
    pub fn tool_tags(&self, e: Entity) -> ToolMask {
        let def = self.ecs.get::<&Thing>(e).map(|t| t.def);
        def.ok().and_then(|d| self.defs.thing(d).tool.as_ref()).map_or(0, |t| t.tags_r)
    }

    /// Every tag some tool in the world has, held or not.
    pub fn colony_tools(&self) -> ToolMask {
        self.tools.iter().fold(0, |m, &t| m | self.tool_tags(t))
    }

    /// Whether what `p` holds covers `need`. Nothing is needed of bare hands.
    pub fn hand_covers(&self, p: &Pawn, need: ToolMask) -> bool {
        need == 0 || p.hand.is_some_and(|t| self.tool_tags(t) & need == need)
    }

    /// How fast a tool works: its own speed times its material's.
    pub fn tool_speed(&self, t: Entity) -> f64 {
        let def = self.ecs.get::<&Thing>(t).map(|t| t.def);
        let speed = def.ok().and_then(|d| self.defs.thing(d).tool.as_ref()).map_or(1.0, |t| t.speed);
        speed * self.stat(t, "tool_speed").unwrap_or(1.0)
    }

    /// The nearest tool lying about that covers `need`, reachable from
    /// `from`, that nobody but `by` has claimed.
    pub fn nearest_tool(&self, by: Entity, from: IVec, need: ToolMask) -> Option<(u32, Entity)> {
        let mut best: Option<(u32, Entity)> = None;
        for &t in &self.tools {
            if self.tool_tags(t) & need != need || self.ecs.get::<&Held>(t).is_ok() || self.reserved_by_other(t, by) {
                continue;
            }
            let Ok(pos) = self.ecs.get::<&Thing>(t).map(|t| t.pos) else { continue };
            let d = pos.octile(from);
            if best.is_some_and(|b| (b.0, b.1.id()) <= (d, t.id())) || !self.map.can_reach(from, Goal::Cell(pos)) {
                continue;
            }
            best = Some((d, t));
        }
        best
    }

    /// `by` picks up `tool` from the map, putting down what it held there.
    pub fn take_tool(&mut self, by: Entity, p: &mut Pawn, tool: Entity) {
        let Some(at) = self.thing(tool).map(|t| t.pos) else { return };
        if self.map.item_at(at) == Some(tool) {
            self.map.set_item(at, None);
        }
        // Held is its claim now; nobody else can take it. What it emits
        // (a torch's light) goes with the pawn, not the cell it lay in.
        self.fields.remove_emitters(tool);
        let _ = self.ecs.insert_one(tool, Held { by });
        if self.reservations.get(&tool) == Some(&by) {
            self.reservations.remove(&tool);
        }
        if let Some(old) = p.hand.replace(tool) {
            self.put_down(old, at);
        }
    }

    /// A held tool goes back on the map, in the nearest free cell to `near`.
    pub fn put_down(&mut self, tool: Entity, near: IVec) {
        let _ = self.ecs.remove_one::<Held>(tool);
        let free = (0..=8i32).flat_map(|r| {
            (-r..=r)
                .flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy)))
                .filter(move |(dx, dy)| dx.abs().max(dy.abs()) == r)
        });
        for (dx, dy) in free {
            let p = near.offset(dx, dy);
            if self.map.passable(p) && self.map.item_at(p).is_none() {
                let Ok(def) = self.ecs.get::<&mut Thing>(tool).map(|mut t| {
                    t.pos = p;
                    t.def
                }) else {
                    return;
                };
                self.map.set_item(p, Some(tool));
                let defs = self.defs.clone();
                self.fields.add_emitters(&defs, &self.map, tool, def, p);
                return;
            }
        }
        // Nowhere to put it: it's lost.
        self.despawn_thing(tool);
    }

    /// A finished job wears what `p` holds. At no hp left it breaks.
    pub fn wear_tool(&mut self, p: &mut Pawn) {
        let Some(tool) = p.hand else { return };
        let Some(t) = self.thing(tool) else { return };
        let td = self.defs.thing(t.def);
        let wear = td.tool.as_ref().map_or(0, |d| d.wear) as i32;
        let hp = t.hp - wear;
        if hp > 0 {
            if let Ok(mut t) = self.ecs.get::<&mut Thing>(tool) {
                t.hp = hp;
            }
            return;
        }
        let what = match self.ecs.get::<&MadeOf>(tool).ok().map(|m| self.defs.thing(m.0).label.clone()) {
            Some(m) => format!("{m} {}", td.label),
            None => td.label.clone(),
        };
        self.message(format!("{}'s {what} broke.", p.name), MsgKind::Bad);
        p.hand = None;
        self.despawn_thing(tool);
    }

    // ------------------------------------------------------------ work

    /// One unit of work on `e`, which stands at `at`, by a worker at `from`.
    /// `total` is asked for only when the work starts, or when it was for
    /// another designation. Returns the work after this unit.
    /// `amount` is how many units the worker managed this tick (skill,
    /// `Pawn::work_amount`); it can be 0.
    pub fn work_on(
        &mut self,
        e: Entity,
        at: IVec,
        from: IVec,
        designation: Option<DefId>,
        amount: u32,
        total: impl FnOnce(&World) -> u32,
    ) -> Option<Work> {
        let side = Side::of(at, from);
        let same = match self.ecs.get::<&mut Work>(e) {
            Ok(mut w) if w.designation == designation => {
                w.done = (w.done + amount).min(w.total);
                w.side = side;
                Some(*w)
            }
            _ => None,
        };
        let w = match same {
            Some(w) => w,
            None => {
                // Other work was here: it goes on the shelf if it got
                // anywhere, and this work comes off it if it was there.
                let was = self.ecs.get::<&Work>(e).ok().map(|k| *k);
                let back = was.and_then(|k| k.other).filter(|o| o.designation == designation);
                let shelf = match was {
                    Some(k) if k.done > 0 => Some(Shelved { designation: k.designation, done: k.done, total: k.total }),
                    Some(k) => k.other.filter(|o| o.designation != designation),
                    None => None,
                };
                let (done, total) = back.map_or_else(|| (0, total(self).max(1)), |b| (b.done, b.total));
                let w = Work { done: (done + amount).min(total), total, designation, side, other: shelf };
                self.ecs.insert_one(e, w).ok()?;
                w
            }
        };
        self.mark_worksite(e, at);
        Some(w)
    }

    /// The work on `e` is done but `e` stays (a bush picked, a tree's
    /// branches gathered): what grows back starts from scratch, and any
    /// work shelved for this comes back.
    pub fn finish_work(&mut self, e: Entity) {
        let shelved = self.ecs.get::<&Work>(e).ok().and_then(|k| k.other.map(|o| (o, k.side)));
        match shelved {
            Some((o, side)) => {
                let k = Work { done: o.done, total: o.total, designation: o.designation, side, other: None };
                let _ = self.ecs.insert_one(e, k);
            }
            None => {
                let _ = self.ecs.remove_one::<Work>(e);
            }
        }
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
                for &(w, l) in &p.priorities {
                    h = crate::rng::mix(h ^ (w as u64) << 8 ^ l as u64);
                }
                for &(s, xp) in &p.skills {
                    h = crate::rng::mix(h ^ (s as u64) << 40 ^ xp as u64);
                }
                h = crate::rng::mix(h ^ p.work_frac as u64);
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
        h = self.zones.hash(h);
        h = crate::rng::mix(h ^ self.stance.map_or(0x57a2, |s| s as u64));
        for (k, v) in &self.data {
            h = v.hash(k.bytes().fold(h, |h, b| crate::rng::mix(h ^ b as u64)));
        }
        h
    }
}
