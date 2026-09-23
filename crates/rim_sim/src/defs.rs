//! Typed definitions. Mods write these as TOML; the engine only ever sees
//! the merged, patched, validated result.
//!
//! Unknown fields are ignored on purpose: a plugin may annotate another
//! mod's defs with data that only it understands.

use serde::Deserialize;
use std::collections::HashMap;

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
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Blob,
    Tree,
    Bush,
    Rock,
    Wall,
    Door,
    Bed,
    Fire,
    Item,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ThingDef {
    pub id: String,
    pub label: String,
    pub color: String,
    pub category: Category,
    #[serde(default)]
    pub shape: Shape,
    #[serde(default)]
    pub market_value: f64,
    /// Blocks movement (walls, rocks).
    #[serde(default)]
    pub blocks: bool,
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
    pub harvest: Option<HarvestDef>,
    pub build: Option<BuildDef>,
    pub food: Option<FoodDef>,
    pub bed: Option<BedDef>,
    pub spawn: Option<SpawnDef>,
    /// Field sources: a campfire emits heat and light.
    #[serde(default)]
    pub emit: Vec<EmitDef>,
    #[serde(skip)]
    pub rgb: [u8; 3],
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
    #[serde(skip)]
    pub desig_r: DefId,
    #[serde(skip)]
    pub yields_r: Vec<(DefId, u32)>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BuildDef {
    pub cost: Vec<ItemCount>,
    pub work: u32,
    /// Toolbar group.
    #[serde(default)]
    pub menu: String,
    #[serde(skip)]
    pub cost_r: Vec<(DefId, u32)>,
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
    #[serde(skip)]
    pub rgb: [u8; 3],
    #[serde(skip)]
    pub field_r: DefId,
}

// ---------------------------------------------------------------- fields

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

#[derive(Deserialize, Clone, Debug)]
pub struct FieldDef {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub unit: String,
    /// Outdoor value until a script sets it.
    #[serde(default)]
    pub ambient: f64,
    #[serde(default)]
    pub indoor: IndoorMode,
    /// Room fields: fraction of the gap to outdoors closed per hour.
    #[serde(default)]
    pub leak_per_hour: f64,
    /// Room fields: how strongly emitters inside push the room's value.
    #[serde(default)]
    pub room_gain: f64,
    /// Overlay colour ramp across `range`.
    pub range: [f64; 2],
    pub color_low: String,
    pub color_high: String,
    /// Show the outdoor value in the top bar.
    #[serde(default)]
    pub hud: bool,
    #[serde(skip)]
    pub rgb_low: [u8; 3],
    #[serde(skip)]
    pub rgb_high: [u8; 3],
}

// ---------------------------------------------------------------- designations

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Targets {
    #[default]
    Thing,
    Creature,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DesignationDef {
    pub id: String,
    pub label: String,
    pub color: String,
    #[serde(default)]
    pub targets: Targets,
    #[serde(skip)]
    pub rgb: [u8; 3],
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
    pub fields: Vec<FieldDef>,
    pub start: Option<StartDef>,
    pub names: Vec<String>,
    index: HashMap<(&'static str, String), DefId>,
}

pub const KINDS: &[&str] = &["terrain", "thing", "creature", "need", "designation", "field", "start", "names"];

impl DefDb {
    pub fn lookup(&self, kind: &'static str, id: &str) -> Option<DefId> {
        self.index.get(&(kind, id.to_string())).copied()
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
        for (i, d) in self.fields.iter().enumerate() {
            index.insert(("field", d.id.clone()), i as DefId);
        }
        self.index = index;

        let idx = &self.index;
        let get = |kind: &'static str, id: &str, ctx: &str| -> Result<DefId, String> {
            idx.get(&(kind, id.to_string())).copied().ok_or_else(|| format!("{ctx}: unknown {kind} '{id}'"))
        };
        let counts = |v: &[ItemCount], ctx: &str| -> Result<Vec<(DefId, u32)>, String> {
            v.iter().map(|c| Ok((get("thing", &c.thing, ctx)?, c.count))).collect()
        };

        for d in &mut self.terrain {
            d.rgb = parse_color(&d.color).map_err(|e| format!("terrain/{}: {e}", d.id))?;
        }
        for d in &mut self.fields {
            d.rgb_low = parse_color(&d.color_low).map_err(|e| format!("field/{}: {e}", d.id))?;
            d.rgb_high = parse_color(&d.color_high).map_err(|e| format!("field/{}: {e}", d.id))?;
        }
        for d in &mut self.needs {
            d.rgb = parse_color(&d.color).map_err(|e| format!("need/{}: {e}", d.id))?;
            if d.satisfier == Satisfier::Field {
                d.field_r = get("field", &d.field, &format!("need/{}", d.id))?;
            }
        }
        for d in &mut self.designations {
            d.rgb = parse_color(&d.color).map_err(|e| format!("designation/{}: {e}", d.id))?;
        }
        for d in &mut self.things {
            let ctx = format!("thing/{}", d.id);
            d.rgb = parse_color(&d.color).map_err(|e| format!("{ctx}: {e}"))?;
            if let Some(h) = &mut d.harvest {
                h.desig_r = get("designation", &h.designation, &ctx)?;
                h.yields_r = counts(&h.yields, &ctx)?;
            }
            if let Some(b) = &mut d.build {
                b.cost_r = counts(&b.cost, &ctx)?;
            }
            if let Some(s) = &mut d.spawn {
                s.terrain_r = s.terrain.iter().map(|t| get("terrain", t, &ctx)).collect::<Result<_, _>>()?;
            }
            for em in &mut d.emit {
                em.field_r = get("field", &em.field, &ctx)?;
            }
            d.stack_limit = d.stack_limit.max(1);
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
            s.creature_r = get("creature", &s.creature, "start")?;
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
    if h.len() != 6 {
        return Err(format!("bad color '{s}' (want #rrggbb)"));
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| format!("bad color '{s}'"));
    Ok([p(0)?, p(2)?, p(4)?])
}
