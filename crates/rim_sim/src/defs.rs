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
    #[serde(skip)]
    pub rgb: [u8; 3],
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
    /// Extra movement cost in percent (doors, trees).
    #[serde(default)]
    pub path_cost: u32,
    /// Passable, but bounds rooms like a wall does.
    #[serde(default)]
    pub door: bool,
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
    pub bed: Option<BedDef>,
    /// Present on items that things can be built out of.
    #[serde(default)]
    pub stuff: Option<StuffDef>,
    pub spawn: Option<SpawnDef>,
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
    /// What this piece does to a room it helps enclose, per field. A wall
    /// leaks a little, a window leaks a lot and lets daylight through.
    #[serde(default)]
    pub boundary: Vec<BoundaryDef>,
    /// It warms (or otherwise comforts) what a need reads: it emits a
    /// positive amount into a field a need is satisfied by, or that one is
    /// worked out from. Until the colony has one, building one is urgent.
    #[serde(skip)]
    pub comforts: bool,
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub look_r: Look,
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
    /// Every cell it covers with its anchor at `at`, row by row.
    pub fn footprint(&self, at: crate::IVec) -> impl Iterator<Item = crate::IVec> {
        let [w, h] = self.size;
        (0..h as i32).flat_map(move |y| (0..w as i32).map(move |x| at.offset(x, y)))
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
    /// A fixed recipe. Empty when the thing is built out of `stuff`.
    #[serde(default)]
    pub cost: Vec<ItemCount>,
    /// Built out of whatever matches: the def says how much, the player
    /// says of what. A wall is 25 of something structural, not 25 wood.
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
    #[serde(skip)]
    pub cost_r: Vec<(DefId, u32)>,
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
    0.45
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
    /// Share of daylight that reaches inside enclosed rooms, as if through
    /// windows (the `light` field itself is 0 indoors).
    #[serde(default = "dshare")]
    pub indoor_share: f64,
    #[serde(skip)]
    pub rgb_night: [u8; 3],
    #[serde(skip)]
    pub rgb_fire: [u8; 3],
}

impl Default for SkyDef {
    fn default() -> Self {
        SkyDef {
            id: "default".into(),
            tint: BTreeMap::new(),
            night: dnight(),
            firelight: dfire(),
            indoor_share: dshare(),
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

/// How many priority levels there are. Core says 4; a mod patches it to 9.
#[derive(Deserialize, Clone, Debug)]
pub struct PriorityScaleDef {
    pub id: String,
    #[serde(default = "d4")]
    pub levels: u8,
}

impl Default for PriorityScaleDef {
    fn default() -> Self {
        PriorityScaleDef { id: "default".into(), levels: 4 }
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
    #[serde(default)]
    pub below: Option<f64>,
    #[serde(default)]
    pub above: Option<f64>,
    #[serde(skip)]
    pub season_r: Vec<u32>,
    #[serde(skip)]
    pub stance_r: Option<DefId>,
    #[serde(skip)]
    pub need_r: Option<DefId>,
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

// ---------------------------------------------------------------- database

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
    pub stances: Vec<StanceDef>,
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
    pub calendar: CalendarDef,
    pub sky: SkyDef,
    pub start: Option<StartDef>,
    pub names: Vec<String>,
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
    "work_style",
    "skill",
    "field",
    "calendar",
    "sky",
    "start",
    "names",
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
            "work_style" => self.work_styles[i].id.clone(),
            "skill" => self.skills[i].id.clone(),
            "field" => self.fields[i].id.clone(),
            _ => String::new(),
        }
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
        for (i, d) in self.fields.iter().enumerate() {
            index.insert(("field", d.id.clone()), i as DefId);
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
            d.rgb = parse_color(&d.color).map_err(|e| format!("terrain/{}: {e}", d.id))?;
        }
        let field_in = |home: &str, id: &str| get("field", id, &format!("field/{home}:")).ok().map(|i| i as usize);
        for d in &mut self.fields {
            let home = home_of(&d.id).to_string();
            let field_index = |id: &str| field_in(&home, id);
            d.rgb_low = parse_color(&d.color_low).map_err(|e| format!("field/{}: {e}", d.id))?;
            d.rgb_high = parse_color(&d.color_high).map_err(|e| format!("field/{}: {e}", d.id))?;
            match &d.ambient {
                AmbientDef::Const(v) => d.base = *v,
                AmbientDef::Terms(t) => d.terms = Terms::compile(t, &format!("field/{}", d.id), &field_index)?,
            }
            // A derived field's value is its outdoor value's terms too, so
            // the outdoor reading, pushes and ordering all come for free.
            match (&d.value, d.kind == FieldKind::Derived) {
                (Some(v), true) if d.terms.is_empty() && d.base == 0.0 && d.indoor == IndoorMode::Outdoor => {
                    d.terms = Terms::compile(v, &format!("field/{}, value", d.id), &field_index)?;
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
                d.from_r = field_index(&d.from)
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
                d.leak_terms = Terms::compile(leak, &format!("field/{}, leak", d.id), &field_index)?;
            }
        }
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
        for (label, t) in &mut sky.tint {
            let ctx = format!("sky/{}, tint '{label}'", sky.id);
            t.rgb = parse_color(&t.color).map_err(|e| format!("{ctx}: {e}"))?;
            let one: TermsDef = [(label.clone(), TermDef { scale: t.scale, of: t.of.clone() })].into_iter().collect();
            t.strength = Terms::compile(&one, &ctx, &field_index)?;
        }
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
        let levels = self.priority_scale.levels;
        if !(1..=9).contains(&levels) {
            return Err(format!("priority_scale/{}: levels must be 1 to 9, not {levels}", self.priority_scale.id));
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
            let fraction = |v: Option<f64>| v.is_none_or(|v| (0.0..=1.0).contains(&v));
            if w.need.is_some() != (w.below.is_some() || w.above.is_some()) || !fraction(w.below) || !fraction(w.above)
            {
                return Err(format!(
                    "{ctx}: a `need` condition takes `below` or `above`, a fraction of full from 0 to 1"
                ));
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
            .flat_map(|d| d.tool.iter().flat_map(|t| &t.tags).chain(d.harvest.iter().flat_map(|h| &h.requires)))
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
        for d in &mut self.things {
            let ctx = format!("thing/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
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
            if let Some(b) = &mut d.build {
                b.cost_r = counts(&b.cost, &ctx)?;
                match (b.cost.is_empty(), b.stuff.is_some()) {
                    (true, false) if !b.free => {
                        return Err(format!("{ctx}: build needs `cost`, `stuff`, or `free = true`"))
                    }
                    (false, _) | (_, true) if b.free => {
                        return Err(format!("{ctx}: build is `free` and has a cost; pick one"))
                    }
                    (false, true) => return Err(format!("{ctx}: build has both `cost` and `stuff`; pick one")),
                    _ => {}
                }
            }
            if let Some(s) = &mut d.spawn {
                s.terrain_r = s.terrain.iter().map(|t| get("terrain", t, &ctx)).collect::<Result<_, _>>()?;
            }
            for b in &mut d.boundary {
                b.field_r = get("field", &b.field, &ctx)?;
            }
            for em in &mut d.emit {
                em.field_r = get("field", &em.field, &ctx)?;
            }
            d.comforts = d.emit.iter().any(|em| em.amount > 0.0 && comfort_fields.contains(&(em.field_r as usize)));
            let fed = d.boundary.iter().map(|b| b.field_r).chain(d.emit.iter().map(|e| e.field_r));
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
