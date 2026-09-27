//! Typed definitions. Mods write these as TOML; the engine only ever sees
//! the merged, patched, validated result.
//!
//! Unknown fields are ignored on purpose: a plugin may annotate another
//! mod's defs with data that only it understands.

use crate::look::{Look, LookDef};
use crate::terms::{InputDef, TermDef, Terms, TermsDef};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

pub type DefId = u16;

fn d100() -> u32 {
    100
}
fn d1() -> u32 {
    1
}
fn dtrue() -> bool {
    true
}
fn d1f() -> f64 {
    1.0
}
/// One table or a list of them: `harvest = { ... }` and `harvest = [{ ... }, { ... }]`.
fn one_or_many<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(t) => vec![t],
        OneOrMany::Many(v) => v,
    })
}

fn d075() -> f64 {
    0.75
}
fn dsize() -> f32 {
    0.35
}
fn d01() -> f64 {
    0.1
}
fn drange01() -> [f64; 2] {
    [0.0, 1.0]
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemCount {
    pub thing: String,
    pub count: u32,
}

// ---------------------------------------------------------------- terrain

#[derive(Deserialize, Clone, Debug)]
pub struct TerrainDef {
    pub id: String,
    pub label: String,
    pub color: String,
    /// Movement cost in percent. 0 means impassable.
    #[serde(default = "d100")]
    pub path_cost: u32,
    #[serde(default)]
    pub gen: Option<TerrainGen>,
    /// Rock: the terrain fills its cell (DESIGN.md §6d). It blocks and bounds
    /// rooms like a wall, and costs nothing until someone works it.
    #[serde(default)]
    pub solid: Option<SolidDef>,
    /// Named numbers terms read with `{ terrain = "fertility" }`. Any names;
    /// one this terrain doesn't give reads 0.
    #[serde(default)]
    pub props: BTreeMap<String, f64>,
    /// What `{ near = "water" }` measures the distance to.
    #[serde(default)]
    pub tags: Vec<String>,
    /// No floor: a pit or a shaft. Nothing walks it; light and water cross
    /// it to the level below (DESIGN.md §6d).
    #[serde(default)]
    pub air: bool,
    #[serde(skip)]
    pub rgb: [u8; 3],
    /// `props` in fixed point, indexed like `DefDb::terrain_props`.
    #[serde(skip)]
    pub props_q: Vec<i64>,
}

/// What a solid terrain is when someone works it, and what it leaves.
/// Without them it can't be worked at all: bedrock.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SolidDef {
    /// The thing that stands in the cell once a pawn is set to work it: its
    /// harvests, look, wear and wind all come from there, so a patch to the
    /// thing reaches every cell of the rock.
    #[serde(default)]
    pub thing: Option<String>,
    /// The terrain left when that thing is gone.
    #[serde(default)]
    pub leaves: Option<String>,
    #[serde(skip)]
    pub thing_r: Option<DefId>,
    #[serde(skip)]
    pub leaves_r: Option<DefId>,
}

/// A level below the surface, as map generation fills it (DESIGN.md §6d).
/// The deepest stratum sets how far down the map goes.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StratumDef {
    pub id: String,
    /// Below the surface: -1 is the first level down.
    pub level: i32,
    /// What the level is made of, in patches.
    pub fill: Vec<StratumFill>,
    /// The ring of cells at the level's edge. Only the surface has a map
    /// edge that raids cross, so below it this is rock nothing can work.
    pub edge: String,
    /// How big a patch of one fill is, in cells.
    #[serde(default = "d_patch")]
    pub patch: f64,
    #[serde(skip)]
    pub edge_r: DefId,
}

fn d_patch() -> f64 {
    12.0
}

/// One terrain a stratum is made of, and how much of it.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StratumFill {
    pub terrain: String,
    pub weight: f64,
    /// Only under these surface terrains (clay under marsh and water);
    /// empty is anywhere.
    #[serde(default)]
    pub under: Vec<String>,
    #[serde(skip)]
    pub terrain_r: DefId,
    #[serde(skip)]
    pub under_r: Vec<DefId>,
}

/// Map generation band: the highest-priority terrain whose ranges contain
/// the cell's elevation and moisture wins.
#[derive(Deserialize, Clone, Debug)]
pub struct TerrainGen {
    #[serde(default = "drange01")]
    pub elevation: [f64; 2],
    #[serde(default = "drange01")]
    pub moisture: [f64; 2],
    #[serde(default)]
    pub priority: i32,
}

// ---------------------------------------------------------------- things

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Plant,
    Rock,
    Building,
    Item,
    /// Built ground: lives under fixtures and items, and its `path_cost`
    /// replaces the terrain's while it is there.
    Floor,
}

fn one_cell() -> [u32; 2] {
    [1, 1]
}

/// The largest side a thing may have, in cells.
pub const MAX_SIZE: u32 = 8;

#[derive(Deserialize, Clone, Debug)]
pub struct ThingDef {
    pub id: String,
    pub label: String,
    pub color: String,
    pub category: Category,
    /// How it is drawn (look.rs).
    #[serde(default)]
    pub look: LookDef,
    /// Gone in API 0.4, for `look`. Read only to say so.
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    pub market_value: f64,
    /// Blocks movement (walls, rocks).
    #[serde(default)]
    pub blocks: bool,
    /// Cells it covers, [w, h] from its anchor (its `pos`) right and down.
    /// It occupies every one in the fixture layer, so pathing, rooms and
    /// fields never learn about footprints (DESIGN.md §6a).
    #[serde(default = "one_cell")]
    pub size: [u32; 2],
    /// How much of the wind it stops, 0 to 1: walls and rock all of it,
    /// trees some. What's behind it downwind is sheltered.
    #[serde(default)]
    pub blocks_wind: f64,
    /// How its light looks, if it gives any (DESIGN.md §6e). The renderer's
    /// alone: the sim only knows how bright and how far.
    #[serde(default)]
    pub glow: Option<GlowDef>,
    /// How tall it stands, in cells, for the shadows the renderer casts
    /// from it (DESIGN.md §6e). Blocking things and plants read it: unset, a
    /// blocking thing stands one cell tall, and a plant casts no shadow.
    /// Anything else casts none either way.
    #[serde(default)]
    pub height: Option<f64>,
    /// Extra movement cost in percent (doors, trees).
    #[serde(default)]
    pub path_cost: u32,
    /// Passable, but bounds rooms like a wall does.
    #[serde(default)]
    pub door: bool,
    /// A way down: the thing stands in its cell and the one below it, and
    /// pawns step between them (DESIGN.md §6d).
    #[serde(default)]
    pub portal: Option<PortalDef>,
    #[serde(default = "d100")]
    pub hp: u32,
    #[serde(default = "d1")]
    pub stack_limit: u32,
    /// Natural features don't count toward colony wealth.
    #[serde(default)]
    pub natural: bool,
    /// Ways to harvest it, one per designation: an oak can be gathered for
    /// branches and chopped for wood.
    #[serde(default, deserialize_with = "one_or_many")]
    pub harvest: Vec<HarvestDef>,
    pub build: Option<BuildDef>,
    /// Present on things a pawn can hold and work with (DESIGN.md §4e).
    #[serde(default)]
    pub tool: Option<ToolDef>,
    pub food: Option<FoodDef>,
    /// An item that spoils: it loses condition as it lies, and rots away
    /// at none (DESIGN.md §4f).
    #[serde(default)]
    pub spoil: Option<SpoilDef>,
    pub bed: Option<BedDef>,
    /// Present on items that things can be built out of.
    #[serde(default)]
    pub stuff: Option<StuffDef>,
    pub spawn: Option<SpawnDef>,
    /// A plant that grows with the weather (DESIGN.md §4c).
    #[serde(default)]
    pub grow: Option<GrowDef>,
    /// Cells a pawn occupies to use this thing. A bed's is the bed; a
    /// chair's is the chair; a workbench's would be in front. A thing with
    /// no spots cannot be used, only had.
    #[serde(default)]
    pub spots: Vec<SpotDef>,
    /// Free labels other defs can ask for by name ("table"). The engine
    /// matches the strings and never reads them.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Field sources: a campfire emits heat and light.
    #[serde(default)]
    pub emit: Vec<EmitDef>,
    /// It holds stacks: a basket, a crate, a shelf (DESIGN.md §4f).
    #[serde(default)]
    pub store: Option<StoreDef>,
    /// What this piece does to a room it helps enclose, per field. A wall
    /// leaks a little, a window leaks a lot and lets daylight through.
    #[serde(default)]
    pub boundary: Vec<BoundaryDef>,
    /// It holds a roof up (DESIGN.md §6c): walls, pillars, rock.
    #[serde(default)]
    pub support: Option<SupportDef>,
    /// It warms (or otherwise comforts) what a need reads: it emits a
    /// positive amount into a field a need is satisfied by, or that one is
    /// worked out from. Until the colony has one, building one is urgent.
    #[serde(skip)]
    pub comforts: bool,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub look_r: Look,
    /// Its tags that some room role asks for, as indices into
    /// `DefDb::room_tags`.
    #[serde(skip)]
    pub room_tags_r: Vec<u16>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EmitDef {
    pub field: String,
    /// Strength at the source, in the field's units.
    pub amount: f64,
    /// Cells of walking distance it reaches, fading linearly to zero.
    pub radius: u32,
    /// Room fields: stop pushing the room past this value (a campfire can't
    /// heat a hut beyond 24°; a cooler with a negative amount stops at its
    /// cap from above). Unset: no limit.
    #[serde(default)]
    pub cap: Option<f64>,
    #[serde(skip)]
    pub field_r: DefId,
}

#[derive(Deserialize, Clone, Debug)]
pub struct HarvestDef {
    /// Which designation marks this for work (e.g. "chop", "mine").
    pub designation: String,
    /// Work ticks.
    pub work: u32,
    #[serde(default)]
    pub yields: Vec<ItemCount>,
    /// Destroyed on harvest (trees, rocks) or regrows (bushes).
    #[serde(default = "dtrue")]
    pub destroy: bool,
    #[serde(default)]
    pub regrow_days: f64,
    /// Tool tags the worker must hold ("chopping"): core's shared names.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(skip)]
    pub requires_r: ToolMask,
    #[serde(skip)]
    pub desig_r: DefId,
    #[serde(skip)]
    pub yields_r: Vec<(DefId, u32)>,
    /// Its place in the thing's list.
    #[serde(skip)]
    pub index: usize,
}

/// How regrowth and jobs name one of a thing's harvests: by its
/// designation, which a load maps like any def reference, or `None` for the
/// thing's first harvest. That is how every save named it before a thing
/// could have several, so those saves read the same.
pub type HarvestKey = Option<DefId>;

impl HarvestDef {
    pub fn key(&self) -> HarvestKey {
        (self.index > 0).then_some(self.desig_r)
    }
}

impl ThingDef {
    /// Its size turned `facing` quarter turns clockwise (DESIGN.md §6c): a
    /// 2×1 bench turned once is 1×2.
    pub fn size_facing(&self, facing: u8) -> [u32; 2] {
        let [w, h] = self.size;
        if facing & 1 == 1 {
            [h, w]
        } else {
            [w, h]
        }
    }

    /// Every cell it covers facing `facing`, with the footprint's top-left
    /// at `at`, row by row. Built on `offset`, so a cell keeps its level.
    pub fn footprint(&self, at: crate::IVec, facing: u8) -> impl Iterator<Item = crate::IVec> {
        let [w, h] = self.size_facing(facing);
        (0..h as i32).flat_map(move |y| (0..w as i32).map(move |x| at.offset(x, y)))
    }

    /// A cell written in the def's own frame (from its anchor, facing
    /// south: a bench's spot below it) turned `facing` quarter turns
    /// clockwise inside the footprint. Cells outside it turn with it.
    pub fn turn(&self, (dx, dy): (i32, i32), facing: u8) -> (i32, i32) {
        let [w, h] = self.size.map(|v| v as i32);
        match facing & 3 {
            0 => (dx, dy),
            1 => (h - 1 - dy, dx),
            2 => (w - 1 - dx, h - 1 - dy),
            _ => (dy, w - 1 - dx),
        }
    }

    /// The harvest a designation marks this thing for.
    pub fn harvest_for(&self, designation: DefId) -> Option<&HarvestDef> {
        self.harvest.iter().find(|h| h.desig_r == designation)
    }

