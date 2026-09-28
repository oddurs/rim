//! `rim --autotest [dir]` drives the real client through every control.
//!
//! It feeds synthetic raw input through the same `frame()` the game loop
//! uses: world clicks and drags pass through the UI's routing exactly as the
//! mouse's would, and UI controls are clicked by node id (`core:toolbar.
//! designate:core:chop`), not screen position. It checks the game state after
//! each step, saves screenshots to `dir` (default `target/autotest`), and
//! exits non-zero if any check failed.

mod building;
mod chalk;
mod indoors;
mod input;
mod light;
mod replace;
mod tools;
mod ui;
mod weather;

use crate::overlay::Mark;
use crate::{apply, draw, frame, render, Action, App, RawInput, Tool};
use macroquad::prelude::*;
use rim_sim::data::Data;
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::path::Goal;
use rim_sim::world::*;
use rim_sim::Command;
use rim_sim::IVec;
use std::path::PathBuf;

struct T {
    app: App,
    dir: PathBuf,
    shots: usize,
    passed: usize,
    failed: Vec<String>,
    mouse: (f32, f32),
    clock: f64,
}

impl T {
    /// One frame of input through the real path, then draw it.
    async fn input(&mut self, mut raw: RawInput) {
        self.clock += 1.0 / 60.0;
        raw.time = self.clock;
        self.mouse = raw.mouse;
        frame(&mut self.app, &raw);
        render(&mut self.app);
        next_frame().await;
    }

    /// Frames until the firelight bake has caught up: it bakes at most once
    /// a second, and draws what it hasn't baked yet as moving lights.
    async fn light_settles(&mut self) {
        let since = get_time();
        loop {
            self.frame().await;
            // Changing level fades for a moment too; what's on screen is
            // only one level's once it has.
            if !self.app.light.owes_a_bake() && self.app.fade.is_none() {
                return;
            }
            // A second's bake and the fade, with room for a slow machine.
            if get_time() - since > 10.0 {
                self.check(false, "the firelight bake caught up within 10 s");
                return;
            }
        }
    }

    async fn frame(&mut self) {
        let raw = RawInput { mouse: self.mouse, ..Default::default() };
        self.input(raw).await;
    }

    /// Render and read the frame back. The read has to come before
    /// `next_frame` swaps the buffer away, or it reads black.
    async fn grab(&mut self) -> Image {
        // Two frames: one to lay out, one to draw what was laid out.
        self.frame().await;
        let raw = RawInput { mouse: self.mouse, ..Default::default() };
        self.clock += 1.0 / 60.0;
        frame(&mut self.app, &RawInput { time: self.clock, ..raw });
        render(&mut self.app);
        let img = get_screen_data();
        next_frame().await;
        img
    }

    async fn shot(&mut self, name: &str) {
        let img = self.grab().await;
        self.shots += 1;
        // Def ids have a colon ("core:light"), which Windows and CI artifacts refuse.
        let path = self.dir.join(format!("{:02}_{}.png", self.shots, name.replace(':', "_")));
        img.export_png(path.to_str().unwrap());
        println!("shot  {}", path.display());
    }

    fn check(&mut self, ok: bool, what: impl Into<String>) {
        let what = what.into();
        if ok {
            self.passed += 1;
            println!("ok    {what}");
        } else {
            println!("FAIL  {what}");
            self.failed.push(what);
        }
    }

    fn act(&mut self, a: Action) {
        apply(&mut self.app, a);
    }

    fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            self.app.sim.step();
            // What the game's frame does after its ticks.
            self.app.motion.stepped(&self.app.sim.world);
            self.app.motion.face(&self.app.sim.world, &self.app.worksites, 0.0);
        }
    }

    fn w(&self) -> &World {
        &self.app.sim.world
    }

    /// Fed, rested, warm and whole: a section that lets weather pass keeps
    /// the colony alive, because later sections select, draft and follow
    /// the founder. Dying in a storm is fair in play, not in the harness.
    fn keep_well(&mut self) {
        let w = &mut self.app.sim.world;
        let colony: Vec<Entity> = w.colonists().collect();
        for e in colony {
            let Ok(mut p) = w.ecs.get::<&mut Pawn>(e) else { continue };
            p.needs.iter_mut().for_each(|n| n.1 = NEED_MAX);
            p.hp = w.defs.creature(p.def).max_hp;
        }
    }

    fn pawn(&self, e: Entity) -> Pawn {
        (*self.w().ecs.get::<&Pawn>(e).expect("pawn exists")).clone()
    }

    fn screen(&self, p: IVec) -> (f32, f32) {
        self.app.cam.to_screen(p.x as f32 + 0.5, p.y as f32 + 0.5)
    }

    fn pawn_screen(&self, e: Entity) -> (f32, f32) {
        let (x, y) = draw::pawn_pos(&self.app, e, &self.pawn(e));
        self.app.cam.to_screen(x, y)
    }

    fn focus(&mut self, p: IVec) {
        self.app.cam.x = p.x as f32 + 0.5;
        self.app.cam.y = p.y as f32 + 0.5;
    }

    /// Left click at a screen point (logical), through the UI's routing.
    async fn click(&mut self, at: (f32, f32)) {
        self.input(RawInput { mouse: at, ..Default::default() }).await;
        self.input(RawInput { mouse: at, left_pressed: true, ..Default::default() }).await;
        self.input(RawInput { mouse: at, left_released: true, ..Default::default() }).await;
    }

    async fn right_click(&mut self, at: (f32, f32)) {
        self.input(RawInput { mouse: at, ..Default::default() }).await;
        self.input(RawInput { mouse: at, right_pressed: true, right_down: true, ..Default::default() }).await;
        self.input(RawInput { mouse: at, right_released: true, ..Default::default() }).await;
    }

    /// Press the right button and hold it still, past the menu's delay.
    async fn right_hold(&mut self, at: (f32, f32)) {
        self.input(RawInput { mouse: at, ..Default::default() }).await;
        self.input(RawInput { mouse: at, right_pressed: true, right_down: true, ..Default::default() }).await;
        for _ in 0..30 {
            self.input(RawInput { mouse: at, right_down: true, ..Default::default() }).await;
        }
    }

    /// Press a key, then let the UI catch up: actions apply after the UI's
    /// frame, and trees rebuild at most every 50 ms without input.
    async fn key(&mut self, k: KeyCode) {
        // Named as the real input path names it, so core's bindings fire.
        let pressed = crate::key_name(k).map(|n| vec![n.to_string()]).unwrap_or_default();
        let raw = RawInput { mouse: self.mouse, keys: vec![k], pressed, ..Default::default() };
        self.input(raw).await;
        self.settle().await;
    }

    /// Enough frames for a tree rebuild to pick up any change.
    async fn settle(&mut self) {
        for _ in 0..4 {
            self.frame().await;
        }
    }

    async fn drag(&mut self, a: IVec, b: IVec) {
        let (ax, ay) = self.screen(a);
        let (bx, by) = self.screen(b);
        self.input(RawInput { mouse: (ax, ay), left_pressed: true, ..Default::default() }).await;
        self.input(RawInput { mouse: (bx, by), ..Default::default() }).await;
        self.input(RawInput { mouse: (bx, by), left_released: true, ..Default::default() }).await;
    }

    /// Where a UI node is, in logical points (the UI works in physical pixels).
    fn ui_rect(&self, id: &str) -> Option<[f32; 4]> {
        let dpi = screen_dpi_scale();
        self.app.ui.find(id).map(|r| [r[0] / dpi, r[1] / dpi, r[2] / dpi, r[3] / dpi])
    }

    /// Click a UI control by its id.
    async fn click_ui(&mut self, id: &str) -> bool {
        self.frame().await;
        let Some(r) = self.ui_rect(id) else {
            println!("      (no UI node '{id}')");
            return false;
        };
        self.click((r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)).await;
        true
    }

    /// Pick a tool from the dock the way a player does: open its category
    /// and its group if its button isn't showing, then click it.
    async fn click_tool(&mut self, key: &str) -> bool {
        self.frame().await;
        let id = format!("core:toolbar.{key}");
        let Some((category, group)) =
            self.app.tools.iter().find(|t| t.key == key).map(|t| (t.category, t.group.clone()))
        else {
            println!("      (no tool '{key}')");
            return false;
        };
        if self.ui_rect(&id).is_none() && self.ui_rect(&format!("core:dock.palette.{category}")).is_none() {
            self.click_ui(&format!("core:dock.{category}")).await;
            self.frame().await;
        }
        if self.ui_rect(&id).is_none() && !group.is_empty() {
            self.click_ui(&format!("core:dock.groups.{group}")).await;
            self.frame().await;
        }
        // The tray is a fixed height and a longer list scrolls (toolbar.luau):
        // wheel the row into view, as a player would, and click it where it
        // is drawn. Layout rects are before scrolling; the offset is apart.
        let shown = |t: &T| {
            let r = t.ui_rect(&id)?;
            let off = t.app.ui.scroll_offset("core:dock.list").unwrap_or(0.0) / screen_dpi_scale();
            Some([r[0], r[1] - off, r[2], r[3]])
        };
        for _ in 0..20 {
            let (Some(r), Some(list)) = (shown(self), self.ui_rect("core:dock.list")) else { break };
            let (below, above) = (r[1] + r[3] > list[1] + list[3], r[1] < list[1]);
            if !below && !above {
                break;
            }
            let over = (list[0] + list[2] / 2.0, list[1] + list[3] / 2.0);
            let wheel = if below { -1.0 } else { 1.0 };
            self.input(RawInput { mouse: over, wheel, ..Default::default() }).await;
            self.frame().await;
        }
        if let Some(r) = shown(self).filter(|_| self.ui_rect("core:dock.list").is_some()) {
            self.frame().await;
            self.click((r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)).await;
            self.frame().await;
            return true;
        }
        let clicked = self.click_ui(&id).await;
        // Let the pick land: the tray folds on the next frame, and the tray
        // floats over the map, so a press in the same frame would hit it.
        self.frame().await;
        clicked
    }

    /// Back out of the dock: stop placing, close its tray and any sheet,
    /// leaving the selection alone.
    async fn clear_dock(&mut self) {
        for _ in 0..4 {
            self.frame().await;
            let busy = self.app.tool != Tool::Select
                || self.ui_rect("core:dock.tray").is_some()
                || self.ui_rect("core:dock.pill").is_some();
            if !busy {
                return;
            }
            self.key(KeyCode::Escape).await;
        }
    }

    fn ui_text(&self) -> String {
        self.app.ui.snapshot()
    }

    fn count<Q: rim_sim::hecs::Query>(&self) -> usize {
        self.w().ecs.query::<Q>().iter().count()
    }
}

