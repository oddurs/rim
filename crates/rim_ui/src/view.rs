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
}

/// Everything a UI script can ask the client to do.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    Select(Option<Entity>),
    /// Centre the camera on a pawn.
    Focus(Entity),
    /// Pick a toolbar tool by key.
    Tool(String),
    Speed(u32),
    TogglePause,
    Draft(Entity, bool),
    CycleOverlay,
    SetOverlay(Option<usize>),
    ToggleProfiler,
    ToggleDevtools,
    /// Devtools: outline every layout box (handled by the engine).
    ToggleOutlines,
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