    pub fn harvest_by_key(&self, key: HarvestKey) -> Option<&HarvestDef> {
        match key {
            None => self.harvest.first(),
            Some(d) => self.harvest_for(d),
        }
    }
}

/// Tool tags as bits, one per tag name any def mentions.
pub type ToolMask = u64;

/// What a tool does, and how well.
#[derive(Deserialize, Clone, Debug)]
pub struct ToolDef {
    /// What it does, in core's shared names (docs/modding/vocabulary.md).
    pub tags: Vec<String>,
    /// Work per tick against bare hands' 1, before its material's
    /// `tool_speed` factor.
    #[serde(default = "d1f")]
    pub speed: f64,
    /// Hit points a finished job costs it. At none left, it breaks.
    #[serde(default)]
    pub wear: u32,
    #[serde(skip)]
    pub tags_r: ToolMask,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BuildDef {
    /// A fixed recipe: the parts. With `stuff`, what goes in besides the
    /// material: a plank wall is planks and nails (DESIGN.md §4f).
    #[serde(default)]
    pub cost: Vec<ItemCount>,
    /// Built out of whatever matches: the def says how much, the player
    /// says of what. A wall is 25 of something structural, not 25 wood.
    /// The material sets the factors; `cost` adds the parts.
    #[serde(default)]
    pub stuff: Option<StuffCost>,
    pub work: u32,
    /// Fraction of the cost that comes back when it is taken down.
    #[serde(default = "d075")]
    pub refund: f64,
    /// Toolbar group.
    #[serde(default)]
    pub menu: String,
    /// Costs nothing: a crafting spot marked on the ground. Said outright,
    /// so a forgotten `cost` isn't a free building.
    #[serde(default)]
    pub free: bool,
    /// Tool tags the builder holds a tool with, as a harvest's `requires`.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Building it digs the cell below out first, with the work and tool
    /// that cell's rock asks (DESIGN.md §6d).
    #[serde(default)]
    pub dig: Option<DigDef>,
    /// It stands over air, and is footing there: a bridge (a floor) or a
    /// drawbridge (a door). Planned only over air, never on ground
    /// (DESIGN.md §6d).
    #[serde(default)]
    pub spans: bool,
    #[serde(skip)]
    pub cost_r: Vec<(DefId, u32)>,
    #[serde(skip)]
    pub requires_r: ToolMask,
}

/// A way between levels.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PortalDef {
    /// Move cost in percent, as a terrain's. At least 200: a level is 20 in
    /// A*'s estimate, and the estimate must never be more than a real step.
    pub cost: u32,
}

/// What a dig leaves.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct DigDef {
    /// The air terrain the cell dug from becomes: a pit, and the thing is
    /// gone. Without it, the thing stays: stairs, a ladder.
    #[serde(default)]
    pub hole: Option<String>,
    #[serde(skip)]
    pub hole_r: Option<DefId>,
}

/// How much material a buildable takes, and what kind will do.
#[derive(Deserialize, Clone, Debug)]
pub struct StuffCost {
    /// Matched against an item's `stuff.categories`.
    pub category: String,
    pub count: u32,
}

/// A cell a pawn stands or sits in to use a thing, relative to it.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SpotDef {
    #[serde(default)]
    pub dx: i32,
    #[serde(default)]
    pub dy: i32,
    /// Only a spot when a thing carrying this tag stands next to it: a
    /// chair is a seat at a table and a stool in a field otherwise.
    #[serde(default)]
    pub beside: String,
}

/// One piece of a room's boundary, as a field sees it. The room's numbers
/// are the average `leak` and the summed `pass` of every piece around it,
/// each scaled by the material factor the field names.
#[derive(Deserialize, Clone, Debug)]
pub struct BoundaryDef {
    pub field: String,
    /// How leaky this piece is, as a multiple of the field's leak. 1.0 is
    /// exactly the field's leak; divided by the material factor.
    #[serde(default = "d1f")]
    pub leak: f64,
    /// Fraction of the outdoor value this piece lets into the room, for
    /// fields that are otherwise dark indoors. Multiplied by the factor.
    #[serde(default)]
    pub pass: f64,
    #[serde(skip)]
    pub field_r: DefId,
}

/// How far a piece holds the roof up: every cell within `span` of it, by
/// Chebyshev distance, is roofed. Its material's `span` factor scales it,
/// so a branch wall holds less than a stone one.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SupportDef {
    pub span: f64,
}

/// Longest roof span a piece can have, in cells.
pub const MAX_SPAN: u8 = 12;

/// An item that things can be built out of.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct StuffDef {
    /// What this material will do for: "structural", "fine", whatever a
    /// mod invents. The engine never reads the strings, only matches them.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Named multipliers on the stats of anything built from it. The engine
    /// carries them and never interprets them (0213).
    #[serde(default)]
    pub factors: HashMap<String, f64>,
    /// How things built of it look on the plan (DESIGN.md §6c).
    #[serde(default)]
    pub look: StuffLookDef,
    /// Tool tags the builder holds a tool with when building of it, on top
    /// of the building's own `build.requires`: ashlar wants a maul.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(skip)]
    pub requires_r: ToolMask,
}

/// A material's look: the patterns its walls and floors are drawn in, and
/// the roof a house of it gets.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct StuffLookDef {
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub floor: Option<String>,
    #[serde(default)]
    pub roof: Option<String>,
    #[serde(skip)]
    pub pattern_r: crate::look::Pattern,
    #[serde(skip)]
    pub floor_r: crate::look::Pattern,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FoodDef {
    /// Fraction of a full stomach restored per unit.
    pub nutrition: f64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BedDef {
    /// Rest-recovery multiplier versus sleeping on the ground (1.0).
    pub rest_rate: f64,
}

fn dhalf() -> f64 {
    0.5
}

/// How a plant grows: from a seedling at 0 to grown at 1, at a speed its
/// `rate` terms give where it stands, losing health by its `harm` terms.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GrowDef {
    /// Game days from seed to grown at a rate of 1.
    pub days: f64,
    /// How fast it grows, as terms read at its cell: 1 is `days`, 0 or less
    /// holds it still (dormant). No terms is always 1.
    #[serde(default)]
    pub rate: TermsDef,
    /// Health lost a day, as terms read at its cell (1 is all of it). At
    /// none left it dies. Frost on a tender plant, drought, flooding.
    #[serde(default)]
    pub harm: TermsDef,
    /// Health regained a day while nothing harms it.
    #[serde(default = "dhalf")]
    pub heal: f64,
    /// How many sizes it is drawn at on the way to grown.
    #[serde(default = "d4u")]
    pub stages: u32,
    /// Where a harvest it survives (`destroy = false`) sets it back to: its
    /// crop grows again with it. A harvest that gives `regrow_days` regrows
    /// by those days instead, as on a plant that doesn't grow.
    #[serde(default = "dhalf")]
    pub after_harvest: f64,
    #[serde(skip)]
    pub rate_terms: Terms,
    #[serde(skip)]
    pub harm_terms: Terms,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SpawnDef {
    pub terrain: Vec<String>,
    /// Chance per matching cell at map generation.
    pub density: f64,
    /// Keeps reseeding slowly during play.
    #[serde(default)]
    pub spread: bool,
    #[serde(skip)]
    pub terrain_r: Vec<DefId>,
}

// ---------------------------------------------------------------- creatures

#[derive(Deserialize, Clone, Debug)]
pub struct CreatureDef {
    pub id: String,
    pub label: String,
    pub color: String,
    #[serde(default = "dsize")]
    pub size: f32,
    /// Ticks to cross one open cell.
    pub speed: u32,
    pub max_hp: i32,
    pub melee_damage: i32,
    pub melee_cooldown: u32,
    /// Can do work, join colonies, raid.
    #[serde(default)]
    pub intelligent: bool,
    /// Attacks intelligent creatures on sight.
    #[serde(default)]
    pub aggressive: bool,
    /// Runs from attackers instead of fighting back.
    #[serde(default)]
    pub flees: bool,
    /// Breaks off fighting below this fraction of max hp: colonists run and
    /// heal, hostiles leave the map. 0 fights to the death.
    #[serde(default)]
    pub retreat_below: f64,
    /// For messages ("wolves"). Defaults to label + "s".
    #[serde(default)]
    pub plural: String,
    #[serde(default)]
    pub market_value: f64,
    #[serde(default)]
    pub butcher: Vec<ItemCount>,
    #[serde(default)]
    pub needs: Vec<String>,
    pub spawn: Option<CreatureSpawn>,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub butcher_r: Vec<(DefId, u32)>,
    #[serde(skip)]
    pub needs_r: Vec<DefId>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CreatureSpawn {
    pub terrain: Vec<String>,
    pub groups: u32,
    pub group: [u32; 2],
    #[serde(skip)]
    pub terrain_r: Vec<DefId>,
}

// ---------------------------------------------------------------- needs

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Satisfier {
    Food,
    Rest,
    /// Driven by a field layer: drains outside `comfort`, recovers inside it.
    Field,
}

impl Satisfier {
    /// As a def writes it.
    pub fn name(self) -> &'static str {
        match self {
            Satisfier::Food => "food",
            Satisfier::Rest => "rest",
            Satisfier::Field => "field",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct NeedDef {
    pub id: String,
    pub label: String,
    pub color: String,
    pub satisfier: Satisfier,
    pub days_to_empty: f64,
    /// Pawn goes looking to satisfy the need below this fraction.
    pub seek_below: f64,
    #[serde(default)]
    pub empty_damage_per_day: f64,
    /// For `satisfier = "field"`: which field, and the comfortable range.
    #[serde(default)]
    pub field: String,
    #[serde(default)]
    pub comfort: [f64; 2],
    /// For field needs, `days_to_empty` is the rate when 10 units outside
    /// comfort; this is how fast it refills inside comfort.
    #[serde(default = "d01")]
    pub recover_days: f64,
    /// What a pawn who can talk says as the need drops below a level.
    #[serde(default)]
    pub say: Option<NeedSay>,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub field_r: DefId,
}

/// Lines a need speaks: one, picked by the pawn and the moment, each time
/// the need drops below `below`. Not again until it has been above it.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct NeedSay {
    /// A fraction of full, 0 to 1.
    pub below: f64,
    pub lines: Vec<String>,
    /// How long a line stays up, in ticks.
    #[serde(default = "d_say_ticks")]
    pub ticks: u32,
}

fn d_say_ticks() -> u32 {
    600
}

// ---------------------------------------------------------------- fields

/// The longest lee a shelter field may cast, in cells.
pub const MAX_LEE: u32 = 64;

/// Where a field's per-cell value comes from.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// Its outdoor value, emitters' stamps and rooms.
    #[default]
    Ambient,
    /// How exposed a cell is to the wind, in percent: 100 in the open, less
    /// in the lee of what blocks the wind, 0 in an enclosed room. The
    /// direction comes from the field named in `from`.
    Shelter,
    /// Worked out on read from other fields, by its `value` terms: nothing
    /// is stored. A `field` input reads the other field at the same cell,
    /// so it's the room's value indoors and the lee's in the lee.
    Derived,
    /// Stored per cell, and changed by its `rate` terms: it remembers. Rain
    /// yesterday is wet ground today. Each cell is worked out once a
    /// `period_minutes`, a slice of the map each tick.
    Stock,
}

fn d60f() -> f64 {
    60.0
}

/// Which levels a stock field keeps a value on.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StockLevels {
    /// The surface only: weather on the ground. It reads 0 elsewhere.
    #[default]
    Surface,
    /// Every level, for what lies underground too.
    All,
}

fn d6() -> u32 {
    6
}

/// What a field is inside an enclosed room.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndoorMode {
    /// Same as outdoors (noise, danger).
    #[default]
    Outdoor,
    /// Zero: only emitters count (light indoors comes from lamps).
    None,
    /// Each room holds its own value, leaking toward outdoors (temperature).
    Room,
}

/// A field's outdoor value: a constant, or labelled terms (see `terms`).
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AmbientDef {
    Const(f64),
    Terms(TermsDef),
}