/// An open, empty square of `size` cells near `c`.
/// Mean difference of two screenshots over 32×20 blocks, per channel, 0 to
/// 255: small for the same picture at two resolutions, large for a flip
/// or a colour shift.
fn block_diff(a: &Image, b: &Image) -> f32 {
    let (w, h) = (a.width as usize, a.height as usize);
    if (w, h) != (b.width as usize, b.height as usize) {
        return f32::MAX;
    }
    let (bx, by) = (32, 20);
    let mut total = 0.0;
    for j in 0..by {
        for i in 0..bx {
            let mut sum = [0.0f32; 2];
            for (k, img) in [a, b].iter().enumerate() {
                for y in (j * h / by..(j + 1) * h / by).step_by(4) {
                    for x in (i * w / bx..(i + 1) * w / bx).step_by(4) {
                        let o = (y * w + x) * 4;
                        sum[k] += img.bytes[o..o + 3].iter().map(|&c| c as f32).sum::<f32>();
                    }
                }
            }
            let n = ((h / by).div_ceil(4) * (w / bx).div_ceil(4) * 3) as f32;
            total += (sum[0] - sum[1]).abs() / n;
        }
    }
    total / (bx * by) as f32
}

/// Mean difference of two screenshots, 0 to 255 per channel, in a square
/// of `r` points about the screen point `at`.
fn patch_diff(a: &Image, b: &Image, at: (f32, f32), r: f32) -> f32 {
    let dpi = screen_dpi_scale();
    let (w, h) = (a.width as i32, a.height as i32);
    let (cx, cy, rr) = ((at.0 * dpi) as i32, (at.1 * dpi) as i32, (r * dpi) as i32);
    let (mut sum, mut n) = (0.0, 0);
    for y in (cy - rr).max(0)..=(cy + rr).min(h - 1) {
        for x in (cx - rr).max(0)..=(cx + rr).min(w - 1) {
            // GL reads the bottom row first.
            let o = (((h - 1 - y) * w + x) * 4) as usize;
            sum += (0..3).map(|c| (a.bytes[o + c] as f32 - b.bytes[o + c] as f32).abs()).sum::<f32>();
            n += 3;
        }
    }
    sum / n.max(1) as f32
}

