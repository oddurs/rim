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
    /// A buildable's cost in the material it would use ("5 stone blocks",
    /// "15 wood", "free"); empty for other tools.
    pub cost: String,
    /// A buildable's work and hit points before its material's factors.
    pub work: u32,
    pub hp: u32,
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
    /// How far into the next sim tick this frame falls (0 to 1): pawns and
    /// what's anchored to them are drawn that far along their step.
    pub frac: f32,
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
    /// The last order given and how long ago, in seconds: what an undo
    /// would take back.
    pub last_order: Option<(String, f64)>,
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
    /// The orders menu's pick at a spot, by option key, for every selected
    /// colonist it's on offer to.
    Order {
        key: String,
        cell: IVec,
        on: Option<Entity>,
    },
    /// Take back the last order given.
    Undo,
    /// A colonist's priority for a work type, by its qualified id.
    SetPriority(Entity, String, u8),
    /// Hand a colonist's work type back to what they'd inherit.
    ClearPriority(Entity, String),
    /// Put a colonist in a work role, by its index in the colony's roles.
    AssignWorkRole(Entity, u16),
    /// A role's level for a work type (qualified id), or `None` to leave it
    /// to the default.
    SetRolePriority(u16, String, Option<u8>),
    /// A new work role named `label`, from a colonist's levels.
    CreateRoleFromPawn(String, Entity),
    /// A new work role named `label`, copying another role.
    CreateRoleFromRole(String, u16),
    /// Put the colony in a stance, by its qualified id.
    SetStance(String),
    /// Switch a priority rule off for the colony, or back on, by its id.
    SetRuleEnabled(String, bool),
    /// Mark a job urgent, or clear the mark.
    MarkUrgent(Entity, bool),
    /// Let a stockpile take an item (by qualified id), or stop it.
    ZoneAllow(u32, String, bool),
    /// Put a stockpile at a level of the store priority scale.
    StoreLevel(u32, u8),
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
    /// Draw the UI this much bigger (0.75 to 2), on top of the display's
    /// own scale.
    UiScale(f32),
    /// Zoom the map by a factor, about the middle of the screen.
    Zoom(f32),
    /// What a scroll does on the map: "auto", "zoom" or "pan".
    ScrollMode(String),
    /// The buildable the build tray's card is about (by tool key), so the
    /// materials view describes it; None goes back to the active tool.
    Preview(Option<String>),
    /// The title screen: play the save at this path.
    Load(String),
    /// The title screen: start a new colony.
    NewColony,
}

/// Screen position of a pawn (physical pixels), interpolated between cells
/// the same way the world renderer draws it.
pub fn pawn_screen(p: &rim_sim::world::Pawn, cv: &ClientView) -> (f32, f32) {
    let (x, y) = p.drawn_at(cv.frac);
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
    frac: f32,
) -> Option<(f32, f32)> {
    match anchor {
        crate::node::Anchor::Entity(bits) => {
            let e = rim_sim::hecs::Entity::from_bits(bits)?;
            let p = world.ecs.get::<&rim_sim::world::Pawn>(e).ok()?;
            let (x, y) = p.drawn_at(frac);
            Some(to_screen(x, y, cam, screen))
        }
        crate::node::Anchor::Cell(x, y) => Some(to_screen(x as f32 + 0.5, y as f32 + 0.5, cam, screen)),
    }
}