impl Default for AmbientDef {
    fn default() -> Self {
        AmbientDef::Const(0.0)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct FieldDef {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub unit: String,
    /// Outdoor value: a constant, or terms over the time of day and year.
    /// Plugins add named contributions on top (`rim.push_ambient`).
    #[serde(default)]
    pub ambient: AmbientDef,
    /// A derived field's value, as terms; read at a cell, `field` inputs
    /// read that cell. Outdoors it's also the field's outdoor value.
    #[serde(default)]
    pub value: Option<TermsDef>,
    #[serde(default)]
    pub indoor: IndoorMode,
    #[serde(default)]
    pub kind: FieldKind,
    /// Shelter fields: the field whose outdoor value is the wind's
    /// direction, in degrees it blows toward (0 east, 90 south).
    #[serde(default)]
    pub from: String,
    #[serde(skip)]
    pub from_r: usize,
    /// Shelter fields: how many cells a full blocker's lee reaches.
    #[serde(default = "d6")]
    pub lee: u32,
    /// Room fields: fraction of the gap to outdoors closed per hour.
    #[serde(default)]
    pub leak_per_hour: f64,
    /// Room fields: the same as terms over the outdoor values, so wind (or
    /// anything else a mod adds) can make rooms draftier. Replaces
    /// `leak_per_hour`; a field gives one or the other.
    #[serde(default)]
    pub leak: Option<TermsDef>,
    /// Room fields: how strongly emitters inside push the room's value.
    #[serde(default)]
    pub room_gain: f64,
    /// Which material factor scales this field's boundary pieces: a better
    /// material leaks less and passes more. Content names it; the engine
    /// only looks it up. Empty means materials do not matter to this field.
    #[serde(default)]
    pub boundary_factor: String,
    /// Overlay colour ramp across `range`.
    pub range: [f64; 2],
    pub color_low: String,
    pub color_high: String,
    /// Show the outdoor value in the top bar.
    #[serde(default)]
    pub hud: bool,
    /// Offer a map overlay (`O`). Off for values that are the same
    /// everywhere, such as cloud cover.
    #[serde(default = "dtrue")]
    pub overlay: bool,
    /// Stock fields: how the stored value changes, in its units per game
    /// hour, as terms read at the cell. Emitters on the field add to it.
    #[serde(default)]
    pub rate: Option<TermsDef>,
    /// Stock fields: what the value settles to, as terms read at the cell.
    /// The rate reads it as `base`, and the gap as `above_base`.
    #[serde(default, rename = "base")]
    pub settle: Option<TermsDef>,
    /// Stock fields: each cell's value when the map is made.
    #[serde(default)]
    pub init: Option<TermsDef>,
    /// Stock fields: every cell is worked out once in this many game minutes.
    #[serde(default = "d60f")]
    pub period_minutes: f64,
    /// Stock fields: the surface, or every level.
    #[serde(default)]
    pub levels: StockLevels,
    #[serde(skip)]
    pub rate_terms: Terms,
    #[serde(skip)]
    pub settle_terms: Terms,
    #[serde(skip)]
    pub init_terms: Terms,
    /// `period_minutes` in ticks.
    #[serde(skip)]
    pub period: u64,
    /// Compiled `ambient` terms (empty for a constant).
    #[serde(skip)]
    pub terms: Terms,
    /// Compiled `leak` terms (empty: `leak_per_hour` is the leak).
    #[serde(skip)]
    pub leak_terms: Terms,
    /// The constant part of `ambient` (0 when it's terms).
    #[serde(skip)]
    pub base: f64,
    #[serde(skip)]
    pub rgb_low: [u8; 3],
    #[serde(skip)]
    pub rgb_high: [u8; 3],
}

// ---------------------------------------------------------------- calendar

fn d60() -> u32 {
    60
}

/// The year: how long it is, what its seasons are called, and where the
/// game starts in it. Core defines one; a plugin patches it.
#[derive(Deserialize, Clone, Debug)]
pub struct CalendarDef {
    pub id: String,
    #[serde(default = "d60")]
    pub year_days: u32,
    /// Equal parts of the year, in order.
    #[serde(default)]
    pub seasons: Vec<String>,
    /// Day of the year (0-based) that the game starts on.
    #[serde(default)]
    pub start_day: u32,
}

impl Default for CalendarDef {
    fn default() -> Self {
        CalendarDef { id: "default".into(), year_days: 60, seasons: vec!["year".into()], start_day: 0 }
    }
}

// ---------------------------------------------------------------- sky

fn dnight() -> String {
    "#4a5478".into()
}
fn dfire() -> String {
    "#ffb060".into()
}
fn dshare() -> f64 {
    0.2
}

/// How the renderer colours light. The sim ignores it: light is a scalar in
/// the simulation, and colour is the renderer's business.
#[derive(Deserialize, Clone, Debug)]
pub struct SkyDef {
    pub id: String,
    /// Colour tints over the day, by label, so a mod can add one (a green
    /// moon) without reshaping the others. Each is a colour and a strength
    /// (terms over the hour, year and outdoor values, 0..1).
    #[serde(default)]
    pub tint: BTreeMap<String, TintDef>,
    /// The darkest the world gets: moonlight and starlight.
    #[serde(default = "dnight")]
    pub night: String,
    /// Colour of stamped light (fires).
    #[serde(default = "dfire")]
    pub firelight: String,
    /// Share of daylight a roofed room gets through its walls and door. Its
    /// windows add the pass their boundary gives the light field, so the
    /// renderer's indoor share is this plus that, at most 1.
    #[serde(default = "dshare")]
    pub indoor_share: f64,
    /// Where the sun crosses the sky, for the shadows the renderer casts
    /// (DESIGN.md §6e). How bright it is stays the `daylight` field; this
    /// is only where it is. Unset: no sun shadows.
    #[serde(default)]
    pub sun: Option<SunPath>,
    #[serde(skip)]
    pub rgb_night: [u8; 3],
    #[serde(skip)]
    pub rgb_fire: [u8; 3],
}

/// How a thing that gives light looks alight.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlowDef {
    #[serde(default)]
    pub flicker: Flicker,
}

/// How a light moves: a flame that dances, or a steady glow.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Flicker {
    #[default]
    Fire,
    Steady,
}

/// A sky body's daily path: up at `rise`, down at `set` (hours), highest at
/// noon between them, crossing from azimuth `arc[0]` to `arc[1]` (degrees;
/// 0 is east, 90 south, the way the map's y grows).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SunPath {
    pub rise: f64,
    pub set: f64,
    /// Elevation at its highest, degrees.
    pub peak: f64,
    pub arc: [f64; 2],
}

impl Default for SkyDef {
    fn default() -> Self {
        SkyDef {
            id: "default".into(),
            tint: BTreeMap::new(),
            night: dnight(),
            firelight: dfire(),
            indoor_share: dshare(),
            sun: None,
            rgb_night: [74, 84, 120],
            rgb_fire: [255, 176, 96],
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TintDef {
    pub color: String,
    #[serde(default = "d_one")]
    pub scale: f64,
    #[serde(default)]
    pub of: Vec<InputDef>,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub strength: Terms,
}

fn d_one() -> f64 {
    1.0
}

// ---------------------------------------------------------------- designations

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Targets {
    /// Things with a harvest def that names this designation.
    #[default]
    Thing,
    /// Wild creatures that can be butchered.
    Creature,
    /// Things the colony built: walls, doors, furniture.
    Built,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DesignationDef {
    pub id: String,
    pub label: String,
    pub color: String,
    #[serde(default)]
    pub targets: Targets,
    /// The work type whose priority decides who does it (DESIGN.md §4d).
    pub work_type: String,
    /// The `[[work_style]]` its work looks like. Unset, it shows no wear.
    #[serde(default)]
    pub style: Option<String>,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub work_r: DefId,
    #[serde(skip)]
    pub style_r: Option<DefId>,
}

// ---------------------------------------------------------------- work styles

/// How work on a cell looks (DESIGN.md §6b): what each blow does, how the
/// thing wears as the work goes on, and how it leaves. Every name is one of
/// a fixed set of client mechanisms, like the look primitives, so a mod
/// picks and colours effects and never draws per frame.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WorkStyleDef {
    pub id: String,
    /// Work between two strikes.
    pub every: u32,
    #[serde(default)]
    pub strike: Vec<Strike>,
    #[serde(default)]
    pub wear: Wear,
    #[serde(default)]
    pub exit: Exit,
    /// The style every build uses. At most one style may say so.
    #[serde(default)]
    pub builds: bool,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Strike {
    /// The thing jolts away from the blow.
    Shake,
    /// Bits of what it yields, or is made of, fly toward the worker.
    Chips,
    Dust,
    /// Bits of the thing's own colour drop from it: leaves, needles.
    Shed,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Wear {
    #[default]
    None,
    /// Layers appear in their `grow` windows; taken down, they go in reverse.
    Grow,
    /// Cracks spread from the worked side.
    Cracks,
    /// It leans away from the worker.
    Lean,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Exit {
    #[default]
    None,
    /// Falls away from the worker, or to an open side.
    Fall,
    Crumble,
    /// Its yield pops out.
    Pop,
}

// ---------------------------------------------------------------- work

/// Kinds of work the engine itself hands out, which a work type claims by
/// name. Designations name their work type; this is for the rest.
pub const ENGINE_JOBS: &[&str] = &["build", "haul"];

fn d3() -> u8 {
    3
}

/// A column of the priority grid: a kind of work a colonist can be told to
/// do more or less of (DESIGN.md §4d).
#[derive(Deserialize, Clone, Debug)]
pub struct WorkTypeDef {
    pub id: String,
    pub label: String,
    /// A short glyph or sprite key for the column header.
    #[serde(default)]
    pub icon: String,
    /// The skill this work trains and is done faster with; empty for none.
    #[serde(default)]
    pub skill: String,
    #[serde(skip)]
    pub skill_r: Option<DefId>,
    /// The level a colonist starts at: 1 is first, `levels` last, 0 never.
    #[serde(default = "d3")]
    pub priority: u8,
    /// Breaks a tie between work types at the same level and the same
    /// distance: lower first. Within a level the nearest job wins
    /// (DESIGN.md §4d), so the leftmost column never beats one next door.
    #[serde(default)]
    pub order: i32,
    /// Engine jobs this work type covers, from `ENGINE_JOBS`: "build" is
    /// raising blueprints and bringing them materials.
    #[serde(default)]
    pub jobs: Vec<String>,
    /// What a planner (Auto, DESIGN.md §4d) reads about this work.
    #[serde(default)]
    pub auto: AutoDef,
}

/// A work type's numbers for a planner: how many waiting jobs one person
/// keeps up with, and how much a waiting job matters beside others. The
/// engine only passes them on.
#[derive(Deserialize, Clone, Debug)]
pub struct AutoDef {
    #[serde(default = "d4u")]
    pub per_person: u32,
    #[serde(default = "d1u")]
    pub weight: u32,
}

impl Default for AutoDef {
    fn default() -> Self {
        AutoDef { per_person: 4, weight: 1 }
    }
}

fn d4u() -> u32 {
    4
}

fn d1u() -> u32 {
    1
}

fn d4() -> u8 {
    4
}

/// Something a colonist gets better at by doing it: work of the work types
/// that name it goes faster, and the skill that says `melee` hits harder.
#[derive(Deserialize, Clone, Debug)]
pub struct SkillDef {
    pub id: String,
    pub label: String,
    /// This is the skill fighting hand to hand trains and uses.
    #[serde(default)]
    pub melee: bool,
}

/// How many priority levels there are, and what the player calls them.
/// Core says 4, named; a mod patches it to 9, and without names the UI
/// shows numbers.
#[derive(Deserialize, Clone, Debug)]
pub struct PriorityScaleDef {
    pub id: String,
    #[serde(default = "d4")]
    pub levels: u8,
    /// A name per level, first to last: "First", "Soon". Empty, or one per
    /// level.
    #[serde(default)]
    pub labels: Vec<String>,
}

impl Default for PriorityScaleDef {
    fn default() -> Self {
        PriorityScaleDef { id: "default".into(), levels: 4, labels: Vec::new() }
    }
}

impl PriorityScaleDef {
    /// What the player calls a level: its label, or its number; 0 is never.
    pub fn name(&self, level: u8) -> String {
        match level {
            0 => "never".into(),
            l => self.labels.get(l as usize - 1).cloned().unwrap_or_else(|| l.to_string()),
        }
    }
}

/// The levels stores sort by (DESIGN.md §4f): a stack only ever moves to a
/// store at a higher one. Core names five; a mod that wants three or nine
/// changes the list, and the UI follows.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StorePriorityDef {
    pub id: String,
    /// Lowest first.
    pub labels: Vec<String>,
    /// The level a new store starts at, an index into `labels`.
    pub default: u8,
}

impl Default for StorePriorityDef {
    fn default() -> Self {
        let labels = ["Low", "Normal", "Preferred", "Important", "Critical"];
        StorePriorityDef { id: "default".into(), labels: labels.map(String::from).to_vec(), default: 1 }
    }
}

/// A named set of priority rules the colony switches with one click:
/// the rules that name a stance hold while it's the colony's (DESIGN.md
/// §4d). A mod adds one with data alone.
#[derive(Deserialize, Clone, Debug)]
pub struct StanceDef {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: String,
    /// Where it sits in the stance bar: lower first. The first is the
    /// colony's stance in a new game.
    #[serde(default)]
    pub order: i32,
}

/// What a room is for (DESIGN.md §6c). In load order, the first role
/// whose needs a room meets names it. The engine only counts tags: what a
/// bedroom *means* is for mood, eras and the storyteller to read.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RoomRoleDef {
    pub id: String,
    pub label: String,
    /// How many things inside must carry each tag: `{ bed = 1, fire = 1 }`.
    pub needs: BTreeMap<String, u32>,
    /// Smallest room, in cells, that can take it.
    #[serde(default)]
    pub min_cells: u32,
    /// Only enclosed rooms can take it (the default); `false` lets an open
    /// yard be a pen or a camp.
    #[serde(default = "dtrue")]
    pub enclosed: bool,
    /// `needs` as (index into `DefDb::room_tags`, count).
    #[serde(skip)]
    pub needs_r: Vec<(u16, u32)>,
}

/// A house as text (DESIGN.md §6c): a grid of characters and what each
/// one builds, placed whole with `Command::PlacePlan` and turned with it.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlanDef {
    pub id: String,
    pub label: String,
    /// Rows, north first. `.` and space build nothing. A piece bigger than
    /// a cell is written at its anchor, and may be repeated over the rest
    /// of its footprint so the grid reads as the house does.
    pub grid: String,
    /// What each character builds.
    pub legend: BTreeMap<String, PieceDef>,
    /// Width and height of the grid, unturned.
    #[serde(skip)]
    pub size: [i32; 2],
    #[serde(skip)]
    pub pieces: Vec<Piece>,
}

/// One character of a plan's legend.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PieceDef {
    pub thing: String,
    /// Its material, for a thing built of one.
    #[serde(default)]
    pub stuff: Option<String>,
    /// Quarter turns clockwise, in the plan's own frame.
    #[serde(default)]
    pub facing: u8,
}

/// A piece of a plan: a thing at its anchor, in the plan's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub at: (i32, i32),
    pub thing: DefId,
    pub stuff: Option<DefId>,
    pub facing: u8,
}

impl PlanDef {
    /// Its pieces with the plan's corner at `at`, turned `facing` quarter
    /// turns clockwise: each keeps its place in the house, and turns too.
    pub fn placed(&self, defs: &DefDb, at: crate::IVec, facing: u8) -> Vec<Piece> {
        let [w, h] = self.size;
        let turn = |(x, y): (i32, i32)| match facing & 3 {
            0 => (x, y),
            1 => (h - 1 - y, x),
            2 => (w - 1 - x, h - 1 - y),
            _ => (y, w - 1 - x),
        };
        self.pieces
            .iter()
            .map(|p| {
                let [pw, ph] = defs.thing(p.thing).size_facing(p.facing).map(|v| v as i32);
                let (a, b) = (turn(p.at), turn((p.at.0 + pw - 1, p.at.1 + ph - 1)));
                Piece { at: (at.x + a.0.min(b.0), at.y + a.1.min(b.1)), facing: (p.facing + facing) & 3, ..*p }
            })
            .collect()
    }
}

/// When a priority rule holds: every condition it gives.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct WhenDef {
    /// Hours of the day, from the first up to the second; `[22, 6]` wraps
    /// past midnight.
    #[serde(default)]
    pub hours: Option<[u32; 2]>,
    /// Any of these seasons, by the calendar's names.
    #[serde(default)]
    pub season: Vec<String>,
    #[serde(default)]
    pub stance: Option<String>,
    /// A need of the colonist's, below or above a fraction of full.
    #[serde(default)]
    pub need: Option<String>,
    /// A colony reading scripts publish ("core:food_days"), below or above
    /// a value: a standing order (DESIGN.md §4d).
    #[serde(default)]
    pub reading: Option<String>,
    #[serde(default)]
    pub below: Option<f64>,
    #[serde(default)]
    pub above: Option<f64>,
    /// For a reading: once on, the rule holds until the reading gets back
    /// past this, so it doesn't flap at the mark (on under 5, off at 8).
    #[serde(default)]
    pub until: Option<f64>,
    /// A reading's marks in thousandths, as readings are kept: (on, off).
    #[serde(skip)]
    pub band_r: Option<Band>,
    #[serde(skip)]
    pub season_r: Vec<u32>,
    #[serde(skip)]
    pub stance_r: Option<DefId>,
    #[serde(skip)]
    pub need_r: Option<DefId>,
}