/// The grid over open ground at `o` (the pointer two cells in): the
/// screen with the select tool, with the wall tool armed, and mid-drag.
async fn grid_shots(t: &mut T, o: IVec, wall: rim_sim::defs::DefId) -> [Image; 3] {
    let at = t.screen(o.offset(2, 2));
    t.mouse = at;
    t.app.tool = Tool::Select;
    t.app.drag_start = None;
    // Long enough for the grid to fade out after its hold.
    for _ in 0..50 {
        t.frame().await;
    }
    let rest = t.grab().await;
    t.app.tool = Tool::Build(wall);
    for _ in 0..20 {
        t.frame().await;
    }
    let lens = t.grab().await;
    // From the cell before: a drag, not a click.
    t.app.drag_start = Some(o.offset(1, 1));
    for _ in 0..20 {
        t.frame().await;
    }
    let plan = t.grab().await;
    t.app.drag_start = None;
    t.app.tool = Tool::Select;
    [rest, lens, plan]
}

fn open_square(w: &World, c: IVec, size: i32) -> Option<IVec> {
    let free = |p: IVec| w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
    (2..30i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|o| (0..size).all(|y| (0..size).all(|x| free(o.offset(x, y)))))
}

/// The colour of a screenshot at a point, in logical points. The screen
/// reads back bottom row first (it's GL's framebuffer), so a row counted
/// from the top is counted from the other end.
fn px(img: &Image, (x, y): (f32, f32)) -> [f32; 3] {
    let dpi = screen_dpi_scale();
    let (w, h) = (img.width() as u32, img.height() as u32);
    let (xi, yi) = (((x * dpi) as u32).min(w - 1), ((y * dpi) as u32).min(h - 1));
    let c = img.get_pixel(xi, h - 1 - yi);
    [c.r, c.g, c.b]
}

/// How far apart two colours are: their channels' differences, summed.
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>()
}

/// A def's colour, as the renderer's 0 to 1.
fn of(rgb: [u8; 3]) -> [f32; 3] {
    [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0]
}

/// A point `(fx, fy)` of the way across cell `p`, on screen.
fn at(t: &T, p: IVec, fx: f32, fy: f32) -> (f32, f32) {
    t.app.cam.to_screen(p.x as f32 + fx, p.y as f32 + fy)
}

/// A field def's index.
fn field(t: &T, id: &str) -> usize {
    t.w().defs.lookup("field", id).unwrap() as usize
}

/// What a section leaves for the ones after it: each field is set by the
/// section that makes it, and read only by later ones.
#[derive(Default)]
pub(super) struct Carry {
    pub(super) defs: Option<std::sync::Arc<rim_sim::defs::DefDb>>,
    pub(super) founder: Option<Entity>,
    pub(super) home: Option<IVec>,
    pub(super) site: Option<IVec>,
    pub(super) wall: Option<rim_sim::defs::DefId>,
    pub(super) hut: Option<IVec>,
    pub(super) wood: Option<rim_sim::defs::DefId>,
    pub(super) stone: Option<rim_sim::defs::DefId>,
    pub(super) west: Option<IVec>,
    pub(super) spot: Option<IVec>,
    pub(super) slot: Option<IVec>,
    pub(super) light: Option<usize>,
    pub(super) cloud: Option<usize>,
    pub(super) zoom: Option<f32>,
    pub(super) calm: Option<[usize; 2]>,
}

