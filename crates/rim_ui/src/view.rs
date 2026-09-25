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
    pub selected: Option<Entity>,
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
    let (mut x, mut y) = (p.pos.x as f32 + 0.5, p.pos.y as f32 + 0.5);
    if let Some(n) = p.next {
        let t = p.progress as f32 / p.step_ticks.max(1) as f32;
        x += (n.x as f32 + 0.5 - x) * t;
        y += (n.y as f32 + 0.5 - y) * t;
    }
    cell_screen(x, y, cv)
}

pub fn cell_screen(wx: f32, wy: f32, cv: &ClientView) -> (f32, f32) {
    let (cx, cy, z) = cv.cam;
    ((wx - cx) * z + cv.screen.0 / 2.0, (wy - cy) * z + cv.screen.1 / 2.0)
}