/// Where a reading rule turns on and off, in thousandths. `Below` holds
/// from under `on` until the reading is back at `off` or more; `Above`
/// the other way round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Below { on: i64, off: i64 },
    Above { on: i64, off: i64 },
}

impl Band {
    /// Whether the rule holds at `v`, given whether it held before.
    pub fn holds(self, v: i64, was: bool) -> bool {
        match (self, was) {
            (Band::Below { off, .. }, true) => v < off,
            (Band::Below { on, .. }, false) => v < on,
            (Band::Above { off, .. }, true) => v > off,
            (Band::Above { on, .. }, false) => v > on,
        }
    }
}

/// A reading as the sim keeps it: thousandths, so it hashes and compares
/// exactly on every platform.
pub fn milli(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// A work role (DESIGN.md §4d): a partial set of levels colonists belong
/// to, one role each. What it leaves out is the work type's default. The
/// colony keeps its own copy of each, seeded from here; see
/// `rules::WorkRole`.
#[derive(Deserialize, Clone, Debug)]
pub struct WorkRoleDef {
    pub id: String,
    pub label: String,
    /// The first by order is where colonists start.
    #[serde(default)]
    pub order: i32,
    /// Levels by work type: `{ build = 1, haul = 2 }`.
    #[serde(default)]
    pub priorities: BTreeMap<String, u8>,
    /// A planned role: this names the planner (`rim.planner`) that sets
    /// its members' levels each in-game hour, and `priorities` is unused.
    #[serde(default)]
    pub planner: Option<String>,
    /// `priorities` resolved, in work-type id order.
    #[serde(skip)]
    pub priorities_r: Vec<(DefId, u8)>,
}

/// Changes work priorities while its `when` holds: `set` puts a work type
/// at a level, `shift` moves it (negative is sooner). A work type a
/// colonist set to 0 stays 0 under a shift; only a `set` overrides never.
#[derive(Deserialize, Clone, Debug)]
pub struct PriorityRuleDef {
    pub id: String,
    /// How it reads in an explanation; the stance's label if it names one.
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub when: WhenDef,
    #[serde(default)]
    pub set: BTreeMap<String, u8>,
    #[serde(default)]
    pub shift: BTreeMap<String, i32>,
    #[serde(skip)]
    pub set_r: Vec<(DefId, u8)>,
    #[serde(skip)]
    pub shift_r: Vec<(DefId, i32)>,
}

// ---------------------------------------------------------------- start / names

#[derive(Deserialize, Clone, Debug)]
pub struct StartDef {
    pub id: String,
    pub creature: String,
    #[serde(default = "d1")]
    pub count: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub founder_damage_bonus: i32,
    #[serde(default)]
    pub items: Vec<ItemCount>,
    #[serde(skip)]
    pub creature_r: DefId,
}

#[derive(Deserialize, Clone, Debug)]
pub struct NamesDef {
    pub id: String,
    pub names: Vec<String>,
}

// ---------------------------------------------------------------- item categories

/// A shelf in the tree players filter items by (DESIGN.md §4f). An item is
/// in it by tag, by id, or by what it is (`with = ["food"]`, `stuff =
/// ["structural"]`), so a mod's new food lands under Food without a patch.
/// An item can sit in several. The sim never walks the tree: toggling one
/// sets the items under it, once.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ItemCategoryDef {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub parent: Option<String>,
    /// Where it sits among its siblings: lower first.
    #[serde(default)]
    pub order: i32,
    /// Items carrying any of these tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// These items, by id.
    #[serde(default)]
    pub things: Vec<String>,
    /// Items that have any of these blocks: "food", "tool" or "stuff".
    #[serde(default)]
    pub with: Vec<String>,
    /// Items that are material of any of these stuff categories.
    #[serde(default)]
    pub stuff: Vec<String>,
    /// Items no other category claims land here. At most one says so.
    #[serde(default)]
    pub rest: bool,
    #[serde(skip)]
    pub parent_r: Option<DefId>,
    /// Its children, in `order` order.
    #[serde(skip)]
    pub children: Vec<DefId>,
    /// The items directly in it, in def order.
    #[serde(skip)]
    pub items: Vec<DefId>,
}

/// A container: a thing whose slots hold stacks, so one cell can keep more
/// than the ground does. Its contents are stacks like any other, kept off
/// the item layer, so the grid still holds one stack per cell (§6a).
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct StoreDef {
    /// How many stacks it holds.
    pub slots: u32,
    /// Each slot holds this many times an item's `stack_limit`: a woodpile
    /// holds wood three stacks deep.
    #[serde(default = "d1")]
    pub stack_scale: u32,
    /// What it can ever take; the player's filter narrows it further.
    #[serde(default)]
    pub accepts: StoreAccepts,
    /// Its contents are out of the weather: what spoils reads `input =
    /// "sky"` as 0 in it, as under a roof.
    #[serde(default)]
    pub shelter: bool,
    /// How much longer what spoils keeps in it, as terms read at its cell:
    /// 2 spoils half as fast. A mod makes cold storage from temperature.
    /// No terms is 1.
    #[serde(default)]
    pub keeps: TermsDef,
    #[serde(skip)]
    pub keeps_terms: Terms,
    /// How the map shows what it holds: "fill" (its look by how full it
    /// is), "items" (the looks of what it holds), or "none".
    #[serde(default)]
    pub display: StoreDisplay,
    /// For "fill": how many steps from empty to full the look has.
    #[serde(default = "d4")]
    pub look_stages: u8,
    /// Items it can ever take, sorted.
    #[serde(skip)]
    pub accepts_r: Vec<DefId>,
}

/// How an item spoils: from whole to rotten in `days` at a rate of 1, at a
/// rate its terms give where it lies.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SpoilDef {
    /// Game days from whole to rotten at a rate of 1.
    pub days: f64,
    /// How fast it spoils, as terms read at its cell: warmth, rain on it
    /// (read `input = "sky"`, so a sheltering store keeps it dry). No
    /// terms is 1.
    #[serde(default)]
    pub rate: TermsDef,
    #[serde(skip)]
    pub rate_terms: Terms,
}

/// What a container can ever take. Empty means any item.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct StoreAccepts {
    /// Items with any of these tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// These items, by id.
    #[serde(default)]
    pub things: Vec<String>,
    /// Items in these categories or those under them.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Never items with any of these tags: `not_tags = ["bulky"]`.
    #[serde(default)]
    pub not_tags: Vec<String>,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StoreDisplay {
    #[default]
    Fill,
    Items,
    None,
}

/// The blocks a category's `with` can name.
const CATEGORY_WITH: &[&str] = &["food", "tool", "stuff"];

// ---------------------------------------------------------------- modifiers

/// A mod's contribution to a thing's stat: the stat pipeline (DESIGN.md
/// §6). While it is on, `value` is added to `stat` of `thing`. Scripts
/// switch a mod's own modifiers off and on by `group`
/// (`rim.set_modifiers`), and the switch is world state, saved by id.
/// `buildable` is such a stat: a buildable whose total is 0 or less can't
/// be placed, and `reason` says why. A modifier on a thing whose mod isn't
/// installed does nothing, so a mod can gate an optional mod's content.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ModifierDef {
    pub id: String,
    pub stat: String,
    pub thing: String,
    pub value: f64,
    /// What the player is told while it holds a thing back.
    #[serde(default)]
    pub reason: String,
    /// What a script switches it with, among its mod's modifiers.
    #[serde(default)]
    pub group: String,
    /// Whether it counts from the start, until a script switches it.
    #[serde(default = "dtrue")]
    pub on: bool,
    #[serde(skip)]
    pub thing_r: Option<DefId>,
}

// ---------------------------------------------------------------- database

/// What terms in a field's def can name: fields as its mod sees them, and
/// every terrain property and tag.
struct TermNames<'a> {
    field: &'a dyn Fn(&str) -> Option<usize>,
    props: &'a [String],
    tags: &'a [String],
}

impl crate::terms::Names for TermNames<'_> {
    fn field(&self, id: &str) -> Option<usize> {
        (self.field)(id)
    }
    fn prop(&self, name: &str) -> Option<usize> {
        self.props.binary_search_by(|p| p.as_str().cmp(name)).ok()
    }
    fn tag(&self, name: &str) -> Option<usize> {
        self.tags.binary_search_by(|t| t.as_str().cmp(name)).ok()
    }
}