pub async fn run(app: App, dir: PathBuf) -> ! {
    std::fs::create_dir_all(&dir).expect("create screenshot dir");
    let mut t = T { app, dir, shots: 0, passed: 0, failed: Vec::new(), mouse: (800.0, 480.0), clock: 0.0 };
    for _ in 0..3 {
        t.frame().await;
    }
    let defs = t.w().defs.clone();
    let founder = t.w().colonists().next().expect("a founder");
    let home = t.pawn(founder).pos;
    t.shot("start").await;

    let mut carry = Carry { defs: Some(defs), founder: Some(founder), home: Some(home), ..Carry::default() };
    ui::the_hud_is_core_s_ui_mod(&mut t).await;
    ui::toolbar(&mut t, &mut carry).await;
    input::camera(&mut t, &mut carry).await;
    input::right_click_orders(&mut t, &mut carry).await;
    tools::designate_build_cancel(&mut t, &mut carry).await;
    building::walls(&mut t, &mut carry).await;
    building::joins_in_quarters(&mut t, &mut carry).await;
    building::openings_turn_to_their_wall(&mut t, &mut carry).await;
    building::roofs_from_far_away(&mut t, &mut carry).await;
    building::room_state(&mut t, &mut carry).await;
    building::fences(&mut t, &mut carry).await;
    building::material_patterns(&mut t, &mut carry).await;
    building::room_labels(&mut t, &mut carry).await;
    building::the_plan_style(&mut t, &mut carry).await;
    building::house_plans(&mut t, &mut carry).await;
    building::materials(&mut t, &mut carry).await;
    replace::replace_in_place(&mut t, &mut carry).await;
    tools::orders(&mut t, &mut carry).await;
    ui::hud(&mut t, &mut carry).await;
    tools::stockpiles(&mut t, &mut carry).await;
    tools::stances(&mut t, &mut carry).await;
    ui::profiler(&mut t).await;
    ui::field_overlay(&mut t, &mut carry).await;
    ui::devtools(&mut t).await;
    ui::render_cost(&mut t).await;
    ui::ui_budget_live(&mut t).await;
    weather::weather(&mut t, &mut carry).await;
    light::sun_shadows(&mut t, &mut carry).await;
    light::firelight(&mut t, &mut carry).await;
    indoors::indoors(&mut t, &mut carry).await;
    indoors::roofs_take_the_sun(&mut t, &mut carry).await;
    light::moving_lights(&mut t, &mut carry).await;
    light::lighting_from_the_palette(&mut t).await;
    input::camera_by_device(&mut t, &mut carry).await;
    input::safe_right_click(&mut t, &mut carry).await;
    chalk::the_grid(&mut t, &mut carry).await;
    chalk::several_selected(&mut t, &mut carry).await;
    chalk::an_urgent_hunt_shows(&mut t, &mut carry).await;
    chalk::unreachable(&mut t, &mut carry).await;
    println!("\n{} passed, {} failed; screenshots in {}", t.passed, t.failed.len(), t.dir.display());
    for f in &t.failed {
        println!("  FAIL {f}");
    }
    std::process::exit(if t.failed.is_empty() { 0 } else { 1 });
}

/// How many shapes `draw::thing` paints for `e` in `cell`: a thing wider
/// than a cell must paint everything at its anchor and nothing elsewhere.
fn tally(app: &App, e: Entity, cell: IVec, z: f32) -> usize {
    struct Tally<'a>(&'a crate::atlas::WorldAtlas, usize);
    impl draw::Sink for Tally<'_> {
        fn rect(&mut self, _: f32, _: f32, _: f32, _: f32, _: Color) {
            self.1 += 1;
        }
        fn poly(&mut self, _: f32, _: f32, _: u8, _: f32, _: Color) {
            self.1 += 1;
        }
        fn line(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Color) {
            self.1 += 1;
        }
        fn tri(&mut self, _: [[f32; 2]; 3], _: Color) {
            self.1 += 1;
        }
        fn image(&mut self, _: f32, _: f32, _: f32, _: f32, _: crate::atlas::Slot, _: Color) {
            self.1 += 1;
        }
        fn atlas(&self) -> &crate::atlas::WorldAtlas {
            self.0
        }
    }
    let mut s = Tally(&app.world_atlas, 0);
    draw::thing(&mut s, &app.sim.world, e, cell, (0.0, 0.0), z, 0.0, Default::default());
    s.1
}

/// How far, in points, `e`'s speech bubble's centre is from `e` across the
/// screen, as last drawn: the anchored label above it with a panel.
fn bubble_offset(t: &T, e: Entity) -> Option<f32> {
    let dpi = screen_dpi_scale();
    let anchor = rim_ui::node::Anchor::Entity(e.to_bits().get());
    let bubble = t
        .app
        .last_anchored
        .iter()
        .filter(|a| a.anchor == anchor)
        .filter_map(|a| {
            t.app.last_draw[a.draws.clone()].iter().find_map(|d| match d {
                rim_ui::paint::Draw::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
        })
        .min_by(|a, b| a[1].total_cmp(&b[1]))?;
    let (x, _) = t.pawn_screen(e);
    Some((bubble[0] + bubble[2] / 2.0) / dpi - x)
}
