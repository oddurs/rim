//! What the client tells the UI each frame, and what the UI asks back.
//!
//! The UI reads the world only through `ClientView` plus a shared reference
//! to the `World`; it acts only by returning `UiAction`s. Neither path can
//! change the simulation, so no UI mod can desync a game.

use rim_sim::hecs::Entity;
use rim_sim::IVec;

#[derive(Clone, Debug, Default)]
pub struct ToolView {
    pub key: String,
    pub label: String,
    pub color: [u8; 3],
    pub active: bool,
    /// Where the dock files it: "orders", "build", "zones", or "" for a
    /// tool that is always on the dock (select).
    pub category: String,
    /// Its row inside the category: a buildable's `build.menu`, or "".
    pub group: String,
}

/// One material the active build tool could use.
#[derive(Clone, Debug, Default)]
pub struct StuffView {
    /// The material's thing id; `act.stuff(id)` picks it.
    pub id: String,
    pub label: String,
    pub color: [u8; 3],
    /// How much of it the colony has lying around.
    pub have: u32,
    pub active: bool,
    /// What the buildable would come out as, made of this.
    pub hp: u32,
    pub work: u32,
}

impl ClientView {
    /// Whether `e` is selected, alone or with others.
    pub fn is_selected(&self, e: Entity) -> bool {
        self.selected == Some(e) || self.group.contains(&e)
    }
}

/// Client state the UI can read, rebuilt by the client every frame.
#[derive(Clone, Debug, Default)]
pub struct ClientView {
    /// Screen size in physical pixels.
    pub screen: (f32, f32),
    /// Physical pixels per logical pixel (DPI times UI scale).
    pub scale: f32,
    /// Camera: centre in tiles, and physical pixels per tile.
    pub cam: (f32, f32, f32),
    /// Mouse in physical pixels.
    pub mouse: (f32, f32),
    /// What the inspector shows: the one selected thing, or the first of
    /// several selected colonists.
    pub selected: Option<Entity>,
    /// Every selected colonist when more than one is; empty otherwise.
    pub group: Vec<Entity>,
    /// Shift is held, so a click adds to the selection.
    pub shift: bool,
    pub paused: bool,
    pub speed: u32,
    pub overlay: Option<usize>,
    pub show_profiler: bool,
    pub show_devtools: bool,
    pub tools: Vec<ToolView>,
    /// Materials for the active build tool, or empty when it takes none.
    pub stuff: Vec<StuffView>,
    /// What a right-click would do here, if anything.
    pub hint: Option<String>,
    /// The world cell under the mouse, when it isn't over the UI.
    pub hover_cell: Option<IVec>,
    pub hover_pawn: Option<Entity>,
    /// Wall-clock seconds, for animation.
    pub time: f64,
    /// Profiler rows (name, smoothed µs), refreshed a few times a second.
    pub profile: Vec<(String, f64)>,
    pub stats: Vec<String>,
    /// (id, version, name) in load order.
    pub mods: Vec<(String, String, String)>,
    pub warnings: Vec<String>,
    /// No colony yet: only the `title` layer is built, over an empty world.
    pub title: bool,
    /// The player's saves, newest first, for the title screen.
    pub saves: Vec<SaveView>,
}

/// A save, as the title screen lists it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveView {
    pub path: String,
    /// The file's name, without its folder.
    pub file: String,
    /// The day the colony reached, counted from 1.
    pub day: u64,
    /// The living colonists, the founder first.
    pub colonists: Vec<String>,
    /// Seconds since it was last played.
    pub age: f64,
    /// Why it can't be read, or why loading it failed.
    pub error: Option<String>,
}

/// Everything a UI script can ask the client to do.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    Select(Option<Entity>),
    /// Add a colonist to the selection, or take them out of it.
    ToggleSelect(Entity),
    /// Centre the camera on a pawn.
    Focus(Entity),
    /// Pick a toolbar tool by key.
    Tool(String),
    /// Pick the material for the active build tool, by thing id.
    Stuff(String),
    Speed(u32),
    TogglePause,
    Draft(Entity, bool),
    /// A colonist's priority for a work type, by its qualified id.
    SetPriority(Entity, String, u8),
    /// Put the colony in a stance, by its qualified id.
    SetStance(String),
    /// Let a stockpile take an item (by qualified id), or stop it.
    ZoneAllow(u32, String, bool),
    CycleOverlay,
    SetOverlay(Option<usize>),
    ToggleProfiler,
    ToggleDevtools,
    /// Devtools: outline every layout box (handled by the engine).
    ToggleOutlines,
    /// A mod's UI sends an event to its own sim scripts ("weather:force"),
    /// through a Command so it replays and stays in lockstep.
    Send(String, Option<rim_sim::data::Data>),
    /// Devtools: run the simulation forward this many game hours now.
    Advance(f64),
    /// Draw the world at this fraction of the screen's pixels (0.25 to 1);
    /// the UI stays at full resolution.
    RenderScale(f32),
    /// The title screen: play the save at this path.
    Load(String),
    /// The title screen: start a new colony.
    NewColony,
}

/// Screen position of a pawn (physical pixels), interpolated between cells
/// the same way the world renderer draws it.
pub fn pawn_screen(p: &rim_sim::world::Pawn, cv: &ClientView) -> (f32, f32) {
    let (x, y) = pawn_cell(p);
    cell_screen(x, y, cv)
}

pub fn cell_screen(wx: f32, wy: f32, cv: &ClientView) -> (f32, f32) {
    to_screen(wx, wy, cv.cam, cv.screen)
}

fn to_screen(wx: f32, wy: f32, (cx, cy, z): (f32, f32, f32), (sw, sh): (f32, f32)) -> (f32, f32) {
    ((wx - cx) * z + sw / 2.0, (wy - cy) * z + sh / 2.0)
}

/// Where an anchored node's anchor is on screen (physical pixels) for the
/// camera `cam` (x, y, pixels per cell), or None if its pawn is gone.
pub fn anchor_screen(
    anchor: crate::node::Anchor,
    world: &rim_sim::world::World,
    cam: (f32, f32, f32),
    screen: (f32, f32),
) -> Option<(f32, f32)> {
    match anchor {
        crate::node::Anchor::Entity(bits) => {
            let e = rim_sim::hecs::Entity::from_bits(bits)?;
            let p = world.ecs.get::<&rim_sim::world::Pawn>(e).ok()?;
            let (x, y) = pawn_cell(&p);
            Some(to_screen(x, y, cam, screen))
        }
        crate::node::Anchor::Cell(x, y) => Some(to_screen(x as f32 + 0.5, y as f32 + 0.5, cam, screen)),
    }
}

/// A pawn's centre in cells, interpolated between cells the way the world
/// renderer draws it.
fn pawn_cell(p: &rim_sim::world::Pawn) -> (f32, f32) {
    let (x, y) = (p.pos.x as f32 + 0.5, p.pos.y as f32 + 0.5);
    match p.next {
        Some(n) => {
            let t = p.progress as f32 / p.step_ticks.max(1) as f32;
            (x + (n.x as f32 + 0.5 - x) * t, y + (n.y as f32 + 0.5 - y) * t)
        }
        None => (x, y),
    }
}