#[derive(Default, Debug)]
pub struct DefDb {
    pub terrain: Vec<TerrainDef>,
    pub things: Vec<ThingDef>,
    pub creatures: Vec<CreatureDef>,
    pub needs: Vec<NeedDef>,
    pub designations: Vec<DesignationDef>,
    pub work_styles: Vec<WorkStyleDef>,
    /// The style builds use (`builds = true`).
    pub build_style: Option<DefId>,
    pub skills: Vec<SkillDef>,
    /// The skill that says `melee`, if any.
    pub melee_skill: Option<DefId>,
    /// Work types in load order; `work_order` has them in `order` order.
    pub work_types: Vec<WorkTypeDef>,
    pub work_order: Vec<DefId>,
    pub priority_scale: PriorityScaleDef,
    pub store_priority: StorePriorityDef,
    pub stances: Vec<StanceDef>,
    /// Work roles in load order; a colony copies them (`World::work_roles`).
    pub work_roles: Vec<WorkRoleDef>,
    /// Where colonists start: the first work role by `order`.
    pub default_work_role: Option<DefId>,
    /// Room roles in load order: the first a room meets names it.
    pub room_roles: Vec<RoomRoleDef>,
    /// Every tag some room role counts, in first-asked order.
    pub room_tags: Vec<String>,
    /// House plans, in load order.
    pub plans: Vec<PlanDef>,
    /// The stance a new colony starts in: the first by `order`.
    pub default_stance: Option<DefId>,
    pub priority_rules: Vec<PriorityRuleDef>,
    /// What any rule's conditions read, so the colony's rules are worked
    /// out again only when one of those changes.
    pub rules_read_hour: bool,
    pub rules_read_season: bool,
    /// The work type that raises blueprints, if any claims "build".
    pub build_work: Option<DefId>,
    /// The work type that carries items to stockpiles, if any claims "haul".
    pub haul_work: Option<DefId>,
    pub fields: Vec<FieldDef>,
    /// Fields in the order their ambient terms must be evaluated.
    pub ambient_order: Vec<usize>,
    /// Every terrain property any terrain gives, sorted: terms read them by
    /// index.
    pub terrain_props: Vec<String>,
    /// Every terrain tag, sorted.
    pub terrain_tags: Vec<String>,
    /// The tags some term reads with `near`: only these keep a distance grid.
    pub near_tags: Vec<usize>,
    /// The fields with `kind = "stock"`, in load order.
    pub stock_fields: Vec<usize>,
    /// What loading noticed but let through, for the mod loader to report.
    pub warnings: Vec<String>,
    pub calendar: CalendarDef,
    pub sky: SkyDef,
    pub start: Option<StartDef>,
    pub names: Vec<String>,
    /// The item category tree, in load order; `category_roots` has the top
    /// level in `order` order.
    pub item_categories: Vec<ItemCategoryDef>,
    /// The stat pipeline's modifiers, in load order.
    pub modifiers: Vec<ModifierDef>,
    /// By thing: the modifiers on it, as indices into `modifiers`.
    pub thing_modifiers: Vec<Vec<u16>>,
    /// The levels below the surface, in no particular order.
    pub strata: Vec<StratumDef>,
    pub category_roots: Vec<DefId>,
    /// Entries of the kinds mods declare (`[[kind]]`), by qualified kind
    /// ("weather:type"), in load order: plain data for scripts.
    pub mod_defs: BTreeMap<String, Vec<crate::data::Data>>,
    /// Labels things join up by (`look.join`), indexed by `Look::join`.
    pub join_groups: Vec<String>,
    /// Sprite keys looks use (`mod:name`), indexed by `Prim::Sprite::id`.
    pub sprites: Vec<String>,
    /// Each sprite's PNG, parallel to `sprites`; the modloader finds them.
    pub sprite_files: Vec<std::path::PathBuf>,
    /// Characters looks draw (`draw = "glyph"`), indexed by `Prim::Glyph::id`.
    pub glyphs: Vec<String>,
    /// Every tool tag any def names, sorted; a tag's bit is its index.
    pub tool_tags: Vec<String>,
    /// Qualified ids ("core:wall").
    index: HashMap<(&'static str, String), DefId>,
    /// Bare ids ("wall"), for tools and tests that don't care which mod.
    bare: HashMap<(&'static str, String), Vec<DefId>>,
}

/// The mod a qualified id belongs to: "core" for "core:wall".
pub fn home_of(id: &str) -> &str {
    id.split_once(':').map_or("", |(m, _)| m)
}

/// Resolve a def reference written in mod `home`: a qualified id is taken
/// as is, a bare one means `home`'s own def (DESIGN.md §10). The error
/// suggests the prefix when another mod has that id.
fn resolve_in(
    index: &HashMap<(&'static str, String), DefId>,
    bare: &HashMap<(&'static str, String), Vec<DefId>>,
    ids: &dyn Fn(DefId) -> String,
    kind: &'static str,
    id: &str,
    home: &str,
) -> Result<DefId, String> {
    let full = if id.contains(':') { id.to_string() } else { format!("{home}:{id}") };
    if let Some(&d) = index.get(&(kind, full.clone())) {
        return Ok(d);
    }
    let elsewhere: Vec<String> = match id.contains(':') {
        false => bare.get(&(kind, id.to_string())).map(|v| v.iter().map(|&d| ids(d)).collect()).unwrap_or_default(),
        true => Vec::new(),
    };
    Err(match elsewhere.as_slice() {
        [] => format!("unknown {kind} '{full}'"),
        [one] => format!("unknown {kind} '{full}': another mod's def needs its prefix, \"{one}\""),
        many => format!("unknown {kind} '{full}': another mod's def needs its prefix, one of {}", many.join(", ")),
    })
}

impl DefDb {
    /// Items that can be built into something wanting `category`, in def
    /// order. Iterating defs keeps it deterministic; a map would not.
    pub fn materials(&self, category: &str) -> Vec<DefId> {
        self.things
            .iter()
            .enumerate()
            .filter(|(_, t)| t.stuff.as_ref().is_some_and(|s| s.categories.iter().any(|c| c == category)))
            .map(|(i, _)| i as DefId)
            .collect()
    }

    /// A material's multiplier for `name`, or None if it said nothing. The
    /// names mean nothing here: "hp" and "work" happen to be read by the
    /// engine, "sparkle" is read by whoever declared it.
    pub fn factor_declared(&self, made_of: Option<DefId>, name: &str) -> Option<f64> {
        self.thing(made_of?).stuff.as_ref()?.factors.get(name).copied()
    }

    /// `factor_declared`, with 1.0 for a material that says nothing.
    pub fn factor(&self, made_of: Option<DefId>, name: &str) -> f64 {
        self.factor_declared(made_of, name).unwrap_or(1.0)
    }

    /// A stack's hp when it's whole: its def's, scaled by its material.
    pub fn full_hp(&self, def: DefId, made_of: Option<DefId>) -> i32 {
        (self.thing(def).hp as f64 * self.factor(made_of, "hp")).round().max(1.0) as i32
    }

    /// Every item in a category and the categories under it, sorted.
    pub fn category_items(&self, c: DefId) -> Vec<DefId> {
        let mut out = Vec::new();
        let mut open = vec![c];
        while let Some(c) = open.pop() {
            let cd = &self.item_categories[c as usize];
            out.extend_from_slice(&cd.items);
            open.extend_from_slice(&cd.children);
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Does `item` satisfy a buildable asking for `category`?
    pub fn is_material_for(&self, item: DefId, category: &str) -> bool {
        self.thing(item).stuff.as_ref().is_some_and(|s| s.categories.iter().any(|c| c == category))
    }
}

pub const KINDS: &[&str] = &[
    "terrain",
    "thing",
    "creature",
    "need",
    "designation",
    "work_type",
    "priority_scale",
    "stance",
    "priority_rule",
    "work_role",
    "room_role",
    "plan",
    "work_style",
    "skill",
    "field",
    "calendar",
    "sky",
    "start",
    "names",
    "item_category",
    "store_priority",
    "stratum",
    "modifier",
];

impl DefDb {
    /// A def by qualified id ("core:wall"), or by a bare id ("wall") that
    /// exactly one mod defines. For tools and tests: mod content resolves
    /// strictly, with `resolve`.
    pub fn lookup(&self, kind: &'static str, id: &str) -> Option<DefId> {
        if id.contains(':') {
            return self.index.get(&(kind, id.to_string())).copied();
        }
        match self.bare.get(&(kind, id.to_string())).map(Vec::as_slice) {
            Some([one]) => Some(*one),
            _ => None,
        }
    }

    /// A def reference written by mod `from`: a bare id is `from`'s own.
    pub fn resolve(&self, kind: &'static str, id: &str, from: &str) -> Result<DefId, String> {
        resolve_in(&self.index, &self.bare, &|d| self.id_of(kind, d), kind, id, from)
    }

    /// The qualified id of a def.
    pub fn id_of(&self, kind: &str, d: DefId) -> String {
        let i = d as usize;
        match kind {
            "terrain" => self.terrain[i].id.clone(),
            "thing" => self.things[i].id.clone(),
            "creature" => self.creatures[i].id.clone(),
            "need" => self.needs[i].id.clone(),
            "designation" => self.designations[i].id.clone(),
            "work_type" => self.work_types[i].id.clone(),
            "stance" => self.stances[i].id.clone(),
            "work_role" => self.work_roles[i].id.clone(),
            "room_role" => self.room_roles[i].id.clone(),
            "plan" => self.plans[i].id.clone(),
            "priority_rule" => self.priority_rules[i].id.clone(),
            "work_style" => self.work_styles[i].id.clone(),
            "skill" => self.skills[i].id.clone(),
            "field" => self.fields[i].id.clone(),
            "item_category" => self.item_categories[i].id.clone(),
            "stratum" => self.strata[i].id.clone(),
            "modifier" => self.modifiers[i].id.clone(),
            _ => String::new(),
        }
    }
    /// How many levels the map has below the surface: the deepest stratum's.
    pub fn depth(&self) -> i32 {
        self.strata.iter().map(|s| -s.level).max().unwrap_or(0)
    }

    /// The stratum that fills level `z`, if any.
    pub fn stratum(&self, z: i32) -> Option<&StratumDef> {
        self.strata.iter().find(|s| s.level == z)
    }

    pub fn thing_id(&self, id: &str) -> Option<DefId> {
        self.lookup("thing", id)
    }
    pub fn creature_id(&self, id: &str) -> Option<DefId> {
        self.lookup("creature", id)
    }
    pub fn thing(&self, d: DefId) -> &ThingDef {
        &self.things[d as usize]
    }
    /// Tag names as a mask, or `None` if one isn't any tool's: work that
    /// asks for it can't be done.
    pub fn tool_mask(&self, tags: &[String]) -> Option<ToolMask> {
        tags.iter().try_fold(0, |m, t| Some(m | 1 << self.tool_tags.iter().position(|n| n == t)?))
    }
    /// The tag names a mask holds, in tag order.
    pub fn tool_tag_names(&self, mask: ToolMask) -> Vec<&str> {
        self.tool_tags.iter().enumerate().filter(|(i, _)| mask & 1 << i != 0).map(|(_, n)| n.as_str()).collect()
    }
    pub fn creature(&self, d: DefId) -> &CreatureDef {
        &self.creatures[d as usize]
    }
    pub fn need(&self, d: DefId) -> &NeedDef {
        &self.needs[d as usize]
    }

    /// Build the index and resolve every string reference into an id.
    pub fn finalize(&mut self) -> Result<(), String> {
        let mut index = HashMap::new();
        for (i, d) in self.terrain.iter().enumerate() {
            index.insert(("terrain", d.id.clone()), i as DefId);
        }
        for (i, d) in self.things.iter().enumerate() {
            index.insert(("thing", d.id.clone()), i as DefId);
        }
        for (i, d) in self.creatures.iter().enumerate() {
            index.insert(("creature", d.id.clone()), i as DefId);
        }
        for (i, d) in self.needs.iter().enumerate() {
            index.insert(("need", d.id.clone()), i as DefId);
        }
        for (i, d) in self.designations.iter().enumerate() {
            index.insert(("designation", d.id.clone()), i as DefId);
        }
        for (i, d) in self.work_types.iter().enumerate() {
            index.insert(("work_type", d.id.clone()), i as DefId);
        }
        for (i, d) in self.work_styles.iter().enumerate() {
            index.insert(("work_style", d.id.clone()), i as DefId);
        }
        for (i, d) in self.skills.iter().enumerate() {
            index.insert(("skill", d.id.clone()), i as DefId);
        }
        for (i, d) in self.stances.iter().enumerate() {
            index.insert(("stance", d.id.clone()), i as DefId);
        }
        for (i, d) in self.work_roles.iter().enumerate() {
            index.insert(("work_role", d.id.clone()), i as DefId);
        }
        for (i, d) in self.room_roles.iter().enumerate() {
            index.insert(("room_role", d.id.clone()), i as DefId);
        }
        for (i, d) in self.strata.iter().enumerate() {
            index.insert(("stratum", d.id.clone()), i as DefId);
        }
        for (i, d) in self.plans.iter().enumerate() {
            index.insert(("plan", d.id.clone()), i as DefId);
        }
        for (i, d) in self.priority_rules.iter().enumerate() {
            index.insert(("priority_rule", d.id.clone()), i as DefId);
        }
        for (i, d) in self.fields.iter().enumerate() {
            index.insert(("field", d.id.clone()), i as DefId);
        }
        for (i, d) in self.item_categories.iter().enumerate() {
            index.insert(("item_category", d.id.clone()), i as DefId);
        }
        for (i, d) in self.modifiers.iter().enumerate() {
            index.insert(("modifier", d.id.clone()), i as DefId);
        }
        let mut bare: HashMap<(&'static str, String), Vec<DefId>> = HashMap::new();
        for ((kind, id), &d) in &index {
            let short = id.split_once(':').map_or(id.as_str(), |(_, b)| b);
            bare.entry((*kind, short.to_string())).or_default().push(d);
        }
        for v in bare.values_mut() {
            v.sort_unstable();
        }
        self.index = index;
        self.bare = bare;

        // Every def's references resolve in its own mod (the id's prefix).
        let ids: HashMap<(&'static str, DefId), String> =
            self.index.iter().map(|((k, id), &d)| ((*k, d), id.clone())).collect();
        let (idx, bare) = (&self.index, &self.bare);
        let get = |kind: &'static str, id: &str, ctx: &str| -> Result<DefId, String> {
            let home = home_of(ctx.split_once('/').map_or("", |(_, rest)| rest));
            resolve_in(idx, bare, &|d| ids.get(&(kind, d)).cloned().unwrap_or_default(), kind, id, home)
                .map_err(|e| format!("{ctx}: {e}"))
        };
        let counts = |v: &[ItemCount], ctx: &str| -> Result<Vec<(DefId, u32)>, String> {
            v.iter().map(|c| Ok((get("thing", &c.thing, ctx)?, c.count))).collect()
        };

        for d in &mut self.terrain {
            let ctx = format!("terrain/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
            if let Some(s) = &mut d.solid {
                s.thing_r = s.thing.as_deref().map(|t| get("thing", t, &ctx)).transpose()?;
                s.leaves_r = s.leaves.as_deref().map(|t| get("terrain", t, &ctx)).transpose()?;
                // Nothing walks into rock, whatever the def says.
                d.path_cost = 0;
            }
            if d.air {
                if d.solid.is_some() {
                    return Err(format!("{ctx}: a terrain is solid or air, not both"));
                }
                // Nor into air.
                d.path_cost = 0;
            }
        }
        let mut props: Vec<String> = self.terrain.iter().flat_map(|t| t.props.keys().cloned()).collect();
        props.sort_unstable();
        props.dedup();
        let mut tags: Vec<String> = self.terrain.iter().flat_map(|t| t.tags.iter().cloned()).collect();
        tags.sort_unstable();
        tags.dedup();
        for d in &mut self.terrain {
            d.props_q = props.iter().map(|p| d.props.get(p).map_or(0, |v| crate::terms::to_q(*v))).collect();
        }
        self.terrain_props = props;
        self.terrain_tags = tags;
        for (i, d) in self.terrain.iter().enumerate() {
            let Some(s) = &d.solid else { continue };
            let ctx = format!("terrain/{}", d.id);
            match (s.thing_r, s.leaves_r) {
                // Bedrock: nothing works it.
                (None, None) => {}
                (Some(thing), Some(leaves)) => {
                    let td = &self.things[thing as usize];
                    if !td.blocks || td.size != [1, 1] || !td.harvest.iter().any(|h| h.destroy) {
                        return Err(format!(
                            "{ctx}: solid.thing '{}' must block, cover one cell and have a harvest that removes it",
                            td.id
                        ));
                    }
                    if leaves as usize == i || self.terrain[leaves as usize].solid.is_some() {
                        return Err(format!(
                            "{ctx}: solid.leaves '{}' must be ground, not rock",
                            self.terrain[leaves as usize].id
                        ));
                    }
                }
                _ => return Err(format!("{ctx}: solid needs both `thing` and `leaves`, or neither (bedrock)")),
            }
        }
        let mut levels = std::collections::BTreeSet::new();
        for st in &mut self.strata {
            let ctx = format!("stratum/{}", st.id);
            if st.level >= 0 {
                return Err(format!("{ctx}: level must be below the surface (-1 or lower), not {}", st.level));
            }
            if !levels.insert(st.level) {
                return Err(format!("{ctx}: another stratum already fills level {}; patch that one", st.level));
            }
            if st.fill.is_empty() || st.fill.iter().any(|f| f.weight <= 0.0 || !f.weight.is_finite()) {
                return Err(format!("{ctx}: fill needs at least one terrain, each with a weight above 0"));
            }
            if st.patch <= 0.0 || !st.patch.is_finite() {
                return Err(format!("{ctx}: patch must be above 0"));
            }
            st.edge_r = get("terrain", &st.edge, &ctx)?;
            for f in &mut st.fill {
                f.terrain_r = get("terrain", &f.terrain, &ctx)?;
                f.under_r = f.under.iter().map(|t| get("terrain", t, &ctx)).collect::<Result<_, _>>()?;
            }
            if self.terrain[st.edge_r as usize].solid.is_none() {
                return Err(format!("{ctx}: edge '{}' must be solid, or a tunnel could reach the map's edge", st.edge));
            }
        }
        if let Some(&deepest) = levels.first() {
            if levels.len() != (-deepest) as usize {
                return Err(format!("stratum: every level from -1 to {deepest} needs a stratum"));
            }
        }
        let field_in = |home: &str, id: &str| get("field", id, &format!("field/{home}:")).ok().map(|i| i as usize);
        let (terrain_props, terrain_tags) = (&self.terrain_props, &self.terrain_tags);
        let mut warnings = Vec::new();
        for d in &mut self.fields {
            let home = home_of(&d.id).to_string();
            let field_index =
                TermNames { field: &|id: &str| field_in(&home, id), props: terrain_props, tags: terrain_tags };
            d.rgb_low = parse_color(&d.color_low).map_err(|e| format!("field/{}: {e}", d.id))?;
            d.rgb_high = parse_color(&d.color_high).map_err(|e| format!("field/{}: {e}", d.id))?;
            match &d.ambient {
                AmbientDef::Const(v) => d.base = *v,
                AmbientDef::Terms(t) => {
                    d.terms = Terms::compile(t, &format!("field/{}", d.id), &field_index, &mut warnings)?
                }
            }
            // A derived field's value is its outdoor value's terms too, so
            // the outdoor reading, pushes and ordering all come for free.
            match (&d.value, d.kind == FieldKind::Derived) {
                (Some(v), true) if d.terms.is_empty() && d.base == 0.0 && d.indoor == IndoorMode::Outdoor => {
                    d.terms = Terms::compile(v, &format!("field/{}, value", d.id), &field_index, &mut warnings)?;
                }
                (None, false) => {}
                (Some(_), false) => return Err(format!("field/{}: `value` is for kind = \"derived\"", d.id)),
                _ => {
                    return Err(format!(
                        "field/{}: a derived field gives `value` terms, and no `ambient` or `indoor`",
                        d.id
                    ))
                }
            }
            if d.kind == FieldKind::Shelter {
                d.from_r = field_in(&home, &d.from)
                    .ok_or_else(|| format!("field/{}: a shelter field needs `from`, the wind direction field", d.id))?;
                if !d.terms.is_empty() || !(1..=MAX_LEE).contains(&d.lee) {
                    return Err(format!(
                        "field/{}: a shelter field is worked out from the map: no ambient terms, and a lee of 1 to {MAX_LEE} cells",
                        d.id
                    ));
                }
                // Out in the open is fully exposed, so that's what it reads
                // wherever the map doesn't say otherwise.
                d.base = 100.0;
            }
            if let Some(leak) = &d.leak {
                if d.leak_per_hour != 0.0 {
                    return Err(format!(
                        "field/{}: give `leak` or `leak_per_hour`, not both (to change a leak given as terms, patch its terms, such as `leak.sealed`)",
                        d.id
                    ));
                }
                d.leak_terms = Terms::compile(leak, &format!("field/{}, leak", d.id), &field_index, &mut warnings)?;
            }
            let ctx = format!("field/{}", d.id);
            if d.kind == FieldKind::Stock {
                if !d.terms.is_empty() || d.base != 0.0 || d.indoor != IndoorMode::Outdoor || d.leak.is_some() {
                    return Err(format!(
                        "{ctx}: a stock field is its `rate`, `base` and `init` terms, with no `ambient`, `indoor` or `leak`"
                    ));
                }
                let Some(rate) = &d.rate else {
                    return Err(format!("{ctx}: a stock field needs `rate` terms, in its units per game hour"));
                };
                d.rate_terms = Terms::compile(rate, &format!("{ctx}, rate"), &field_index, &mut warnings)?;
                for (key, def, out) in [("base", &d.settle, &mut d.settle_terms), ("init", &d.init, &mut d.init_terms)]
                {
                    let Some(def) = def else { continue };
                    *out = Terms::compile(def, &format!("{ctx}, {key}"), &field_index, &mut warnings)?;
                    if out.reads_own() {
                        return Err(format!(
                            "{ctx}, {key}: `self`, `base` and `above_base` are for the rate, which they feed"
                        ));
                    }
                }
                // Values are kept in 1/10000ths in an i32.
                let fits = |v: f64| v.abs() <= 200_000.0;
                if !(d.period_minutes > 0.0 && d.period_minutes.is_finite())
                    || d.range[0] >= d.range[1]
                    || !fits(d.range[0])
                    || !fits(d.range[1])
                {
                    return Err(format!(
                        "{ctx}: a stock field needs `period_minutes` above 0 and a `range` from low to high, within ±200000"
                    ));
                }
                d.period = (d.period_minutes * crate::TICKS_PER_DAY as f64 / 1440.0).round().max(1.0) as u64;
            } else if d.rate.is_some() || d.settle.is_some() || d.init.is_some() {
                return Err(format!("{ctx}: `rate`, `base` and `init` are for kind = \"stock\""));
            }
            if d.terms.reads_own() || d.leak_terms.reads_own() {
                return Err(format!("{ctx}: `self`, `base` and `above_base` are for a stock field's rate"));
            }
        }
        self.stock_fields = (0..self.fields.len()).filter(|&i| self.fields[i].kind == FieldKind::Stock).collect();
        let mut near: Vec<usize> = self
            .fields
            .iter()
            .flat_map(|f| [&f.terms, &f.rate_terms, &f.settle_terms, &f.init_terms])
            .flat_map(|t| t.nears())
            .collect();
        near.sort_unstable();
        near.dedup();
        self.near_tags = near;
        let reads: Vec<Vec<usize>> = self.fields.iter().map(|f| f.terms.reads()).collect();
        let names: Vec<&str> = self.fields.iter().map(|f| f.id.as_str()).collect();
        self.ambient_order = crate::terms::order(&reads, &names)?;
        let c = &mut self.calendar;
        c.year_days = c.year_days.max(1);
        if c.seasons.is_empty() {
            c.seasons.push("year".into());
        }
        c.start_day %= c.year_days;
        let sky = &mut self.sky;
        let sky_home = home_of(&sky.id).to_string();
        let field_index = |id: &str| field_in(&sky_home, id);
        sky.rgb_night = parse_color(&sky.night).map_err(|e| format!("sky/{}: {e}", sky.id))?;
        sky.rgb_fire = parse_color(&sky.firelight).map_err(|e| format!("sky/{}: {e}", sky.id))?;
        if let Some(sun) = &sky.sun {
            let hours = |h: f64| (0.0..24.0).contains(&h);
            if !(hours(sun.rise) && hours(sun.set) && sun.rise != sun.set && (0.0..=90.0).contains(&sun.peak))
                || !sun.arc.iter().all(|a| a.is_finite())
            {
                return Err(format!(
                    "sky/{}: sun needs rise and set hours from 0 to 24, apart, and a peak of 0 to 90°",
                    sky.id
                ));
            }
        }
        for (label, t) in &mut sky.tint {
            let ctx = format!("sky/{}, tint '{label}'", sky.id);
            t.rgb = parse_color(&t.color).map_err(|e| format!("{ctx}: {e}"))?;
            let one: TermsDef = [(label.clone(), TermDef { scale: t.scale, of: t.of.clone() })].into_iter().collect();
            t.strength = Terms::compile(&one, &ctx, &field_index, &mut warnings)?;
            if t.strength.reads_own() {
                return Err(format!("{ctx}: `self`, `base` and `above_base` are for a stock field's rate"));
            }
        }
        self.warnings.extend(warnings);
        for d in &mut self.needs {
            d.rgb = parse_color(&d.color).map_err(|e| format!("need/{}: {e}", d.id))?;
            if let Some(say) = &d.say {
                if !(0.0..=1.0).contains(&say.below) || say.lines.is_empty() || say.ticks == 0 {
                    return Err(format!(
                        "need/{}: say wants `below` from 0 to 1, at least one line, and `ticks` above 0",
                        d.id
                    ));
                }
            }
            if d.satisfier == Satisfier::Field {
                d.field_r = get("field", &d.field, &format!("need/{}", d.id))?;
            }
        }
        for d in &mut self.designations {
            let ctx = format!("designation/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
            d.work_r = get("work_type", &d.work_type, &ctx)?;
            d.style_r = d.style.as_ref().map(|st| get("work_style", st, &ctx)).transpose()?;
        }
        for (i, st) in self.work_styles.iter().enumerate() {
            if st.every == 0 {
                return Err(format!("work_style/{}: `every` is the work between strikes, at least 1", st.id));
            }
            if st.builds {
                if let Some(b) = self.build_style {
                    return Err(format!(
                        "work_style/{}: only one style can be the one builds use, and work_style/{} already is",
                        st.id, self.work_styles[b as usize].id
                    ));
                }
                self.build_style = Some(i as DefId);
            }
        }
        let sp = &self.store_priority;
        if !(2..=9).contains(&sp.labels.len()) || sp.default as usize >= sp.labels.len() {
            return Err(format!(
                "store_priority/{}: 2 to 9 labels, and a default that is one of them (an index from 0)",
                sp.id
            ));
        }
        let levels = self.priority_scale.levels;
        if !(1..=9).contains(&levels) {
            return Err(format!("priority_scale/{}: levels must be 1 to 9, not {levels}", self.priority_scale.id));
        }
        let named = self.priority_scale.labels.len();
        if named != 0 && named != levels as usize {
            return Err(format!(
                "priority_scale/{}: {named} labels for {levels} levels; give one per level or none",
                self.priority_scale.id
            ));
        }
        // Each engine job is claimed by at most one work type.
        let mut claims: Vec<Option<usize>> = vec![None; ENGINE_JOBS.len()];
        for (i, d) in self.work_types.iter_mut().enumerate() {
            d.priority = d.priority.min(levels);
            for j in &d.jobs {
                let Some(k) = ENGINE_JOBS.iter().position(|e| e == j) else {
                    return Err(format!(
                        "work_type/{}: no engine job '{j}' (there are: {})",
                        d.id,
                        ENGINE_JOBS.join(", ")
                    ));
                };
                if claims[k].is_some() {
                    return Err(format!("work_type/{}: another work type already covers '{j}'", d.id));
                }
                claims[k] = Some(i);
            }
        }
        let claim =
            |job: &str| claims[ENGINE_JOBS.iter().position(|e| *e == job).expect("an engine job")].map(|i| i as DefId);
        self.build_work = claim("build");
        self.haul_work = claim("haul");
        for d in &mut self.work_types {
            if !d.skill.is_empty() {
                d.skill_r = Some(get("skill", &d.skill, &format!("work_type/{}", d.id))?);
            }
        }
        let melee: Vec<usize> = (0..self.skills.len()).filter(|&i| self.skills[i].melee).collect();
        if melee.len() > 1 {
            return Err(format!("skill/{}: only one skill can be the melee skill", self.skills[melee[1]].id));
        }
        self.melee_skill = melee.first().map(|&i| i as DefId);
        let mut order: Vec<DefId> = (0..self.work_types.len() as DefId).collect();
        order.sort_by_key(|&w| (self.work_types[w as usize].order, w));
        self.work_order = order;
        self.default_stance = (0..self.stances.len() as DefId).min_by_key(|&s| (self.stances[s as usize].order, s));
        for r in &mut self.work_roles {
            let ctx = format!("work_role/{}", r.id);
            r.priorities_r = r
                .priorities
                .iter()
                .map(|(t, &l)| Ok((get("work_type", t, &ctx)?, l.min(levels))))
                .collect::<Result<_, String>>()?;
            r.priorities_r.sort_unstable();
        }
        self.default_work_role =
            (0..self.work_roles.len() as DefId).min_by_key(|&r| (self.work_roles[r as usize].order, r));
        // Room roles count tags; each thing learns which of its tags count.
        self.room_tags.clear();
        for r in &mut self.room_roles {
            if r.needs.is_empty() {
                return Err(format!("room_role/{}: `needs` is empty, so every room would take it", r.id));
            }
            r.needs_r = r
                .needs
                .iter()
                .map(|(tag, &n)| {
                    let i = self.room_tags.iter().position(|t| t == tag).unwrap_or_else(|| {
                        self.room_tags.push(tag.clone());
                        self.room_tags.len() - 1
                    });
                    (i as u16, n)
                })
                .collect();
        }
        for d in &mut self.things {
            d.room_tags_r =
                d.tags.iter().filter_map(|t| self.room_tags.iter().position(|r| r == t).map(|i| i as u16)).collect();
        }
        let seasons = &self.calendar.seasons;
        for r in &mut self.priority_rules {
            let ctx = format!("priority_rule/{}", r.id);
            let w = &mut r.when;
            if let Some([a, b]) = w.hours {
                if a > 23 || b > 24 || a == b {
                    return Err(format!("{ctx}: hours are [from, to] within 0 to 24 and not equal, not [{a}, {b}]"));
                }
            }
            w.season_r = w
                .season
                .iter()
                .map(|n| {
                    seasons.iter().position(|s| s == n).map(|i| i as u32).ok_or_else(|| {
                        format!("{ctx}: no season '{n}' in the calendar (there are: {})", seasons.join(", "))
                    })
                })
                .collect::<Result<_, _>>()?;
            w.stance_r = w.stance.as_ref().map(|s| get("stance", s, &ctx)).transpose()?;
            w.need_r = w.need.as_ref().map(|n| get("need", n, &ctx)).transpose()?;
            if w.reading.is_some() {
                if w.need.is_some() {
                    return Err(format!("{ctx}: a rule reads a need or a reading, not both"));
                }
                if w.reading.as_deref().is_some_and(|r| !r.contains(':')) {
                    return Err(format!("{ctx}: name the reading with its mod, like \"core:food_days\""));
                }
                w.band_r = Some(match (w.below, w.above, w.until) {
                    (Some(b), None, u) if u.is_none_or(|u| u >= b) => {
                        Band::Below { on: milli(b), off: milli(u.unwrap_or(b)) }
                    }
                    (None, Some(a), u) if u.is_none_or(|u| u <= a) => {
                        Band::Above { on: milli(a), off: milli(u.unwrap_or(a)) }
                    }
                    _ => {
                        return Err(format!(
                            "{ctx}: a `reading` takes `below` or `above`, and `until` on the far side of it \
                             (below 5 until 8; above 30 until 10)"
                        ))
                    }
                });
            } else {
                if w.until.is_some() {
                    return Err(format!("{ctx}: `until` belongs to a `reading` condition"));
                }
                let fraction = |v: Option<f64>| v.is_none_or(|v| (0.0..=1.0).contains(&v));
                if w.need.is_some() != (w.below.is_some() || w.above.is_some())
                    || !fraction(w.below)
                    || !fraction(w.above)
                {
                    return Err(format!(
                        "{ctx}: a `need` condition takes `below` or `above`, a fraction of full from 0 to 1"
                    ));
                }
            }
            r.set_r = r
                .set
                .iter()
                .map(|(t, &l)| Ok((get("work_type", t, &ctx)?, l.min(levels))))
                .collect::<Result<_, String>>()?;
            // A shift past the whole scale does no more than crossing it.
            let span = levels as i32;
            r.shift_r = r
                .shift
                .iter()
                .map(|(t, &d)| Ok((get("work_type", t, &ctx)?, d.clamp(-span, span))))
                .collect::<Result<_, String>>()?;
            // Work-type order, not the order the TOML map sorted names in.
            r.set_r.sort_unstable();
            r.shift_r.sort_unstable();
        }
        self.rules_read_hour = self.priority_rules.iter().any(|r| r.when.hours.is_some());
        self.rules_read_season = self.priority_rules.iter().any(|r| !r.when.season_r.is_empty());
        // Tool tags become bits: a gate is then a mask test.
        let tags: std::collections::BTreeSet<String> = self
            .things
            .iter()
            .flat_map(|d| {
                let builds = d.build.iter().flat_map(|b| &b.requires).chain(d.stuff.iter().flat_map(|s| &s.requires));
                d.tool.iter().flat_map(|t| &t.tags).chain(d.harvest.iter().flat_map(|h| &h.requires)).chain(builds)
            })
            .cloned()
            .collect();
        if tags.len() > ToolMask::BITS as usize {
            let all: Vec<String> = tags.into_iter().collect();
            return Err(format!("more than {} tool tags: {}", ToolMask::BITS, all.join(", ")));
        }
        self.tool_tags = tags.into_iter().collect();
        let names = self.tool_tags.clone();
        let mask = |tags: &[String]| -> ToolMask {
            tags.iter().filter_map(|t| names.iter().position(|n| n == t)).fold(0, |m, i| m | 1 << i)
        };
        for d in &mut self.things {
            if let Some(t) = &mut d.tool {
                t.tags_r = mask(&t.tags);
            }
            for h in &mut d.harvest {
                h.requires_r = mask(&h.requires);
            }
            if let Some(b) = &mut d.build {
                b.requires_r = mask(&b.requires);
            }
            if let Some(st) = &mut d.stuff {
                st.requires_r = mask(&st.requires);
            }
        }
        // Fields a need is satisfied by, and those they're worked out from.
        let comfort_fields: Vec<usize> = self
            .needs
            .iter()
            .filter(|n| n.satisfier == Satisfier::Field)
            .flat_map(|n| {
                let f = n.field_r as usize;
                std::iter::once(f).chain(self.fields[f].terms.reads())
            })
            .collect();
        // Shelter and derived fields are worked out, not stamped or kept
        // per room: an emitter or a boundary piece on one would do nothing.
        let computed: Vec<Option<&str>> =
            self.fields.iter().map(|f| (f.kind != FieldKind::Ambient).then_some(f.id.as_str())).collect();
        // A stock field takes emitters as a rate; nothing bounds it.
        let stock: Vec<bool> = self.fields.iter().map(|f| f.kind == FieldKind::Stock).collect();
        let (terrain_props, terrain_tags) = (&self.terrain_props, &self.terrain_tags);
        let mut spoil_warnings = Vec::new();
        let mut grow_warnings = Vec::new();
        for d in &mut self.things {
            let ctx = format!("thing/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
            if let Some(sp) = &d.support {
                if !(sp.span > 0.0 && sp.span <= MAX_SPAN as f64) {
                    return Err(format!(
                        "{ctx}: support.span is cells, above 0 and at most {MAX_SPAN}, not {}",
                        sp.span
                    ));
                }
            }
            if let Some(st) = &mut d.stuff {
                let parse = |v: &Option<String>| {
                    v.as_deref().map_or(Ok(crate::look::Pattern::None), crate::look::Pattern::parse)
                };
                st.look.pattern_r = parse(&st.look.pattern).map_err(|e| format!("{ctx}: stuff.look.pattern: {e}"))?;
                st.look.floor_r = parse(&st.look.floor).map_err(|e| format!("{ctx}: stuff.look.floor: {e}"))?;
            }
            let [sw, sh] = d.size;
            if !(1..=MAX_SIZE).contains(&sw) || !(1..=MAX_SIZE).contains(&sh) {
                return Err(format!("{ctx}: size is [w, h], each 1 to {MAX_SIZE}, not [{sw}, {sh}]"));
            }
            if d.size != [1, 1] && matches!(d.category, Category::Item | Category::Floor) {
                return Err(format!("{ctx}: items and floors are one cell; only fixtures have a size"));
            }
            if d.shape.is_some() {
                return Err(format!("{ctx}: `shape` was replaced by `look` in API 0.4; see docs/modding/looks.md"));
            }
            let mut art =
                crate::look::Art { sprites: &mut self.sprites, glyphs: &mut self.glyphs, home: home_of(&d.id) };
            d.look_r = d.look.compile(&mut self.join_groups, &mut art).map_err(|e| format!("{ctx}: {e}"))?;
            for (i, h) in d.harvest.iter_mut().enumerate() {
                h.desig_r = get("designation", &h.designation, &ctx)?;
                h.yields_r = counts(&h.yields, &ctx)?;
                h.index = i;
            }
            for (i, h) in d.harvest.iter().enumerate() {
                if d.harvest[..i].iter().any(|o| o.desig_r == h.desig_r) {
                    return Err(format!(
                        "{ctx}: two harvests use the designation '{}'; each needs its own",
                        h.designation
                    ));
                }
            }
            if let Some(pd) = &d.portal {
                if pd.cost < 200 {
                    return Err(format!("{ctx}: portal.cost must be at least 200 (it's {})", pd.cost));
                }
                if d.size != [1, 1] || d.blocks {
                    return Err(format!("{ctx}: a portal covers one cell and doesn't block"));
                }
            }
            if let Some(b) = &mut d.build {
                // A dig leaves a hole, and nothing; or the thing, as a portal.
                if let Some(g) = &mut b.dig {
                    if g.hole.is_some() == d.portal.is_some() {
                        return Err(format!("{ctx}: build.dig leaves a hole or a portal: one, not both or neither"));
                    }
                    g.hole_r = g.hole.as_deref().map(|h| get("terrain", h, &ctx)).transpose()?;
                }
                // Something to stand on over a pit: a floor, or a door that
                // opens for its owner. A wall over air is no footing.
                if b.spans && (d.blocks || d.size != [1, 1] || !(d.category == Category::Floor || d.door)) {
                    return Err(format!("{ctx}: build.spans is for a one-cell floor or door"));
                }
                b.cost_r = counts(&b.cost, &ctx)?;
                match (b.cost.is_empty(), b.stuff.is_some()) {
                    (true, false) if !b.free => {
                        return Err(format!("{ctx}: build needs `cost`, `stuff`, or `free = true`"))
                    }
                    (false, _) | (_, true) if b.free => {
                        return Err(format!("{ctx}: build is `free` and has a cost; pick one"))
                    }
                    _ => {}
                }
            }
            if let Some(s) = &mut d.spawn {
                s.terrain_r = s.terrain.iter().map(|t| get("terrain", t, &ctx)).collect::<Result<_, _>>()?;
            }
            // Spoiling and keeping are terms read at a cell, like a derived
            // field's.
            let home = home_of(&d.id).to_string();
            let names = TermNames { field: &|id: &str| field_in(&home, id), props: terrain_props, tags: terrain_tags };
            if let Some(sp) = &mut d.spoil {
                sp.rate_terms = Terms::compile(&sp.rate, &format!("{ctx}, spoil.rate"), &names, &mut spoil_warnings)?;
                if !(sp.days > 0.0 && sp.days.is_finite()) || sp.rate_terms.reads_own() {
                    return Err(format!("{ctx}: spoil needs `days` above 0, and rate terms read no `self` or `base`"));
                }
            }
            if let Some(st) = &mut d.store {
                st.keeps_terms =
                    Terms::compile(&st.keeps, &format!("{ctx}, store.keeps"), &names, &mut spoil_warnings)?;
                if st.keeps_terms.reads_own() {
                    return Err(format!("{ctx}: store.keeps reads no `self` or `base`"));
                }
            }
            if let Some(g) = &mut d.grow {
                g.rate_terms = Terms::compile(&g.rate, &format!("{ctx}, grow.rate"), &names, &mut grow_warnings)?;
                g.harm_terms = Terms::compile(&g.harm, &format!("{ctx}, grow.harm"), &names, &mut grow_warnings)?;
                if g.rate_terms.reads_own() || g.harm_terms.reads_own() {
                    return Err(format!("{ctx}: `self`, `base` and `above_base` are for a stock field's rate"));
                }
                if !(g.days > 0.0 && g.days.is_finite()) || g.stages == 0 || !(0.0..=1.0).contains(&g.after_harvest) {
                    return Err(format!(
                        "{ctx}: grow needs `days` above 0, `stages` of 1 or more, and `after_harvest` from 0 to 1"
                    ));
                }
            }
            for b in &mut d.boundary {
                b.field_r = get("field", &b.field, &ctx)?;
            }
            for em in &mut d.emit {
                em.field_r = get("field", &em.field, &ctx)?;
            }
            d.comforts = d.emit.iter().any(|em| em.amount > 0.0 && comfort_fields.contains(&(em.field_r as usize)));
            let fed = d
                .boundary
                .iter()
                .map(|b| b.field_r)
                .chain(d.emit.iter().map(|e| e.field_r).filter(|&f| !stock[f as usize]));
            if let Some(f) = fed.filter_map(|f| computed[f as usize]).next() {
                return Err(format!(
                    "{ctx}: field {f} is worked out from others, so nothing can emit into it or bound it"
                ));
            }
            d.stack_limit = d.stack_limit.max(1);
            if let Some(t) = &d.tool {
                // A tool is one thing in one hand, with its own wear.
                d.stack_limit = 1;
                if t.speed <= 0.0 || !t.speed.is_finite() {
                    return Err(format!("{ctx}: tool speed must be above 0 (it's {})", t.speed));
                }
            }
        }
        let mut by_thing: Vec<Vec<u16>> = vec![Vec::new(); self.things.len()];
        for (i, d) in self.modifiers.iter_mut().enumerate() {
            let ctx = format!("modifier/{}", d.id);
            if d.stat.is_empty() || !d.value.is_finite() {
                return Err(format!("{ctx}: a modifier names a stat and adds a finite value to it"));
            }
            match get("thing", &d.thing, &ctx) {
                Ok(t) => {
                    d.thing_r = Some(t);
                    by_thing[t as usize].push(i as u16);
                }
                // An optional mod's thing, with that mod not installed: nothing
                // to modify, and normal, as patching it would be. A thing its
                // mod doesn't have is a mistake worth saying.
                Err(_) => {
                    let owner = d.thing.split_once(':').map(|(m, _)| format!("{m}:"));
                    let loaded = owner.is_none_or(|m| self.index.keys().any(|(_, id)| id.starts_with(&m)));
                    if loaded {
                        self.warnings.push(format!("{ctx}: no thing '{}' is loaded, so it does nothing", d.thing));
                    }
                }
            }
        }
        self.thing_modifiers = by_thing;
        self.warnings.append(&mut spoil_warnings);
        // What spoils or keeps by how near water is keeps that tag's grid.
        let spoil_nears = self.things.iter().flat_map(|t| {
            let spoil = t.spoil.iter().flat_map(|s| s.rate_terms.nears());
            spoil.chain(t.store.iter().flat_map(|s| s.keeps_terms.nears()))
        });
        self.warnings.append(&mut grow_warnings);
        // A plant reading how near water is keeps that tag's grid too.
        let grown = self.things.iter().filter_map(|t| t.grow.as_ref());
        let grow_nears = grown.flat_map(|g| g.rate_terms.nears().chain(g.harm_terms.nears()));
        let mut near: Vec<usize> = spoil_nears.chain(grow_nears).chain(self.near_tags.iter().copied()).collect();
        near.sort_unstable();
        near.dedup();
        self.near_tags = near;
        for d in &mut self.creatures {
            let ctx = format!("creature/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
            d.butcher_r = counts(&d.butcher, &ctx)?;
            d.needs_r = d.needs.iter().map(|n| get("need", n, &ctx)).collect::<Result<_, _>>()?;
            if let Some(s) = &mut d.spawn {
                s.terrain_r = s.terrain.iter().map(|t| get("terrain", t, &ctx)).collect::<Result<_, _>>()?;
            }
            if d.plural.is_empty() {
                d.plural = format!("{}s", d.label);
            }
            d.speed = d.speed.max(1);
            d.melee_cooldown = d.melee_cooldown.max(1);
        }
        if let Some(s) = &mut self.start {
            s.creature_r = get("creature", &s.creature, &format!("start/{}", s.id))?;
        }
        // House plans: each character resolves to a buildable, and each
        // piece is found in the grid once.
        for plan in &mut self.plans {
            let ctx = format!("plan/{}", plan.id);
            let mut legend: BTreeMap<char, Piece> = BTreeMap::new();
            for (key, pd) in &plan.legend {
                let mut chars = key.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    return Err(format!("{ctx}: legend key '{key}' must be one character"));
                };
                if c == '.' || c == ' ' {
                    return Err(format!("{ctx}: '{c}' builds nothing and can't be in the legend"));
                }
                let thing = get("thing", &pd.thing, &ctx)?;
                let td = &self.things[thing as usize];
                let Some(bd) = &td.build else {
                    return Err(format!("{ctx}: '{c}' is {}, which can't be built", pd.thing));
                };
                let stuff = pd.stuff.as_deref().map(|m| get("thing", m, &ctx)).transpose()?;
                match (&bd.stuff, stuff) {
                    (Some(sc), Some(m)) => {
                        let fits =
                            self.things[m as usize].stuff.as_ref().is_some_and(|s| s.categories.contains(&sc.category));
                        if !fits {
                            return Err(format!(
                                "{ctx}: '{c}': {} isn't a {} material",
                                pd.stuff.as_deref().unwrap_or(""),
                                sc.category
                            ));
                        }
                    }
                    (Some(_), None) => {
                        return Err(format!("{ctx}: '{c}': {} is built of a material; give `stuff`", pd.thing))
                    }
                    (None, Some(_)) => return Err(format!("{ctx}: '{c}': {} takes no material", pd.thing)),
                    (None, None) => {}
                }
                legend.insert(c, Piece { at: (0, 0), thing, stuff, facing: pd.facing & 3 });
            }
            let rows: Vec<Vec<char>> = plan.grid.lines().map(|l| l.chars().collect()).collect();
            let (w, h) = (rows.iter().map(Vec::len).max().unwrap_or(0) as i32, rows.len() as i32);
            if w == 0 {
                return Err(format!("{ctx}: the grid is empty"));
            }
            plan.size = [w, h];
            let mut covered = vec![false; (w * h) as usize];
            plan.pieces.clear();
            for (y, row) in rows.iter().enumerate() {
                for (x, &c) in row.iter().enumerate() {
                    let (x, y) = (x as i32, y as i32);
                    if c == '.' || c == ' ' || covered[(y * w + x) as usize] {
                        continue;
                    }
                    let Some(&piece) = legend.get(&c) else {
                        return Err(format!("{ctx}: '{c}' at row {}, column {} isn't in the legend", y + 1, x + 1));
                    };
                    let [pw, ph] = self.things[piece.thing as usize].size_facing(piece.facing).map(|v| v as i32);
                    if x + pw > w || y + ph > h {
                        return Err(format!("{ctx}: '{c}' at row {}, column {} runs off the grid", y + 1, x + 1));
                    }
                    for cy in y..y + ph {
                        for cx in x..x + pw {
                            covered[(cy * w + cx) as usize] = true;
                        }
                    }
                    plan.pieces.push(Piece { at: (x, y), ..piece });
                }
            }
        }
        // A thing that blocks keeps its outline until it is gone (DESIGN.md
        // §6b), so whatever takes it down may crack it but not shrink it.
        let take_down = self.designations.iter().find(|d| d.targets == Targets::Built).and_then(|d| d.style_r);
        for d in self.things.iter().filter(|d| d.blocks) {
            let felled =
                d.harvest.iter().filter(|h| h.destroy).filter_map(|h| self.designations[h.desig_r as usize].style_r);
            let dismantled = d.build.as_ref().and(take_down);
            for st in felled.chain(dismantled).map(|st| &self.work_styles[st as usize]) {
                if st.wear == Wear::Grow {
                    return Err(format!(
                        "work_style/{}: wear = \"grow\" can't take down thing/{}, which blocks: it would look open \
                         while it still stands. Use \"cracks\" or \"lean\"",
                        st.id, d.id
                    ));
                }
            }
        }
        // The item category tree (DESIGN.md §4f).
        let mut rest: Option<usize> = None;
        for i in 0..self.item_categories.len() {
            let c = &self.item_categories[i];
            let ctx = format!("item_category/{}", c.id);
            let parent = c.parent.as_ref().map(|p| get("item_category", p, &ctx)).transpose()?;
            if let Some(w) = c.with.iter().find(|w| !CATEGORY_WITH.contains(&w.as_str())) {
                return Err(format!("{ctx}: `with` names \"{w}\"; it takes {}", CATEGORY_WITH.join(", ")));
            }
            let mut named = Vec::new();
            for t in &c.things {
                let d = get("thing", t, &ctx)?;
                if self.things[d as usize].category != Category::Item {
                    return Err(format!("{ctx}: {t} isn't an item, so it can't be stored or filtered"));
                }
                named.push(d);
            }
            if c.rest {
                if let Some(r) = rest {
                    return Err(format!(
                        "{ctx}: only one category takes the rest, and item_category/{} already does",
                        self.item_categories[r].id
                    ));
                }
                rest = Some(i);
            }
            let has = |t: &ThingDef, w: &str| match w {
                "food" => t.food.is_some(),
                "tool" => t.tool.is_some(),
                _ => t.stuff.is_some(),
            };
            let items: Vec<DefId> = (0..self.things.len() as DefId)
                .filter(|&d| {
                    let t = &self.things[d as usize];
                    t.category == Category::Item
                        && (named.contains(&d)
                            || t.tags.iter().any(|g| c.tags.contains(g))
                            || c.with.iter().any(|w| has(t, w))
                            || t.stuff.as_ref().is_some_and(|s| s.categories.iter().any(|k| c.stuff.contains(k))))
                })
                .collect();
            let c = &mut self.item_categories[i];
            c.parent_r = parent;
            c.items = items;
        }
        for i in 0..self.item_categories.len() {
            let mut up = self.item_categories[i].parent_r;
            for _ in 0..self.item_categories.len() {
                match up {
                    Some(p) if p as usize == i => {
                        return Err(format!(
                            "item_category/{}: it is its own ancestor; parents must form a tree",
                            self.item_categories[i].id
                        ))
                    }
                    Some(p) => up = self.item_categories[p as usize].parent_r,
                    None => break,
                }
            }
        }
        if let Some(r) = rest {
            let claimed: std::collections::BTreeSet<DefId> =
                self.item_categories.iter().flat_map(|c| c.items.iter().copied()).collect();
            let things = &self.things;
            self.item_categories[r].items.extend(
                (0..things.len() as DefId)
                    .filter(|&d| things[d as usize].category == Category::Item && !claimed.contains(&d)),
            );
            self.item_categories[r].items.sort_unstable();
        }
        let cats = &self.item_categories;
        let by_order = |v: &mut Vec<DefId>| v.sort_by_key(|&c| (cats[c as usize].order, c));
        let mut children: Vec<Vec<DefId>> = vec![Vec::new(); cats.len()];
        let mut roots = Vec::new();
        for (i, c) in cats.iter().enumerate() {
            match c.parent_r {
                Some(p) => children[p as usize].push(i as DefId),
                None => roots.push(i as DefId),
            }
        }
        children.iter_mut().for_each(&by_order);
        by_order(&mut roots);
        for (c, kids) in self.item_categories.iter_mut().zip(children) {
            c.children = kids;
        }
        self.category_roots = roots;
        // What each container can ever take, now the category tree exists.
        for i in 0..self.things.len() {
            let Some(st) = &self.things[i].store else { continue };
            let ctx = format!("thing/{}", self.things[i].id);
            if !(1..=64).contains(&st.slots) || st.stack_scale == 0 || st.look_stages == 0 {
                return Err(format!(
                    "{ctx}: a store has 1 to 64 slots, a stack_scale of at least 1, and look_stages of at least 1"
                ));
            }
            if self.things[i].category == Category::Item {
                return Err(format!("{ctx}: an item can't be a store; a basket that stores is a building"));
            }
            let a = &st.accepts;
            let mut named = Vec::new();
            for t in &a.things {
                named.push(get("thing", t, &ctx)?);
            }
            let mut in_cats = Vec::new();
            for c in &a.categories {
                in_cats.extend(self.category_items(get("item_category", c, &ctx)?));
            }
            let any = a.tags.is_empty() && a.things.is_empty() && a.categories.is_empty();
            let accepts: Vec<DefId> = (0..self.things.len() as DefId)
                .filter(|&d| {
                    let t = &self.things[d as usize];
                    t.category == Category::Item
                        && !t.tags.iter().any(|g| a.not_tags.contains(g))
                        && (any
                            || named.contains(&d)
                            || in_cats.contains(&d)
                            || t.tags.iter().any(|g| a.tags.contains(g)))
                })
                .collect();
            if let Some(st) = &mut self.things[i].store {
                st.accepts_r = accepts;
            }
        }
        for d in &self.things {
            let hole = d.build.as_ref().and_then(|b| b.dig.as_ref()).and_then(|g| g.hole_r);
            if let Some(h) = hole.filter(|&h| !self.terrain[h as usize].air) {
                return Err(format!(
                    "thing/{}: build.dig.hole '{}' must be an air terrain",
                    d.id, self.terrain[h as usize].id
                ));
            }
        }
        if self.terrain.is_empty() {
            return Err("no terrain defined — is the core mod installed?".into());
        }
        if self.start.is_none() {
            return Err("no [[start]] defined — is the core mod installed?".into());
        }
        Ok(())
    }
}

pub fn parse_color(s: &str) -> Result<[u8; 3], String> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("bad color '{s}' (want #rrggbb)"));
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| format!("bad color '{s}'"));
    Ok([p(0)?, p(2)?, p(4)?])
}
