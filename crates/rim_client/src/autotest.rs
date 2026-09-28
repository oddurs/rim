//! `rim --autotest [dir]` drives the real client through every control.
//!
//! It feeds synthetic raw input through the same `frame()` the game loop
//! uses: world clicks and drags pass through the UI's routing exactly as the
//! mouse's would, and UI controls are clicked by node id (`core:toolbar.
//! designate:core:chop`), not screen position. It checks the game state after
//! each step, saves screenshots to `dir` (default `target/autotest`), and
//! exits non-zero if any check failed.

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
            if get_time() - since > 3.0 {
                self.check(false, "the firelight bake caught up within 3 s");
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
        let (x, y) = draw::pawn_pos(&self.pawn(e), self.app.tick_frac());
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

    // ---------------------------------------------------------- 0171 the HUD is core's UI mod
    // A castaway on Auto gets a first-day hint over the map; a player would
    // read it and wave it away, and it would sit under clicks meant for the map.
    println!("\n# the first-day hint (5dc8f858)");
    t.check(t.app.ui.find("core:auto_hint").is_some(), "a castaway on Auto gets the first-day hint");
    t.click_ui("core:auto_hint.ok").await;
    t.check(t.app.ui.find("core:auto_hint").is_none(), "Got it waves it away");

    println!("\n# the HUD is a mod (0171)");
    for id in [
        "core:topbar",
        "core:clock",
        "core:status",
        "core:colonists",
        "core:toolbar",
        "core:messages",
        "core:inspector",
    ] {
        let found = t.app.ui.find(id).is_some();
        t.check(found, format!("core's UI draws '{id}'"));
    }
    t.check(t.app.ui.warnings().is_empty(), format!("core's UI loads cleanly ({:?})", t.app.ui.warnings()));
    let font = t.app.ui.info.font.clone();
    t.check(!font.is_empty(), format!("UI font: {font}"));

    // ---------------------------------------------------------- 0046 toolbar
    println!("\n# toolbar (0046)");
    let keys: Vec<String> = t.app.tools.iter().map(|b| b.key.clone()).collect();
    let markable = (0..defs.designations.len()).filter(|&d| crate::markable(&defs, d as rim_sim::defs::DefId)).count();
    // Select, cancel, stockpile, clear zone and save as plan, besides one
    // per def and one per house plan.
    let n_expected = 5 + markable + defs.things.iter().filter(|d| d.build.is_some()).count() + defs.plans.len();
    t.check(keys.len() == n_expected, format!("one tool per markable designation and buildable def ({})", keys.len()));
    for k in &keys {
        t.click_tool(k).await;
        t.frame().await;
        let picked = t.app.tools.iter().find(|b| b.key == *k).is_some_and(|b| b.tool == t.app.tool);
        t.check(picked, format!("the dock picks '{k}'"));
    }
    // The dock is one row whatever is loaded, and every tray, group by
    // group, fits a 1280-point screen with room to spare.
    let dock = t.ui_rect("core:dock").map(|r| r[3]);
    t.check(dock.is_some_and(|h| h < 60.0), format!("the dock is one row ({dock:?})"));
    let mut widest = 0.0f32;
    let groups: Vec<String> = t.app.tools.iter().filter(|b| b.category == "build").map(|b| b.group.clone()).collect();
    for (key, category) in [(KeyCode::Q, "orders"), (KeyCode::B, "build"), (KeyCode::Z, "zones")] {
        t.clear_dock().await;
        t.key(key).await;
        t.frame().await;
        let group_list: Vec<String> = if category == "build" { groups.clone() } else { vec![String::new()] };
        for g in group_list {
            if !g.is_empty() {
                t.click_ui(&format!("core:dock.groups.{g}")).await;
                t.frame().await;
            }
            match t.ui_rect("core:dock.tray") {
                Some(r) => widest = widest.max(r[2]),
                None => t.check(false, format!("the {category} tray opens")),
            }
        }
    }
    t.clear_dock().await;
    t.click_tool("build:core:wall").await;
    t.check(widest > 0.0 && widest < 1280.0 - 64.0, format!("the widest tray fits at 1280 ({widest:.0} pt)"));
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(t.app.tool == Tool::Select, "Escape drops the tool");
    t.check(t.ui_rect("core:dock.tray").is_some(), "and brings its tray back");
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(t.ui_rect("core:dock.tray").is_none(), "a second Escape closes the tray");
    let selected = t.app.selected;
    let docked = (t.ui_rect("core:dock"), t.ui_rect("core:inspector"), t.ui_rect("core:colonists"));
    t.key(KeyCode::B).await;
    t.frame().await;
    t.check(t.ui_rect("core:dock.palette.build").is_some(), "B opens the Build palette");
    let now = (t.ui_rect("core:dock"), t.ui_rect("core:inspector"), t.ui_rect("core:colonists"));
    t.check(now == docked, format!("the tray floats: nothing docked moved ({docked:?} → {now:?})"));
    t.shot("build_tray").await;
    // Tab and Shift+Tab step through the open tray's groups (5689930d):
    // [ and ] are levels now.
    let shown = |t: &T| -> Vec<String> {
        t.app
            .tools
            .iter()
            .map(|b| b.key.clone())
            .filter(|k| t.ui_rect(&format!("core:toolbar.{k}")).is_some())
            .collect()
    };
    let first = shown(&t);
    t.key(KeyCode::Tab).await;
    let second = shown(&t);
    t.check(!second.is_empty() && second != first, "Tab shows the open tray's next group");
    t.check(t.app.selected == selected, "and leaves the selection alone");
    let raw =
        RawInput { keys: vec![KeyCode::Tab], pressed: vec!["shift+tab".into()], shift: true, ..Default::default() };
    t.input(RawInput { mouse: t.mouse, ..raw }).await;
    t.settle().await;
    t.check(shown(&t) == first, "Shift+Tab goes back");
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(
        t.ui_rect("core:dock.palette.build").is_none() && t.app.selected == selected,
        "Escape closes an open palette before it touches the selection",
    );

    // ---------------------------------------------------------- 0045 camera
    println!("\n# camera (0045)");
    let (x0, y0) = (t.app.cam.x, t.app.cam.y);
    t.act(Action::Pan(3.0, -2.0));
    t.check((t.app.cam.x - x0 - 3.0).abs() < 1e-4 && (t.app.cam.y - y0 + 2.0).abs() < 1e-4, "pan moves the camera");
    t.act(Action::Pan(-3.0, 2.0));
    let z0 = t.app.cam.zoom;
    let anchor = (500.0, 400.0);
    let before = t.app.cam.to_world(anchor.0, anchor.1);
    t.act(Action::Zoom(1.5, anchor.0, anchor.1));
    let after = t.app.cam.to_world(anchor.0, anchor.1);
    t.check((t.app.cam.zoom - z0 * 1.5).abs() < 1e-3, "wheel zoom changes scale");
    t.check(
        (before.0 - after.0).abs() < 1e-3 && (before.1 - after.1).abs() < 1e-3,
        "zoom keeps the point under the cursor",
    );
    t.act(Action::Zoom(1.0 / 1.5, anchor.0, anchor.1));
    for _ in 0..40 {
        t.act(Action::Zoom(0.5, 0.0, 0.0));
    }
    t.check(t.app.cam.zoom >= 4.0, "zoom is clamped");
    t.act(Action::Zoom(z0 / t.app.cam.zoom, 0.0, 0.0));
    t.focus(home);

    // ---------------------------------------------------------- 0071 right-click orders
    println!("\n# right-click orders (0071)");
    t.frame().await;
    let at = t.pawn_screen(founder);
    t.click(at).await;
    t.check(t.app.selected == Some(founder), "clicking a colonist selects them");
    t.check(!t.pawn(founder).drafted, "and they start undrafted");

    let oak = defs.thing_id("tree_oak").unwrap();
    let hp = t.pawn(founder).pos;
    // With the stone age on, an oak waits for an axe: hand one over.
    if let Some(axe) = defs.thing_id("primitive:hand_axe") {
        t.app.sim.world.place_item(axe, hp, 1);
    }
    let tree = {
        let w = t.w();
        let mut best: Option<(u32, Entity, IVec)> = None;
        for (e, th) in w.ecs.query::<(Entity, &Thing)>().without::<&Blueprint>().iter() {
            let d = th.pos.octile(hp);
            if th.def == oak && best.is_none_or(|b| d < b.0) && w.map.can_reach(hp, Goal::Touch(th.pos)) {
                best = Some((d, e, th.pos));
            }
        }
        best.expect("a reachable oak")
    };
    // Marked for chopping, so a click chops it: unmarked, a click prefers
    // the gentlest harvest, and the stone age lets you gather an oak.
    let chop = defs.lookup("designation", "chop").unwrap();
    t.app.sim.push(Command::Designate { designation: chop, a: tree.2, b: tree.2 });
    t.ticks(1);
    t.focus(tree.2);
    let (tx, ty) = t.screen(tree.2);
    t.input(RawInput { mouse: (tx, ty), ..Default::default() }).await;
    t.frame().await;
    let hint = order::resolve(t.w(), founder, tree.2, None).map(|o| o.label);
    t.check(hint.as_deref() == Some("Chop oak tree"), format!("the order resolves ({hint:?})"));
    t.check(t.app.ui.find("core:hint").is_some(), "the cursor label shows it (core:hint)");
    t.check(t.ui_text().contains("Chop oak tree"), "and says what the click will do");
    t.shot("order").await;

    // A click on a thing selects it, and the inspector shows it (5305a161).
    t.click((tx, ty)).await;
    t.check(t.app.selected == Some(tree.1), "clicking a tree selects it");
    t.frame().await;
    t.check(t.app.ui.find("core:inspector.thing").is_some(), "the inspector shows the tree");
    t.shot("thing").await;
    // Selection is chalk brackets just outside the footprint (d83192ed).
    let marks = crate::overlay::scene(&t.app).marks;
    let (cx, cy) = t.screen(tree.2);
    let z = t.app.cam.zoom;
    let around = marks.iter().filter(|m| {
        matches!(m, Mark::Brackets { rect, alpha, .. }
        if (rect[0] + z / 2.0 - cx).abs() < 0.5 && (rect[1] + z / 2.0 - cy).abs() < 0.5 && (rect[2] - z).abs() < 0.5 && *alpha == 1.0)
    });
    let sets = marks.iter().filter(|m| matches!(m, Mark::Brackets { .. })).count();
    t.check(around.count() == 1 && sets == 1, format!("a selected tree gets one set of brackets ({marks:?})"));
    t.shot("chalk-select-thing").await;
    // Hover: an edge on what a click would pick, and nothing on bare
    // ground or over a panel (bd7a158e).
    t.mouse = (tx, ty);
    for _ in 0..8 {
        t.frame().await;
    }
    let hovers = |t: &T| {
        crate::overlay::scene(&t.app)
            .marks
            .into_iter()
            .filter(|m| matches!(m, Mark::Hover { .. } | Mark::HoverRing { .. }))
            .collect::<Vec<_>>()
    };
    let on_tree = hovers(&t);
    let z = t.app.cam.zoom;
    let edge = on_tree.iter().any(|m| {
        matches!(m, Mark::Hover { rect, alpha }
        if (rect[0] + z / 2.0 - tx).abs() < 0.5 && (rect[1] + z / 2.0 - ty).abs() < 0.5 && *alpha == 1.0)
    });
    t.check(edge && on_tree.len() == 1, format!("hovering the tree puts an edge on its cell ({on_tree:?})"));
    t.shot("chalk-hover").await;
    // Bare ground: nothing there, no floor, no stockpile, no pawn near.
    let bare = {
        let w = t.w();
        let clear = |p: IVec| {
            w.map.fixture_at(p).is_none()
                && w.map.item_at(p).is_none()
                && w.map.floor_at(p).is_none()
                && w.zones.at(&w.map, p).is_none()
                && w.pawns.iter().all(|&e| w.pawn_pos(e).is_none_or(|q| (q.x - p.x).abs() + (q.y - p.y).abs() > 2))
        };
        (3..12).flat_map(|r| (-r..=r).map(move |d| tree.2.offset(d, r))).find(|&p| clear(p))
    };
    match bare {
        Some(p) => t.mouse = t.screen(p),
        None => t.check(false, "bare ground near the tree to hover"),
    }
    for _ in 0..12 {
        t.frame().await;
    }
    let none = hovers(&t);
    t.check(none.is_empty(), format!("bare ground gets no hover ({none:?})"));
    match t.ui_rect("core:dock") {
        Some(r) => {
            t.mouse = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
            for _ in 0..12 {
                t.frame().await;
            }
            let none = hovers(&t);
            t.check(none.is_empty(), format!("a panel over the map gets no hover ({none:?})"));
        }
        None => t.check(false, "the dock is there to hover"),
    }
    let at = t.pawn_screen(founder);
    t.click(at).await;
    t.check(t.app.selected == Some(founder), "and a click on the colonist selects them again");
    // A colonist's panel has tabs (4ad6b386).
    t.frame().await;
    t.click_ui("core:inspector.tabs.skills").await;
    t.check(t.app.ui.find("core:inspector.skills").is_some(), "the Skills tab lists every skill");
    t.shot("inspector_skills").await;
    t.click_ui("core:inspector.tabs.work").await;
    t.check(t.app.ui.find("core:inspector.work.core:build").is_some(), "the Work tab lists the work types");
    t.click_ui("core:inspector.tabs.overview").await;
    t.check(t.app.ui.find("core:inspector.bars").is_some(), "and Overview has the needs again");

    t.right_click((tx, ty)).await;
    t.ticks(1); // commands apply on the next tick
    t.check(
        matches!(t.pawn(founder).job, Job::Harvest { target, .. } if target == tree.1),
        "right-click sends an undrafted colonist to chop",
    );
    t.check(t.app.order_flash.is_some(), "the order is acknowledged on the map");
    // The chop itself: blows throw chips once the axe lands (DESIGN.md §6b).
    let zoom = t.app.cam.zoom;
    t.app.cam.zoom = 48.0;
    let mut thrown = false;
    for _ in 0..600 {
        t.ticks(2);
        t.frame().await;
        if !t.app.worksites.parts.is_empty() {
            thrown = true;
            break;
        }
    }
    t.check(thrown, "chopping throws chips toward the colonist");
    t.frame().await;
    t.shot("chopping").await;
    t.app.cam.zoom = zoom;
    for _ in 0..3000 {
        t.ticks(1);
        if t.w().thing(tree.1).is_none() {
            break;
        }
    }
    t.check(t.w().thing(tree.1).is_none(), "the ordered tree comes down");
    t.focus(home);

    // ---------------------------------------------------------- 0046 designate / build / cancel
    println!("\n# designate, build, cancel (0046)");
    let chop = defs.lookup("designation", "chop").unwrap();
    t.click_tool("designate:core:chop").await;
    t.check(t.app.tool == Tool::Designate(chop), "clicking Chop selects the chop tool");
    // Drag over the trees nearest home, wherever this map put them.
    let oak = defs.thing_id("tree_oak").unwrap();
    let near_tree = t
        .w()
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|th| th.def == oak)
        .map(|th| th.pos)
        .min_by_key(|p| (p.octile(home), p.x, p.y));
    let (a, b) = match near_tree {
        Some(p) => (p.offset(-4, -4), p.offset(4, 4)),
        None => (home.offset(-8, -8), home.offset(8, 8)),
    };
    t.drag(a, b).await;
    t.ticks(1);
    let designated = t.count::<(&Thing, &Designated)>();
    t.check(designated > 0 || near_tree.is_none(), format!("dragging designates trees ({designated})"));
    let wrong = t.w().ecs.query::<(&Thing, &Designated)>().iter().filter(|(_, d)| d.0 != chop).count();
    t.check(wrong == 0, "only chop designations were made");

    let site = open_square(t.w(), home, 6).expect("open ground for a hut");
    let wall = defs.thing_id("wall").unwrap();
    t.click_tool("build:core:wall").await;
    t.check(t.app.tool == Tool::Build(wall), "clicking wooden wall selects the wall tool");
    let (ax, ay) = t.screen(site);
    let (bx, by) = t.screen(site.offset(5, 5));
    t.input(RawInput { mouse: (ax, ay), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: (bx, by), ..Default::default() }).await;
    t.frame().await;
    t.check(t.ui_text().contains("6 × 6"), "dragging shows its size at the cursor");
    t.input(RawInput { mouse: (bx, by), left_released: true, ..Default::default() }).await;
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 20,
        format!("wall drag places the 20-cell outline ({})", t.count::<&Blueprint>()),
    );
    t.check(t.w().map.fixture_at(site.offset(2, 2)).is_none(), "the inside of the outline stays empty");

    t.click_tool("cancel").await;
    t.drag(site, site.offset(5, 0)).await;
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 14,
        format!("cancel removes the dragged row ({} left)", t.count::<&Blueprint>()),
    );
    t.click_tool("build:core:wall").await;
    t.drag(site, site.offset(4, 0)).await;
    let (door, bed) = (defs.thing_id("door").unwrap(), defs.thing_id("bed").unwrap());
    t.click_tool("build:core:door").await;
    t.check(t.app.tool == Tool::Build(door), "clicking door selects the door tool");
    t.drag(site.offset(5, 0), site.offset(5, 0)).await;
    t.click_tool("build:core:bed").await;
    t.check(t.app.tool == Tool::Build(bed), "clicking bed selects the bed tool");
    t.drag(site.offset(2, 2), site.offset(2, 2)).await;
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 21,
        format!("walls, a door and a bed are planned ({})", t.count::<&Blueprint>()),
    );
    // Drawing the room again over its door: the ring skips what is there.
    t.click_tool("build:core:wall").await;
    t.drag(site, site.offset(5, 5)).await;
    t.ticks(1);
    let kept = t.w().map.fixture_at(site.offset(5, 0)).and_then(|e| t.w().thing(e)).map(|th| th.def);
    t.check(
        kept == Some(door) && t.count::<&Blueprint>() == 21,
        format!("a ring drawn over a door keeps the door ({kept:?}, {} plans)", t.count::<&Blueprint>()),
    );
    t.right_click((600.0, 500.0)).await;
    t.check(t.app.tool == Tool::Select, "right-click drops the current tool");
    t.shot("plans").await;

    // Let the warrior work for a while.
    t.act(Action::Speed(6));
    t.check(t.app.speed == 6 && !t.app.paused, "speed 6x");
    let mut saw_interp = false;
    // Long enough to chop, haul and build on any map: stop early once a wall
    // stands and the walking interpolation has been seen.
    let built_walls =
        |t: &T| t.w().ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|th| th.def == wall).count();
    for i in 0..12000 {
        if i >= 6000 && saw_interp && built_walls(&t) > 0 {
            break;
        }
        t.ticks(1);
        if !saw_interp {
            let p = t.pawn(founder);
            if p.next.is_some() && p.progress > 0 && p.progress < p.step_ticks {
                let (x, y) = draw::pawn_pos(&p, t.app.tick_frac());
                saw_interp = (x.fract() - 0.5).abs() > 1e-3 || (y.fract() - 0.5).abs() > 1e-3;
            }
        }
    }
    t.check(saw_interp, "pawns are drawn between cells while walking");
    let built = t.w().ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|th| th.def == wall).count();
    t.check(built > 0, format!("the warrior chopped and built walls ({built})"));
    t.focus(site.offset(3, 3));
    t.shot("building").await;

    // ---------------------------------------------------------- 0220 walls
    println!("\n# walls join, in the colour of what they are made of (0220)");
    let stone = defs.thing_id("stone").unwrap();
    let wood = defs.thing_id("wood").unwrap();
    // A free row of three cells: wood, wood, stone.
    let row = (2..30i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
        .find(|&o| {
            (0..3).all(|i| {
                let p = o.offset(i, 0);
                t.w().map.passable(p) && t.w().map.fixture_at(p).is_none() && t.w().map.item_at(p).is_none()
            }) && (-1..=3).all(|i| {
                t.w().map.fixture_at(o.offset(i, -1)).is_none() && t.w().map.fixture_at(o.offset(i, 1)).is_none()
            }) && t.w().map.fixture_at(o.offset(-1, 0)).is_none()
                && t.w().map.fixture_at(o.offset(3, 0)).is_none()
                // A pawn's token and name label would cover the seam.
                && t.w().pawns.iter().filter_map(|&e| t.w().pawn_pos(e)).all(|p| p.chebyshev(o.offset(1, 0)) > 4)
        })
        .expect("a free row for three walls, with nothing at either end and no pawn near");
    for (i, m) in [wood, wood, stone].into_iter().enumerate() {
        t.app.sim.world.spawn_fixture_of(wall, row.offset(i as i32, 0), false, Some(m)).expect("a wall");
    }
    // The right-click ring from earlier fades on the wall clock: on a slow
    // frame it's still over the seam when the screenshot is taken.
    t.app.order_flash = None;
    t.focus(row.offset(1, 0));
    let img = t.grab().await;
    let dpi = screen_dpi_scale();
    let z = t.app.cam.zoom;
    // The screen reads back bottom row first (it's GL's framebuffer), so a
    // row counted from the top is counted from the other end.
    let px = |img: &Image, (x, y): (f32, f32)| {
        let (w, h) = (img.width() as u32, img.height() as u32);
        let (xi, yi) = (((x * dpi) as u32).min(w - 1), ((y * dpi) as u32).min(h - 1));
        let c = img.get_pixel(xi, h - 1 - yi);
        [c.r, c.g, c.b]
    };
    let dist = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>();
    let of = |rgb: [u8; 3]| [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0];
    let wood_c = px(&img, t.screen(row));
    let stone_c = px(&img, t.screen(row.offset(2, 0)));
    t.check(dist(wood_c, [0.0; 3]) > 0.1, format!("the frame was read back, not a cleared buffer ({wood_c:?})"));
    t.check(
        dist(wood_c, stone_c) > 0.15,
        format!("a wood wall and a stone wall look different ({wood_c:?} vs {stone_c:?})"),
    );
    // Lighting tints everything alike, so ask which colour the stone wall
    // is nearer: its material's, or the wall def's own brown.
    let (to_stone, to_def) = (dist(stone_c, of(defs.thing(stone).rgb)), dist(stone_c, of(defs.thing(wall).rgb)));
    t.check(
        to_stone < to_def,
        format!(
            "the stone wall takes its colour from the material def, not the wall def ({to_stone:.2} vs {to_def:.2})"
        ),
    );
    // The seam between the two wood walls carries no outline; the run's west end does.
    // A seam is an outline across the joint, dark its whole height; the
    // wood's own plank lines run along it, and one pixel can land on one.
    // So look down a short span of the joint for the fill.
    let (cx, cy) = t.screen(row);
    let seam = (-3..=3)
        .map(|k| px(&img, (cx + z / 2.0, cy + k as f32 * z / 10.0)))
        .min_by(|a, b| dist(*a, wood_c).total_cmp(&dist(*b, wood_c)))
        .expect("seven samples");
    t.check(dist(seam, wood_c) < 0.08, format!("no seam between joined walls ({seam:?} vs fill {wood_c:?})"));
    // Wood meets stone at the second wall's east side: a hairline, darker than either.
    let (sx2, _) = t.screen(row.offset(2, 0));
    let change = [0.0, 0.5, 1.0, 1.5]
        .into_iter()
        .map(|dx| px(&img, (sx2 - z / 2.0 + dx, cy)))
        .fold(f32::INFINITY, |m, c| m.min(c.iter().sum::<f32>()));
    t.check(
        change < stone_c.iter().sum::<f32>().min(wood_c.iter().sum::<f32>()) - 0.05,
        format!("a hairline where wood meets stone (darkest {change:.2} against {wood_c:?} and {stone_c:?})"),
    );
    // The edge is a 1.5px line on the cell's border; where it rasterises is
    // the renderer's business, so look across the first two pixels.
    let end = [0.0, 0.5, 1.0, 1.5]
        .into_iter()
        .map(|dx| dist(px(&img, (cx - z / 2.0 + dx, cy)), wood_c))
        .fold(0.0f32, f32::max);
    t.check(
        end > 0.12,
        format!("the end of the run has an edge (strongest contrast {end:.2} against fill {wood_c:?})"),
    );
    t.shot("walls").await;

    // ---------------------------------------------------------- joins in quarters
    println!("\n# a wall run draws as one mass, round where it ends (DESIGN.md §6c)");
    // Post, run, corner, tee, cross and block, each in a 3×3 slot.
    let shapes: [&[(i32, i32)]; 6] = [
        &[(1, 1)],
        &[(0, 1), (1, 1), (2, 1)],
        &[(1, 1), (2, 1), (1, 2)],
        &[(0, 1), (1, 1), (2, 1), (1, 2)],
        &[(0, 1), (1, 1), (2, 1), (1, 0), (1, 2)],
        &[(0, 1), (1, 1), (2, 1), (1, 2), (2, 2)],
    ];
    let (sw, sh) = (4 * shapes.len() as i32, 4);
    let slot = (2..40i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
        .find(|&o| {
            // Open ground, nothing built, and no pawn standing on it; plants
            // are cleared below.
            (-1..=sw).all(|x| {
                (-1..=sh).all(|y| {
                    let p = o.offset(x, y);
                    t.w().map.inb(p)
                        && t.w().map.passable(p)
                        && t.w()
                            .map
                            .fixture_at(p)
                            .is_none_or(|e| t.w().thing(e).is_some_and(|th| t.w().defs.thing(th.def).natural))
                })
            }) && t
                .w()
                .pawns
                .iter()
                .filter_map(|&e| t.w().pawn_pos(e))
                .all(|p| !(o.x - 2..=o.x + sw + 2).contains(&p.x) || !(o.y - 2..=o.y + sh + 2).contains(&p.y))
        })
        .expect("a clear strip for the join shapes");
    for x in -1..=sw {
        for y in -1..=sh {
            if let Some(e) = t.w().map.fixture_at(slot.offset(x, y)) {
                t.app.sim.world.despawn_thing(e);
            }
        }
    }
    for (i, cells) in shapes.iter().enumerate() {
        for &(x, y) in cells.iter() {
            t.app
                .sim
                .world
                .spawn_fixture_of(wall, slot.offset(4 * i as i32 + x, y), false, Some(wood))
                .expect("a wall");
        }
    }
    let zoom = t.app.cam.zoom;
    t.app.cam.zoom = 40.0;
    t.focus(slot.offset(sw / 2, 1));
    // Chunks are drawn scaled from the zoom they were built at until it
    // settles (mesh.rs): point-sized detail is only right after a rebuild.
    for _ in 0..20 {
        t.frame().await;
    }
    let img = t.grab().await;
    let z = t.app.cam.zoom;
    let at = |t: &T, p: IVec, fx: f32, fy: f32| t.app.cam.to_screen(p.x as f32 + fx, p.y as f32 + fy);
    let post = slot.offset(1, 1);
    let fill = px(&img, at(&t, post, 0.5, 0.5));
    let corner = px(&img, at(&t, post, 2.0 / z, 2.0 / z));
    t.check(dist(corner, fill) > 0.12, format!("a lone post's corner is rounded off ({corner:?} vs fill {fill:?})"));
    // The corner shape's inner corner stays square: just inside it is wall.
    let l = slot.offset(4 * 2 + 1, 1);
    let inner = px(&img, at(&t, l, 1.0 - 2.0 / z, 1.0 - 2.0 / z));
    t.check(dist(inner, fill) < 0.1, format!("an inner corner is square ({inner:?} vs fill {fill:?})"));
    // Where a run meets its neighbour there is no round and no seam.
    let run = slot.offset(4 + 1, 1);
    // Below the lit edge, and well inside where a round would cut.
    let joint = px(&img, at(&t, run, 1.0 - 1.0 / z, 5.0 / z));
    t.check(dist(joint, fill) < 0.1, format!("a joined corner is square and seamless ({joint:?} vs fill {fill:?})"));
    // One light, from the north-west: an open top side catches it just
    // inside the outline; the bottom doesn't.
    let base: f32 = fill.iter().sum();
    // The brightest pixel in a band along the run's middle cell.
    let band = |y0: f32, y1: f32| {
        let mut best = 0.0f32;
        for i in 0..12 {
            for j in 0..8 {
                let (fx, fy) = (0.2 + 0.6 * i as f32 / 11.0, y0 + (y1 - y0) * j as f32 / 7.0);
                best = best.max(px(&img, at(&t, run, fx, fy)).iter().sum::<f32>());
            }
        }
        best
    };
    let (top, bottom) = (band(1.0 / z, 3.0 / z), band(1.0 - 3.0 / z, 1.0 - 1.0 / z));
    t.check(
        top > base + 0.1 && bottom < base + 0.05,
        format!("the top edge catches the light ({top:.2}) and the bottom doesn't ({bottom:.2}) against {base:.2}"),
    );
    t.shot("joins").await;

    // ---------------------------------------------------------- openings turn to their wall
    println!("\n# a door turns to its wall and swings into the room (DESIGN.md §6c)");
    let door = defs.thing_id("door").unwrap();
    let hut = slot.offset(0, 6);
    // Grass all round, whatever the map put here: the checks tell wood from
    // the ground by colour, and dirt is nearly wood.
    let grass = defs.lookup("terrain", "core:grass").expect("grass");
    let grass_cost = defs.terrain[grass as usize].path_cost;
    for x in -3..=7 {
        for y in -1..=6 {
            if let Some(e) = t.w().map.fixture_at(hut.offset(x, y)) {
                t.app.sim.world.despawn_thing(e);
            }
            t.app.sim.world.map.set_terrain(hut.offset(x, y), grass, grass_cost);
        }
    }
    let west = hut.offset(0, 2);
    for y in 0..5 {
        for x in 0..5 {
            if x == 0 || y == 0 || x == 4 || y == 4 {
                let def = if hut.offset(x, y) == west { door } else { wall };
                t.app.sim.world.spawn_fixture_of(def, hut.offset(x, y), false, Some(wood)).expect("a hut piece");
            }
        }
    }
    t.app.sim.world.map.ensure_rooms();
    t.focus(west);
    let img = t.grab().await;
    let z = t.app.cam.zoom;
    // Wood is red over green and the grass laid above is green over red,
    // whatever shade a cell's variation or a wall's edge gives them. (By
    // distance to wood, a darkened edge sat halfway to the grass.)
    let near = |c: [f32; 3]| c[0] > c[1] + 0.03;
    // In a north–south wall the wall's ends are the door's top and bottom.
    let jamb = px(&img, at(&t, west, 0.5, 2.0 / z));
    let side = px(&img, at(&t, west, 2.0 / z, 0.5));
    t.check(
        near(jamb) && !near(side),
        format!("the door's jambs turned to a north–south wall ({jamb:?}, side {side:?})"),
    );
    // The leaf stands open toward the hut, across the door cell's east
    // half just below the jamb, and nothing stands in the west half. Both
    // are in the doorway, under the same light: wood is redder than grass.
    let woody = |c: [f32; 3]| c[0] > c[1];
    let count = |x0: f32, x1: f32| {
        let mut n = 0;
        for i in 0..=20 {
            for j in 0..=8 {
                let (fx, fy) = (x0 + (x1 - x0) * i as f32 / 20.0, 0.13 + 0.15 * j as f32 / 8.0);
                n += woody(px(&img, at(&t, west, fx, fy))) as usize;
            }
        }
        n
    };
    let (east, west_half) = (count(0.55, 0.98), count(0.02, 0.45));
    t.check(
        east >= 10 && west_half == 0,
        format!("the leaf swings into the room ({east} wood samples east of the hinge, {west_half} west)"),
    );
    t.shot("door").await;

    // ---------------------------------------------------------- roofs from far away
    println!("\n# zoomed out, a house has its roof, and pointing at it lifts it (DESIGN.md §6c)");
    let inside = west.offset(2, 0);
    t.app.cam.zoom = 8.0;
    t.focus(inside);
    // The pointer away from the hut, then over it.
    t.mouse = (4.0, 200.0);
    let img = t.grab().await;
    let shingle = [0x7d as f32 / 255.0, 0x5d as f32 / 255.0, 0x3d as f32 / 255.0];
    let roofed = px(&img, t.screen(inside));
    let hue = |c: [f32; 3]| (c[0] - c[2]) / (c[0] + c[1] + c[2]).max(0.01);
    t.check(
        (hue(roofed) - hue(shingle)).abs() < 0.08,
        format!("the hut has a shingle roof ({roofed:?} against shingle {shingle:?})"),
    );
    t.shot("roofs").await;
    t.mouse = t.screen(inside);
    let img = t.grab().await;
    let lifted = px(&img, t.screen(inside));
    t.check(dist(lifted, roofed) > 0.1, format!("pointing at the hut lifts its roof ({lifted:?} was {roofed:?})"));
    t.mouse = (4.0, 200.0);
    t.app.cam.zoom = 40.0;
    // ---------------------------------------------------------- room state
    println!("\n# a gap in a ring of walls is marked (DESIGN.md §6c)");
    // Knock out the hut's east wall, opposite its door.
    let gap = hut.offset(4, 2);
    if let Some(e) = t.w().map.fixture_at(gap) {
        t.app.sim.world.despawn_thing(e);
    }
    t.app.sim.world.map.ensure_rooms();
    t.focus(gap);
    t.grab().await;
    t.check(
        t.app.marks.gaps.iter().any(|&(p, _)| p == gap),
        format!("the gap is marked at {gap:?} ({:?})", t.app.marks.gaps),
    );
    t.shot("gap").await;
    t.app.sim.world.spawn_fixture_of(wall, gap, false, Some(wood)).expect("the wall back");
    t.app.sim.world.map.ensure_rooms();
    t.grab().await;
    t.check(!t.app.marks.gaps.iter().any(|&(p, _)| p == gap), "walled up again, no mark");
    // ---------------------------------------------------------- fences
    println!("\n# a fence runs into a wall with no post, and posts stand where it needs them (DESIGN.md §6c)");
    let fence = defs.thing_id("fence").unwrap();
    let mut fences = Vec::new();
    for k in 1..=8 {
        fences.push(hut.offset(4 + k, 2));
    }
    for x in 3..=12 {
        fences.push(hut.offset(x, 6));
    }
    for y in 3..=5 {
        fences.push(hut.offset(12, y));
    }
    for &p in &fences {
        if let Some(e) = t.w().map.fixture_at(p) {
            t.app.sim.world.despawn_thing(e);
        }
        t.app.sim.world.spawn_fixture_of(fence, p, false, Some(wood)).expect("a fence");
    }
    t.focus(hut.offset(7, 4));
    t.grab().await;
    t.check(t.w().map.fixture_at(hut.offset(5, 2)).is_some(), "the fence stands against the hut");
    t.shot("fences").await;

    // ---------------------------------------------------------- material patterns
    println!("\n# a material shows as a pattern, running on along the wall (DESIGN.md §6c)");
    let run_mid = slot.offset(4 + 1, 1);
    t.focus(run_mid);
    let img = t.grab().await;
    let z = t.app.cam.zoom;
    // A log course is a dark line a third of the way down; where two
    // joined walls meet it carries straight across.
    let line_y = 1.0 / 3.0;
    let darkest = |t: &T, img: &Image, fx: f32| {
        [-1.5f32, -0.75, 0.0, 0.75, 1.5]
            .iter()
            .map(|dy| px(img, at(t, run_mid, fx, line_y + dy / z)).iter().sum::<f32>())
            .fold(f32::INFINITY, f32::min)
    };
    // Between the courses, the wood itself.
    let body = px(&img, at(&t, run_mid, 0.5, 0.5)).iter().sum::<f32>();
    let (before, across) = (darkest(&t, &img, 0.96), darkest(&t, &img, 1.04));
    t.check(
        before < body - 0.1 && across < body - 0.1,
        format!("a log course runs across the joint ({before:.2} and {across:.2} against the wood's {body:.2})"),
    );
    // Zoomed right out, a pattern would be noise: it isn't drawn.
    t.app.cam.zoom = 6.0;
    let img = t.grab().await;
    let row: Vec<f32> = (0..8).map(|i| px(&img, at(&t, run_mid, 0.3 + i as f32 * 0.05, line_y)).iter().sum()).collect();
    let spread = row.iter().cloned().fold(f32::MIN, f32::max) - row.iter().cloned().fold(f32::MAX, f32::min);
    t.check(spread < 0.08, format!("no pattern zoomed out (brightness spread {spread:.2} across a wall)"));
    t.app.cam.zoom = 40.0;
    t.shot("patterns").await;

    // ---------------------------------------------------------- room labels
    println!("\n# rooms are labelled on the plan (DESIGN.md §6c)");
    let inside = west.offset(2, 0);
    t.app.sim.world.ensure_roles();
    let room = t.w().map.room_at(inside).expect("the hut's room");
    let label = format!("core:rooms.{}", room.id);
    t.app.cam.zoom = 40.0;
    t.focus(inside);
    t.grab().await;
    t.check(t.ui_rect(&label).is_some(), "the hut's room is labelled, zoomed in");
    t.app.cam.zoom = 8.0;
    t.grab().await;
    t.check(t.ui_rect(&label).is_none(), "no room labels zoomed out");
    t.app.cam.zoom = 40.0;
    t.app.cam.zoom = zoom;

    // ---------------------------------------------------------- the plan style
    println!("\n# every core building is drawn in the plan style (DESIGN.md §6c)");
    let gallery = [
        [
            "core:wall",
            "core:wall",
            "core:window",
            "core:door",
            "core:wall",
            "",
            "core:fence",
            "core:gate",
            "core:fence",
        ],
        ["core:bed", "", "core:table", "core:chair", "", "core:stove", "", "core:campfire", ""],
        ["core:floor", "core:floor", "", "core:pillar", "", "crafting:spot", "", "crafting:workbench", ""],
        ["core:stairs", "", "core:ladder", "", "", "", "", "", ""],
    ];
    let free = |w: &World, p: IVec| w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
    t.clear_dock().await;
    let corner = (2..40i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
        .find(|&o| (-1..7).all(|y| (-1..10).all(|x| free(t.w(), o.offset(x, y)))));
    let mut placed = 0;
    let mut portals = Vec::new();
    if let Some(o) = corner {
        for (row, ids) in gallery.iter().enumerate() {
            for (x, id) in ids.iter().enumerate().filter(|(_, id)| !id.is_empty()) {
                let Some(def) = t.w().defs.thing_id(id) else { continue };
                let stuff = t.w().defs.thing(def).build.as_ref().and_then(|b| b.stuff.as_ref()).map(|_| wood);
                let p = o.offset(x as i32, row as i32 * 2);
                let e = t.app.sim.world.spawn_fixture_of(def, p, false, stuff);
                // Stairs and ladders reach the level below, as when dug.
                if let Some(e) = e.filter(|_| t.w().defs.thing(def).portal.is_some()) {
                    t.app.sim.world.open_portal(e);
                    portals.push((e, IVec::at(p.x, p.y, p.z - 1)));
                }
                placed += e.is_some() as usize;
            }
        }
        t.app.sim.world.map.ensure_rooms();
        let zoom = t.app.cam.zoom;
        t.app.cam.zoom = 56.0;
        t.focus(o.offset(4, 2));
        t.grab().await;
        t.shot("plan_gallery").await;
        // One level down, the stair and the ladder's other ends, saying UP.
        t.app.cam.z -= 1;
        for _ in 0..12 {
            t.frame().await;
        }
        t.shot("plan_gallery_below").await;
        t.app.cam.z += 1;
        t.app.cam.zoom = zoom;
        let held = portals.iter().filter(|&&(e, below)| t.w().map.fixture_at(below) == Some(e)).count();
        t.check(held == 2, format!("the stair and the ladder reach the level below ({held} of 2)"));
    }
    let want = gallery.iter().flatten().filter(|id| !id.is_empty()).count();
    t.check(placed == want, format!("the gallery holds every core building ({placed} of {want})"));
    // ---------------------------------------------------------- house plans
    println!("\n# a house plan is placed from the build menu, turned with T (DESIGN.md §6c)");
    if let Some(plan) = t.w().defs.lookup("plan", "primitive:branch_hut") {
        t.clear_dock().await;
        t.click_tool("plan:primitive:branch_hut").await;
        t.check(t.app.tool == Tool::Plan(plan), "the build menu offers the branch hut");
        let facing = t.app.build_facing;
        t.key(KeyCode::T).await;
        t.check(t.app.build_facing == (facing + 1) & 3, "T turns the plan");
        let site = open_square(t.w(), home, 5).expect("open ground for a hut").offset(1, 1);
        t.focus(site);
        t.frame().await;
        t.drag(site, site).await;
        t.ticks(1);
        let defs = t.w().defs.clone();
        let pieces = defs.plans[plan as usize].placed(&defs, site, t.app.build_facing);
        let placed = pieces
            .iter()
            .filter(|pc| {
                let at = IVec::new(pc.at.0, pc.at.1);
                t.w().map.fixture_at(at).and_then(|e| t.w().thing(e)).is_some_and(|th| th.def == pc.thing)
            })
            .count();
        t.check(
            placed == pieces.len(),
            format!("every piece planned where the turned plan puts it ({placed} of {})", pieces.len()),
        );
        t.shot("house_plan").await;
        t.app.sim.push(Command::Cancel { a: site, b: site.offset(2, 2) });
        t.ticks(1);
        t.app.build_facing = facing;
        t.right_click((600.0, 500.0)).await;
    }

    // ---------------------------------------------------------- 0215 materials
    println!("\n# pick the material before you place it (0215)");
    t.clear_dock().await;
    t.check(t.ui_rect("core:stuff").is_none(), "no material row while nothing is being built");
    t.click_tool("build:core:wall").await;
    t.frame().await;
    t.check(t.ui_rect("core:stuff").is_some(), "the wall tool brings up the material row");
    t.check(t.ui_rect("core:dock.pill").is_some(), "in the placing pill, in the dock bar");
    t.check(t.ui_rect("core:stuff.core:wood").is_some(), "the wall tool offers wood");
    t.check(t.ui_rect("core:stuff.core:stone").is_some(), "and stone, whether or not there is any");
    let have_stone: u32 = t
        .w()
        .ecs
        .query::<&Thing>()
        .without::<&Blueprint>()
        .iter()
        .filter(|th| th.def == stone)
        .map(|th| th.count)
        .sum();
    t.check(
        have_stone > 0 || (t.ui_rect("core:stuff.core:stone").is_some() && t.ui_text().contains("\"none\"")),
        format!("a material you have none of says so ({have_stone} stone on the map)"),
    );
    let clicked = t.click_ui("core:stuff.core:stone").await;
    t.frame().await;
    t.check(clicked && t.app.stuff_for.contains(&(wall, stone)), "clicking stone picks it for the wall");
    let spot = (2..30i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
        .find(|&p| t.w().map.passable(p) && t.w().map.fixture_at(p).is_none() && t.w().map.item_at(p).is_none())
        .expect("a free cell");
    t.drag(spot, spot).await;
    t.ticks(1);
    let made = t.w().map.fixture_at(spot).and_then(|e| t.w().ecs.get::<&rim_sim::world::MadeOf>(e).ok().map(|m| m.0));
    t.check(made == Some(stone), format!("the blueprint is made of the chosen material ({made:?})"));
    t.shot("materials").await;

    // ---------------------------------------------------------- replace in place
    println!("\n# a wall planned over another is drawn hatched over it until they swap (DESIGN.md §6c)");
    let north = hut.offset(2, 0);
    let old = t.w().map.fixture_at(north);
    t.focus(north);
    // Blue less red, over the cell: the hatch is pale blue on wood.
    let blueness = |img: &Image, t: &T| {
        let mut sum = 0.0;
        for i in 1..8 {
            for j in 1..8 {
                let c = px(img, at(t, north, i as f32 / 8.0, j as f32 / 8.0));
                sum += c[2] - c[0];
            }
        }
        sum / 49.0
    };
    let img = t.grab().await;
    let before = blueness(&img, &t);
    t.app.sim.push(Command::Build { thing: wall, stuff: Some(stone), a: north, b: north, facing: 0 });
    t.ticks(1);
    let planned = old.and_then(|o| t.w().replacement_of(o)).is_some();
    t.check(planned && t.w().map.fixture_at(north) == old, "stone planned over the wood wall, which still stands");
    let img = t.grab().await;
    let after = blueness(&img, &t);
    t.check(after > before + 0.05, format!("the plan is drawn over the wall ({before:.2} -> {after:.2})"));
    t.shot("replace").await;
    t.app.sim.push(Command::Cancel { a: north, b: north });
    t.ticks(1);

    // A mod's sprite: wildlife_plus ships a salt lick drawn from the world
    // atlas, built beside the material test and seen up close.
    if let Some(lick) = t.w().defs.thing_id("wildlife_plus:salt_lick") {
        let cell = (2..30i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
            .find(|&p| t.w().map.passable(p) && t.w().map.fixture_at(p).is_none() && t.w().map.item_at(p).is_none())
            .expect("a free cell");
        let stuff = t.w().defs.materials("structural").first().copied();
        let built = t.app.sim.world.spawn_fixture_of(lick, cell, false, stuff).is_some();
        let sprites = t.w().defs.sprites.len();
        t.check(built && sprites > 0, format!("a mod's sprite def builds ({sprites} sprites packed)"));
        // And a glyph look beside it, from the same atlas.
        if let Some(marker) = t.w().defs.thing_id("wildlife_plus:trail_marker") {
            let next = (1..6i32)
                .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| cell.offset(dx, dy))))
                .find(|&p| t.w().map.passable(p) && t.w().map.fixture_at(p).is_none() && t.w().map.item_at(p).is_none())
                .expect("a free cell near the salt lick");
            let placed = t.app.sim.world.spawn_fixture_of(marker, next, false, stuff).is_some();
            t.check(placed, "a mod's glyph def builds");
            // The marker's own glyph, by the id its look holds.
            let id = t.w().defs.thing(marker).look_r.layers.iter().find_map(|l| match l.prim {
                rim_sim::look::Prim::Glyph { id, .. } => Some(id),
                _ => None,
            });
            let drawn = id.and_then(|i| t.app.world_atlas.glyph(i));
            t.check(drawn.is_some(), format!("and its glyph rasterised into the world atlas ({drawn:?})"));
        }
        t.focus(cell);
        t.app.cam.zoom = 48.0;
        t.frame().await;
        t.shot("sprite").await;
        t.app.cam.zoom = 28.0;
    }

    // Worksites (DESIGN.md §6b): a row of things part-way through being
    // built, mined, felled, taken down and broken, each drawn from its stage.
    println!("\n# worksites show how far along they are");
    {
        let row = (2..40i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
            .find(|&p| {
                (-1..7).all(|k| {
                    let q = p.offset(k, 0);
                    (-1..=1).all(|dy| {
                        let q = q.offset(0, dy);
                        t.w().map.passable(q) && t.w().map.fixture_at(q).is_none() && t.w().map.item_at(q).is_none()
                    })
                })
            })
            .expect("a free row");
        let d = t.w().defs.clone();
        let id = |s: &str| d.thing_id(s).unwrap_or_else(|| panic!("{s}"));
        let (wall, wood, oak, granite) = (id("wall"), id("wood"), id("tree_oak"), id("granite"));
        let decon = d.designations.iter().position(|x| x.targets == rim_sim::defs::Targets::Built).unwrap() as u16;
        let desig = |thing| d.thing(thing).harvest.iter().find(|h| h.destroy).unwrap().desig_r;
        let w = &mut t.app.sim.world;
        let mut at = |k: i32, def, plan: bool, work: Option<(u32, Option<u16>, Side)>| {
            let cell = row.offset(k, 0);
            let e = w.spawn_fixture_of(def, cell, plan, Some(wood)).expect("placed");
            if !plan && d.thing(def).build.is_some() {
                rim_sim::ai::complete_building(w, e);
            }
            if let Some((pct, designation, side)) = work {
                let total = w.ecs.get::<&Work>(e).map_or(100, |k| k.total);
                let k = Work { done: total * pct / 100, side, ..Work::new(total, designation) };
                w.ecs.insert_one(e, k).unwrap();
            }
            w.map.touch(cell);
            e
        };
        let rising = at(0, wall, true, Some((40, None, Side::West)));
        at(1, wall, true, Some((90, None, Side::West)));
        let rock = at(2, granite, false, Some((60, Some(desig(granite)), Side::West)));
        at(3, oak, false, Some((75, Some(desig(oak)), Side::East)));
        at(4, wall, false, Some((55, Some(decon), Side::North)));
        let hurt = at(5, wall, false, None);
        let max = w.stat(hurt, "hp").unwrap().round() as i32;
        w.ecs.get::<&mut Thing>(hurt).unwrap().hp = max * 3 / 8;
        w.mark_worksite(rock, row.offset(2, 0));
        t.check(
            (t.w().stage(rising), t.w().stage(rock), t.w().stage(hurt)) == (3, 4, 5),
            format!(
                "stages read work and lost hp ({}, {}, {})",
                t.w().stage(rising),
                t.w().stage(rock),
                t.w().stage(hurt)
            ),
        );
        t.focus(row.offset(3, 0));
        t.app.cam.zoom = 48.0;
        t.frame().await;
        let live: Vec<IVec> = (0..3).flat_map(|l| t.app.meshes.live(l).collect::<Vec<_>>()).collect();
        t.check(live.contains(&row.offset(2, 0)), "the site being worked is drawn live");
        t.check(!live.contains(&row), "and a plan nobody is building comes from the cache");
        t.shot("worksites").await;
        t.app.cam.zoom = 28.0;
    }

    // A thing wider than a cell (8cf4db07) draws once, from its anchor,
    // across its footprint, even where it crosses a chunk edge.
    if let Some(trough) = t.w().defs.thing_id("wildlife_plus:feeding_trough") {
        println!("\n# a two-cell thing draws once, across a chunk edge");
        let chunk = rim_sim::map::CHUNK;
        let open =
            |w: &World, p: IVec| w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
        let at = (2..60i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| home.offset(dx, dy))))
            .find(|&p| p.x.rem_euclid(chunk) == chunk - 1 && open(t.w(), p) && open(t.w(), p.offset(1, 0)))
            .expect("two free cells across a chunk edge");
        let stuff = t.w().defs.materials("structural").first().copied();
        let e = t.app.sim.world.spawn_fixture_of(trough, at, false, stuff).expect("placed");
        rim_sim::ai::complete_building(&mut t.app.sim.world, e);
        t.check(t.w().map.fixture_at(at.offset(1, 0)) == Some(e), "it is one thing in both cells");
        let z = 48.0;
        let (a, b) = (tally(&t.app, e, at, z), tally(&t.app, e, at.offset(1, 0), z));
        t.check(a > 0 && b == 0, format!("drawn from its anchor only ({a} shapes there, {b} beside it)"));
        // The view's left edge between its two cells: the anchor's chunk is
        // off screen, and the half in view must still be drawn from it.
        t.app.cam.zoom = z;
        t.app.cam.x = at.x as f32 + 0.5 + screen_width() / 2.0 / z;
        t.app.cam.y = at.y as f32 + 0.5;
        t.frame().await;
        let anchor_chunk = t.w().map.chunk_of(at);
        t.check(t.app.meshes.drawn(anchor_chunk), "its anchor's chunk, off screen, is drawn for the half in view");
        t.shot("two_cells").await;
        t.app.cam.zoom = 28.0;
    }

    // The view (5689930d): one level at a time. A room dug below with pits
    // over it and a way down; [ and ] change level, the ruler says who is
    // where and what is wrong there, and a level shown again comes from the
    // chunk cache.
    {
        println!("\n# one level at a time (5689930d)");
        let paused = t.app.paused;
        t.app.paused = true;
        let top = crate::bench::stacked(&mut t.app.sim, home.offset(-24, 6), 16);
        t.check(top.is_some(), "the stacked scene digs a way down, pits and a room below");
        if let Some(top) = top {
            let below: Vec<Entity> = t.w().colonists().filter(|&e| t.pawn(e).pos.z == -1).collect();
            // One of them badly hurt: core's alert names them, on their level.
            let hurt = below[0];
            let max = t.w().defs.creature(t.pawn(hurt).def).max_hp;
            let hp = t.pawn(hurt).hp;
            t.app.sim.world.ecs.get::<&mut Pawn>(hurt).unwrap().hp = max / 10;
            // The founder hurt too, up top: the alert names them first, and
            // still counts below for the one down there (4796c539).
            let founder_hp = t.pawn(founder).hp;
            t.app.sim.world.ecs.get::<&mut Pawn>(founder).unwrap().hp = max / 10;
            t.focus(top);
            t.app.cam.zoom = 20.0;
            // Long enough for the zoom to settle (a chunk drawn scaled from
            // another zoom is rebuilt once it has, whatever level is shown),
            // and for the alerts to be checked again: at most every quarter
            // second, and a frame here is a sixtieth.
            for _ in 0..30 {
                t.frame().await;
            }
            t.check(t.ui_rect("core:depth.level.-1").is_some(), "the ruler lists the level dug into");
            t.check(t.ui_rect("core:depth.count.-1").is_some(), "with the colonists on it");
            t.check(t.ui_rect("core:depth.alerts.-1").is_some(), "and the alert about one of them");
            t.check(t.ui_rect("core:depth.alerts.0").is_some(), "which counts up top too, for the founder");
            t.check(t.ui_rect("core:depth.level.-2").is_none(), "but not a level nobody has reached");
            t.shot("level_surface").await;
            t.key(KeyCode::LeftBracket).await;
            t.check(t.app.cam.z == -1, format!("[ goes down a level ({})", t.app.cam.z));
            t.shot("level_below").await;
            t.key(KeyCode::LeftBracket).await;
            t.check(t.app.cam.z == -1, "and no further than the levels reached");
            // Both levels are drawn now: up, down and up again rebuilds nothing.
            let mut rebuilt = 0;
            for k in [KeyCode::RightBracket, KeyCode::LeftBracket, KeyCode::RightBracket] {
                let pressed = crate::key_name(k).map(|n| vec![n.to_string()]).unwrap_or_default();
                t.input(RawInput { mouse: t.mouse, keys: vec![k], pressed, ..Default::default() }).await;
                rebuilt += t.app.meshes.rebuilt;
                for _ in 0..3 {
                    t.frame().await;
                    rebuilt += t.app.meshes.rebuilt;
                }
            }
            t.check(t.app.cam.z == 0, "] comes back up");
            t.check(rebuilt == 0, format!("changing between cached levels rebuilds no chunks ({rebuilt})"));
            t.click_ui("core:depth.level.-1").await;
            t.check(t.app.cam.z == -1, "a click on the ruler goes to that level");
            // Picking a colonist from the bar on another level goes to them.
            t.key(KeyCode::RightBracket).await;
            let name = t.pawn(hurt).name.clone();
            t.click_ui(&format!("core:colonists.{name}")).await;
            t.check(t.app.cam.z == -1, "selecting a colonist on another level shows their level");
            t.app.sim.world.ecs.get::<&mut Pawn>(hurt).unwrap().hp = hp;
            t.app.sim.world.ecs.get::<&mut Pawn>(founder).unwrap().hp = founder_hp;
            t.key(KeyCode::Escape).await;
            t.key(KeyCode::RightBracket).await;

            println!("\n# every level is lit by its own light (3124bd7b)");
            // Full daylight up top: below, none of it reaches.
            let field = |t: &T, id: &str| t.w().defs.lookup("field", id).unwrap() as usize;
            let (cloud, light) = (field(&t, "cloud"), field(&t, "light"));
            t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
            t.app.sim.world.fields.set_ambient(light, Some(100.0));
            t.app.light.adapt_now();
            t.focus(top);
            t.light_settles().await;
            let mean = |img: &Image| {
                let n = img.bytes.len() / 4;
                img.bytes.chunks(4).map(|c| c[0] as f32 + c[1] as f32 + c[2] as f32).sum::<f32>() / (n as f32 * 765.0)
            };
            let surface = mean(&t.grab().await);
            t.key(KeyCode::LeftBracket).await;
            t.app.light.adapt_now();
            t.light_settles().await;
            let below = mean(&t.grab().await);
            let sun = t.app.light.sun_visibility(top.x as f32 + 0.5, top.y as f32 + 0.5).unwrap_or(1.0);
            // The stacked scene has pits, and the sky comes down those
            // (7161f369): the level is darker than the surface, and sunless
            // where rock is over it.
            t.check(
                t.app.cam.z == -1 && below < surface * 0.75 && sun == 0.0,
                format!(
                    "less daylight reaches below ({below:.2} to the surface's {surface:.2}, sun {sun:.2} under rock)"
                ),
            );
            // A fire on the level below lights it, and only it.
            let campfire = defs.thing_id("campfire").unwrap();
            let spot =
                (-8..=8i32).flat_map(|dy| (-8..=8i32).map(move |dx| IVec::at(top.x + dx, top.y + dy, -1))).find(|&p| {
                    let m = &t.w().map;
                    m.inb(p) && m.passable(p) && m.fixture_at(p).is_none() && m.item_at(p).is_none()
                });
            let fire = spot.and_then(|p| t.app.sim.world.spawn_fixture(campfire, p, false));
            if let (Some(p), Some(_)) = (spot, fire) {
                t.light_settles().await;
                let glow = |t: &T| {
                    t.app.light.fire_at(p.x as f32 + 0.5, p.y as f32 + 0.5).map_or(0.0, |c| c.iter().sum::<f32>())
                };
                let lit_below = glow(&t);
                t.shot("light_below").await;
                t.key(KeyCode::RightBracket).await;
                t.light_settles().await;
                let lit_above = glow(&t);
                t.check(
                    lit_below > 0.3 && lit_above < 0.05,
                    format!("a fire below lights its own level ({lit_below:.2}), not the one above ({lit_above:.2})"),
                );
                // Both levels are kept: changing between them works nothing out again.
                let (bakes, runs) = (t.app.light.bakes, t.app.light.sun_runs);
                let mut repacked = false;
                for k in [KeyCode::LeftBracket, KeyCode::RightBracket, KeyCode::LeftBracket, KeyCode::RightBracket] {
                    t.key(k).await;
                    repacked |= t.app.light.passes.iter().any(|p| p.name == "occluders" && p.ran);
                    for _ in 0..3 {
                        t.frame().await;
                        repacked |= t.app.light.passes.iter().any(|p| p.name == "occluders" && p.ran);
                    }
                }
                t.check(
                    t.app.light.bakes == bakes && t.app.light.sun_runs == runs && !repacked,
                    format!(
                        "changing between cached levels bakes nothing ({} bakes, {} sun passes, repacked {repacked})",
                        t.app.light.bakes - bakes,
                        t.app.light.sun_runs - runs
                    ),
                );
            } else {
                t.check(false, "free floor below for a fire");
            }
            if let Some(e) = fire {
                t.app.sim.world.despawn_thing(e);
            }
            println!("\n# changing level fades, and the eye follows the sky in view (220a059e)");
            // From the lit surface to the dark level below and back, frame
            // by frame: no frame jumps.
            t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
            t.app.sim.world.fields.set_ambient(light, Some(100.0));
            t.app.light.adapt_now();
            t.light_settles().await;
            // Each level settled, then the change frame by frame: no frame
            // moves the brightness by a tenth of the brighter level's. A grab
            // draws two frames, so half its change is a frame's.
            let mut settled = Vec::new();
            let mut steps = Vec::new();
            // Down, up, and down then straight back up in the middle of the
            // fade: the mix on screen fades out, it doesn't jump.
            let (down, up) = (KeyCode::LeftBracket, KeyCode::RightBracket);
            for keys in [vec![down], vec![up], vec![down, up]] {
                t.app.light.adapt_now();
                t.light_settles().await;
                let mut last = mean(&t.grab().await);
                settled.push(last);
                let presses = keys.len();
                for (n, k) in keys.into_iter().enumerate() {
                    let pressed = crate::key_name(k).map(|n| vec![n.to_string()]).unwrap_or_default();
                    t.input(RawInput { mouse: t.mouse, keys: vec![k], pressed, ..Default::default() }).await;
                    // Another press to come: it lands two grabs in.
                    for _ in 0..if n + 1 < presses { 2 } else { 40 } {
                        let now = mean(&t.grab().await);
                        steps.push((now - last).abs() / 2.0);
                        last = now;
                    }
                }
            }
            let bright = settled.iter().copied().fold(0.0f32, f32::max).max(1e-3);
            let worst = steps.iter().copied().fold(0.0f32, f32::max) / bright;
            t.check(
                worst < 0.1,
                format!("changing level, no frame moves the brightness a tenth ({:.0}% at most)", worst * 100.0),
            );
            t.app.sim.world.fields.set_ambient(cloud, None);
            t.app.sim.world.fields.set_ambient(light, None);

            println!("\n# the sky down a shaft (7161f369)");
            // Three pits beside the stacked scene, one, two and three levels
            // deep, at noon under a clear sky.
            t.app.light.pin_sun = Some((90.0, 60.0));
            let air = defs.terrain.iter().position(|d| d.air).unwrap() as rim_sim::defs::DefId;
            // Every cell dug, with what it was, to put back after.
            let mut dug: Vec<(IVec, rim_sim::defs::DefId)> = Vec::new();
            let mut pit = |t: &mut T, o: IVec, deep: i32| {
                let mut set = |t: &mut T, q: IVec, to: rim_sim::defs::DefId| {
                    let was = t.w().map.terrain[t.w().map.idx(q)];
                    dug.push((q, was));
                    let cost = t.w().defs.terrain[to as usize].path_cost;
                    t.app.sim.world.map.set_terrain(q, to, cost);
                };
                for p in (0..3).flat_map(|y| (0..3).map(move |x| o.offset(x, y))) {
                    if !t.w().map.inb(IVec::at(p.x, p.y, -deep)) {
                        continue;
                    }
                    for e in
                        [t.w().map.fixture_at(p), t.w().map.item_at(p), t.w().map.floor_at(p)].into_iter().flatten()
                    {
                        t.app.sim.world.despawn_thing(e);
                    }
                    for z in 0..deep {
                        set(t, IVec::at(p.x, p.y, -z), air);
                    }
                    let floor = IVec::at(p.x, p.y, -deep);
                    if let Some(leaves) = t.w().solid_at(floor).and_then(|r| r.leaves_r) {
                        set(t, floor, leaves);
                    }
                }
            };
            let pits: Vec<(IVec, i32)> = (1..=3).map(|deep| (top.offset(-6 - 5 * deep, -12), deep)).collect();
            if t.w().map.levels().start() <= &-3 {
                for &(o, deep) in &pits {
                    pit(&mut t, o, deep);
                }
                t.app.sim.world.map.ensure_rooms();
                t.focus(pits[1].0);
                let mut seen = Vec::new();
                for &(o, deep) in &pits {
                    t.app.cam.z = -deep;
                    t.app.light.adapt_now();
                    t.light_settles().await;
                    let (open, sun) = t.app.light.sky_at(o.x as f32 + 1.5, o.y as f32 + 1.5).unwrap_or((1.0, 1.0));
                    seen.push((deep, open, sun));
                    if deep == 1 {
                        t.shot("pit_at_noon").await;
                    }
                }
                let sky = |(_, open, sun): (i32, f32, f32)| open * 0.4 + sun * 0.6;
                let falls = seen.windows(2).all(|w| sky(w[1]) < sky(w[0]));
                t.check(falls, format!("the sky falls with depth down a shaft: {seen:.2?}"));
                t.check(
                    seen[0].2 > 0.5,
                    format!("a pit a level deep is sunlit on its floor at noon ({:.2})", seen[0].2),
                );
                // The stacked scene's room below, beside its pits: no sky.
                t.app.cam.z = -1;
                t.light_settles().await;
                let cellar = (-6..=6i32)
                    .flat_map(|dy| (-6..=6i32).map(move |dx| IVec::at(top.x + dx, top.y + dy, -1)))
                    .find(|&p| {
                        let m = &t.w().map;
                        m.passable(p) && !crate::occluders::open_to_sky(m, p.x, p.y, -1)
                    });
                let dark = cellar.and_then(|p| t.app.light.sky_at(p.x as f32 + 0.5, p.y as f32 + 0.5));
                t.check(
                    dark.is_some_and(|(open, sun)| open == 0.0 && sun == 0.0),
                    format!("a cellar beside it sees no sky ({dark:?})"),
                );
            } else {
                t.check(false, "three levels to dig a shaft down");
            }
            for (q, was) in dug.into_iter().rev() {
                let cost = t.w().defs.terrain[was as usize].path_cost;
                t.app.sim.world.map.set_terrain(q, was, cost);
            }
            t.app.sim.world.map.ensure_rooms();
            t.app.light.pin_sun = None;
            t.app.cam.z = 0;
            // Back on the surface once its fade is over, and the eye with it.
            t.app.light.adapt_now();
            t.light_settles().await;
            t.app.sim.world.fields.set_ambient(cloud, None);
            t.app.sim.world.fields.set_ambient(light, None);
            if t.app.cam.z != 0 {
                t.key(KeyCode::RightBracket).await;
            }
            t.app.light.adapt_now();
        }
        t.app.paused = paused;
    }

    // Speech (DESIGN.md §11): a bubble sits on its speaker, even on the
    // frame a wheel zoom lands, when the UI was laid out for the old zoom.
    {
        println!("\n# a speech bubble sits on its speaker");
        t.app.sim.world.recent_events.clear();
        t.app.sim.world.say(founder, "Over here!", 600, 5);
        t.focus(t.pawn(founder).pos);
        t.app.cam.zoom = 40.0;
        t.frame().await;
        t.frame().await;
        let at = t.pawn_screen(founder);
        // A wheel step about a point away from the pawn moves it on screen.
        t.input(RawInput { mouse: (at.0 + 200.0, at.1 + 150.0), wheel: 1.0, ..Default::default() }).await;
        let off = bubble_offset(&t, founder);
        t.check(
            off.is_some_and(|d| d.abs() < 2.0),
            format!("the bubble is centred on its speaker the frame a zoom lands ({off:?} px off)"),
        );
        t.shot("speech").await;
        t.app.cam.zoom = 28.0;
    }

    // Render scale: the world at half the pixels, the UI still full. The
    // same frame at both scales must look alike: a flipped or darkened
    // world (translucent plans are on screen) would not.
    let native = screen_width() * screen_dpi_scale();
    // Fog: a translucent veil over the whole world.
    let fog = t.w().defs.lookup("field", "fog").map(|f| f as usize);
    if let Some(f) = fog {
        t.app.sim.world.fields.set_ambient(f, Some(90.0));
        t.ticks(2);
    }
    let full = t.grab().await;
    crate::apply_ui(&mut t.app, rim_ui::view::UiAction::RenderScale(0.5));
    t.frame().await;
    let scaled = t.grab().await;
    let diff = block_diff(&full, &scaled);
    t.check(
        diff < 4.0,
        format!("half render scale looks like full, only softer (mean block difference {diff:.1} of 255)"),
    );
    if let Some(f) = fog {
        t.app.sim.world.fields.set_ambient(f, None);
    }
    let half = t.app.world_target.as_ref().map(|rt| rt.texture.width());
    t.check(
        half.is_some_and(|w| (w - native / 2.0).abs() <= 1.0),
        format!("half render scale draws the world at half the width ({half:?} of {native})"),
    );
    t.shot("render_scale_50").await;
    crate::apply_ui(&mut t.app, rim_ui::view::UiAction::RenderScale(1.0));
    t.frame().await;
    let full_w = t.app.world_target.as_ref().map(|rt| rt.texture.width());
    t.check(
        full_w.is_some_and(|w| (w - native).abs() <= 1.0),
        format!("full render scale draws the world at the screen's width ({full_w:?} of {native})"),
    );
    t.click_tool("cancel").await;
    t.drag(spot, spot).await;
    t.ticks(1);
    t.click_tool("build:core:wall").await;
    t.frame().await;
    t.check(t.app.stuff_for.contains(&(wall, stone)), "the choice is remembered for the wall");
    t.click_ui("core:stuff.core:wood").await;
    t.key(KeyCode::Escape).await;

    // ---------------------------------------------------------- 0047 orders
    println!("\n# select, draft, move, attack (0047)");
    t.act(Action::Speed(1));
    t.focus(t.pawn(founder).pos);
    t.clear_dock().await;
    t.key(KeyCode::Escape).await;
    t.check(t.app.selected.is_none(), "escape clears the selection");
    t.frame().await;
    let at = t.pawn_screen(founder);
    t.click(at).await;
    t.check(t.app.selected == Some(founder), "clicking a colonist selects them again");
    t.key(KeyCode::R).await;
    t.ticks(1);
    t.check(t.pawn(founder).drafted, "R drafts the selected colonist");

    let here = t.pawn(founder).pos;
    // Any open cell a few steps off that the founder can reach, ring by
    // ring: whatever the map put around them. A scene above may have stood
    // a wall where they were; they walk out of it, so reach is judged from
    // the open ground beside them.
    t.app.sim.world.map.ensure_regions();
    let region = std::iter::once(here)
        .chain(rim_sim::map::NEIGHBORS8.iter().map(|&(dx, dy)| here.offset(dx, dy)))
        .find(|p| t.w().map.passable(*p))
        .map_or(0, |p| t.w().map.region_at(p));
    let dest = (3..30)
        .flat_map(|r| {
            (-r..=r).flat_map(move |d| [here.offset(r, d), here.offset(-r, d), here.offset(d, r), here.offset(d, -r)])
        })
        .find(|p| t.w().map.passable(*p) && t.w().map.region_at(*p) == region)
        .expect("somewhere to walk");
    let d = t.screen(dest);
    t.right_click(d).await;
    t.ticks(1); // commands apply on the next tick
    t.check(matches!(t.pawn(founder).job, Job::MoveTo { to } if to == dest), "right-click orders a move");
    t.ticks(600);
    t.check(t.pawn(founder).pos == dest, "the colonist walks there");

    let human = defs.creature_id("human").unwrap();
    let raider_at = (2..6)
        .flat_map(|r| [dest.offset(r, 0), dest.offset(-r, 0), dest.offset(0, r), dest.offset(0, -r)])
        .find(|p| t.w().map.passable(*p) && t.w().map.region_at(*p) == t.w().map.region_at(dest))
        .expect("room for a raider");
    let raider = t.app.sim.world.spawn_pawn(human, Faction::Hostile, raider_at, Some("Testrunner".into()));
    t.frame().await;
    let rs = t.pawn_screen(raider);
    t.right_click(rs).await;
    t.ticks(1);
    t.check(
        matches!(t.pawn(founder).job, Job::Attack { target, .. } if target == raider),
        "right-click on an enemy orders an attack",
    );
    t.ticks(240);
    // Hurt, dead, or already gone (retreated off the map).
    let hurt = t.w().ecs.get::<&Pawn>(raider).map_or(true, |p| p.hp < 100 || p.dead);
    t.check(hurt, "the attack lands");
    t.focus(t.pawn(founder).pos);
    t.shot("fight").await;
    if let Ok(mut p) = t.app.sim.world.ecs.get::<&mut Pawn>(raider) {
        p.dead = true;
    }
    t.key(KeyCode::R).await;
    t.ticks(2);
    t.check(!t.pawn(founder).drafted, "R again undrafts");

    // ---------------------------------------------------------- 0048 HUD
    println!("\n# messages, colonist bar, clock, speed (0048)");
    t.key(KeyCode::Space).await;
    t.check(t.app.paused, "space pauses");
    t.check(t.ui_text().contains("\"paused\""), "the clock says paused");
    // Paused, orders still land: planning while paused is how the game is
    // played. Nothing ticks here; the frames alone must apply them.
    let tick = t.w().tick;
    let (marked, planned) = (t.count::<(&Thing, &Designated)>(), t.count::<&Blueprint>());
    let oak = defs.thing_id("tree_oak").unwrap();
    let tree = t
        .w()
        .ecs
        .query::<(&Thing, Option<&Designated>)>()
        .iter()
        .filter(|(th, d)| th.def == oak && d.is_none())
        .map(|(th, _)| th.pos)
        .min_by_key(|p| (p.octile(home), p.x, p.y));
    t.click_tool("designate:core:chop").await;
    // Centred first, so the cell isn't under a panel whatever the map.
    if let Some(p) = tree {
        t.focus(p);
        t.drag(p, p).await;
    }
    let spot = open_square(t.w(), home.offset(10, 10), 2).unwrap_or(home.offset(10, 10));
    t.click_tool("build:core:wall").await;
    t.focus(spot);
    t.drag(spot, spot).await;
    t.frame().await;
    t.check(t.w().tick == tick, "paused: no time passed");
    t.check(tree.is_none() || t.count::<(&Thing, &Designated)>() > marked, "paused, a designation shows at once");
    t.check(t.count::<&Blueprint>() > planned, "paused, a plan shows at once");
    t.shot("paused_orders").await;
    t.key(KeyCode::Escape).await;
    t.key(KeyCode::Space).await;
    t.check(!t.app.paused, "space resumes");
    for (k, want) in [(KeyCode::Key1, 1), (KeyCode::Key2, 3), (KeyCode::Key3, 6)] {
        t.key(k).await;
        t.check(t.app.speed == want, format!("speed key sets {want}x"));
    }
    t.frame().await;
    let snap = t.ui_text();
    t.check(snap.contains(&format!("\"Day {}\"", t.w().day() + 1)), "top bar shows the day");
    let hh = t.w().hour() as u32;
    t.check(snap.contains(&format!("\"{hh:02}:")), "top bar shows the hour");
    t.check(snap.contains("\"6×\""), "top bar shows the speed");

    let kinds = [MsgKind::Info, MsgKind::Good, MsgKind::Threat, MsgKind::Bad];
    for k in kinds {
        t.app.sim.world.message(format!("autotest {k:?} message"), k);
    }
    t.settle().await;
    t.check(t.ui_text().contains("autotest Threat message"), "messages show as toasts");
    let th = &t.app.ui.theme;
    let colours: Vec<[f32; 4]> = ["text", "good", "threat", "bad"].iter().map(|c| th.color[*c]).collect();
    let distinct = (0..4).all(|i| (0..4).all(|j| i == j || colours[i] != colours[j]));
    t.check(distinct, "each message kind has its own colour token");

    t.key(KeyCode::Escape).await;
    t.key(KeyCode::Escape).await;
    let name = t.pawn(founder).name.clone();
    t.click_ui(&format!("core:colonists.{name}")).await;
    t.check(t.app.selected == Some(founder), "clicking the colonist bar selects that colonist");
    let cols: Vec<Entity> = t.w().colonists().collect();
    t.key(KeyCode::Tab).await;
    t.check(t.app.selected.is_some_and(|s| cols.contains(&s)), "tab cycles colonists");
    t.shot("messages").await;

    // ---------------------------------------------------------- ecd54de8 stockpiles
    println!("\n# stockpiles (ecd54de8)");
    let spot = (4..30i32)
        .flat_map(|r| [home.offset(r, r), home.offset(-r, r), home.offset(r, -r), home.offset(-r, -r)])
        .find(|&o| {
            (0..4).all(|x| {
                (0..3)
                    .all(|y| t.w().map.passable(o.offset(x, y)) && t.w().zones.at(&t.w().map, o.offset(x, y)).is_none())
            })
        })
        .expect("open ground for a stockpile");
    t.click_tool("stockpile").await;
    t.check(t.app.tool == Tool::Stockpile, "clicking Stockpile selects the stockpile tool");
    // On screen, clear of the HUD: the camera is wherever the last step left it.
    t.focus(spot.offset(1, 1));
    t.drag(spot, spot.offset(3, 2)).await;
    t.ticks(1);
    let zone = t.w().zones.at(&t.w().map, spot).map(|z| z.id);
    t.check(zone.is_some(), "dragging paints a stockpile");
    // A drag touching it grows the same zone rather than making another.
    t.drag(spot.offset(3, 0), spot.offset(5, 0)).await;
    t.ticks(1);
    t.check(
        t.w().zones.list.len() == 1 && t.w().zones.at(&t.w().map, spot.offset(5, 0)).map(|z| z.id) == zone,
        "a touching drag extends it",
    );
    // Zones in violet (11080b20): a drag shows the cells it adds, counted,
    // and they are the cells the zone gains.
    let count = |t: &T| t.w().zones.cells.iter().filter(|&&c| Some(c) == zone).count();
    let before = count(&t);
    let (a, b) = (spot.offset(0, 2), spot.offset(1, 6));
    t.input(RawInput { mouse: t.screen(a), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: t.screen(b), ..Default::default() }).await;
    let chip = crate::overlay::drag_hint(&t.app);
    t.shot("chalk-zone-paint").await;
    t.input(RawInput { mouse: t.screen(b), left_released: true, ..Default::default() }).await;
    t.ticks(1);
    let gained = count(&t) - before;
    t.check(
        chip.as_deref() == Some("Stockpile · +8") && gained == 8,
        format!("a stockpile drag counts the cells it adds, and adds them ({chip:?}, +{gained})"),
    );
    t.click_tool("clear_zone").await;
    let (a, b) = (spot.offset(0, 5), spot.offset(1, 6));
    t.input(RawInput { mouse: t.screen(a), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: t.screen(b), ..Default::default() }).await;
    let hatched = t.app.zone_preview.as_ref().map(|zp| zp.cells.clone()).unwrap_or_default();
    t.shot("chalk-zone-clear").await;
    t.input(RawInput { mouse: t.screen(b), left_released: true, ..Default::default() }).await;
    t.ticks(1);
    let freed: Vec<usize> = hatched.iter().copied().filter(|&i| t.w().zones.cells[i] == 0).collect();
    t.check(
        hatched.len() == 4 && freed == hatched,
        format!("a clear-zone drag hatches the cells it frees ({} hatched, {} freed)", hatched.len(), freed.len()),
    );
    t.clear_dock().await;
    t.app.tool = Tool::Select;
    t.click(t.screen(spot.offset(1, 1))).await;
    t.frame().await;
    let edge = zone.map(|z| draw::zone_edge(&t.app, z));
    t.check(
        t.app.selected_zone == zone && edge.is_some_and(|(c, _, keyed)| c == t.app.palette.chalk && keyed),
        format!("a selected stockpile's edge is chalk on a keyline ({edge:?})"),
    );
    t.shot("chalk-zone").await;
    t.key(KeyCode::Escape).await;
    t.check(t.app.selected_zone.is_none(), "Escape lets go of the stockpile");
    t.clear_dock().await;
    t.focus(spot);
    t.shot("stockpile").await;
    t.key(KeyCode::Z).await;
    t.click_ui("core:zones.open").await;
    t.check(
        t.app.ui.find("core:zones.1.core:wood").is_some(),
        "Z, then the list, opens the stockpiles panel, a toggle per item",
    );
    t.click_ui("core:zones.1.core:wood").await;
    t.ticks(1);
    let wood = defs.thing_id("wood").unwrap();
    t.check(
        zone.and_then(|z| t.w().zones.get(z)).is_some_and(|z| !z.takes(wood)),
        "a toggle stops the zone taking wood",
    );
    t.click_ui("core:zones.open").await;
    t.key(KeyCode::Z).await;
    t.frame().await;
    t.check(
        !t.app.ui.is_open("core:zones") && t.ui_rect("core:dock.palette.zones").is_none(),
        "the list button closes the panel, and Z the palette",
    );

    // ---------------------------------------------------------- 0cb48faf stances
    println!("\n# stances (0cb48faf)");
    t.key(KeyCode::P).await;
    t.check(t.app.ui.find("core:work.stance.core:siege").is_some(), "P opens the Work Board with a stance bar");
    // One colonist on Auto opens to their plan, more to the board; the
    // stance step reads the board. (Whether a wanderer has joined by now
    // is the map's doing.)
    let alone = t.w().colonists().count() == 1;
    t.check(
        t.app.ui.find("core:work.show_board").is_some() == alone,
        format!("one colonist on Auto opens to their plan, more to the board (alone: {alone})"),
    );
    t.shot("work_plan").await;
    if alone {
        t.click_ui("core:work.show_board").await;
    }
    t.click_ui("core:work.stance.core:siege").await;
    t.ticks(1);
    t.check(t.w().stance == defs.lookup("stance", "core:siege"), "a stance button puts the colony in it");
    for _ in 0..20 {
        t.frame().await;
    }
    // Siege sets Hunt to never, wherever Auto's plan had it.
    let hunt = {
        let d = &t.w().defs;
        d.work_order.iter().position(|&w| d.work_types[w as usize].id == "core:hunt").unwrap() + 1
    };
    let cell = t.app.ui.grid_cell("core:work.grid", 1, hunt).map(|c| c.text);
    t.check(
        cell.as_deref().is_some_and(|c| c.ends_with("→–") && c.len() > "→–".len()),
        format!("a cell a stance moves reads where it was and where it is ({cell:?})"),
    );
    t.shot("stance_siege").await;
    t.click_ui("core:work.stance.core:normal").await;
    t.ticks(1);
    t.check(t.w().stance == defs.lookup("stance", "core:normal"), "and back");
    t.key(KeyCode::P).await;

    // ---------------------------------------------------------- 0049 profiler
    println!("\n# profiler (0049)");
    t.key(KeyCode::F3).await;
    t.check(t.app.show_profiler, "F3 opens the profiler");
    t.ticks(1200);
    for _ in 0..20 {
        t.frame().await;
    }
    let names: Vec<String> = t.app.sim.profile.entries.iter().map(|e| e.0.clone()).collect();
    for sys in ["tick", "pawns", "needs", "regions", "rooms", "fields", "wealth"] {
        t.check(names.iter().any(|n| n == sys), format!("profiler times system '{sys}'"));
    }
    t.check(names.iter().any(|n| n == "mod:core"), "profiler times core's scripts");
    t.check(t.app.ui.find("core:profiler.panel").is_some(), "the profiler panel is drawn");
    t.check(t.ui_text().contains("ui:core"), "and it times core's UI code too");
    t.shot("profiler").await;
    t.key(KeyCode::F3).await;

    // ---------------------------------------------------------- 0160 field overlay
    println!("\n# field overlay (0160)");
    let shown: Vec<usize> = (0..defs.fields.len()).filter(|&i| defs.fields[i].overlay).collect();
    let n = shown.len();
    t.check(n >= 2, format!("core defines field layers with overlays ({n})"));
    t.check(t.app.overlay.is_none(), "overlay starts off");
    let fire = defs.thing_id("campfire").unwrap();
    // Outdoors: inside an enclosed room the temperature is the room's own
    // value, not the outdoor air plus the fire (seed 37 put one in the hut).
    let spot = (0..40)
        .filter_map(|r| open_square(t.w(), site.offset(r, -r), 1))
        .find(|&p| !t.w().map.indoors(p))
        .expect("open ground outdoors for a fire");
    let _ = t.app.sim.world.spawn_fixture(fire, spot, false);
    t.ticks(1);
    for &i in &shown {
        t.key(KeyCode::O).await;
        t.check(t.app.overlay == Some(i), format!("O shows the '{}' overlay", defs.fields[i].label));
        t.focus(spot);
        t.shot(&format!("overlay_{}", defs.fields[i].id)).await;
    }
    t.key(KeyCode::O).await;
    t.check(t.app.overlay.is_none() && t.app.storage_overlay, "after the fields, O shows the storage overlay");
    t.shot("overlay_storage").await;
    t.key(KeyCode::O).await;
    t.check(t.app.overlay.is_none() && !t.app.storage_overlay, "O again turns the overlay off");
    let temp = defs.lookup("field", "temperature").unwrap() as usize;
    let near = t.w().fields.value(&defs, &t.w().map, temp, spot);
    let outside = t.w().fields.ambient(temp);
    t.check(near > outside, format!("the campfire warms its cell ({near:.1}° vs {outside:.1}° outside)"));
    t.check(t.ui_text().contains("°C outside"), "top bar shows the outdoor temperature");

    // Hover readout over the world.
    let sp = t.screen(spot);
    t.input(RawInput { mouse: sp, ..Default::default() }).await;
    t.frame().await;
    t.check(t.app.ui.find("core:hover").is_some(), "hovering the world shows the readout");
    t.check(t.ui_text().contains("campfire"), "and names what's there");

    // ---------------------------------------------------------- 0173 devtools
    println!("\n# devtools (0173)");
    t.key(KeyCode::F12).await;
    t.frame().await;
    t.check(t.app.ui.find("core:devtools.panel").is_some(), "F12 opens devtools");
    if let Some(r) = t.ui_rect("core:dock.orders") {
        t.input(RawInput { mouse: (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0), ..Default::default() }).await;
        t.frame().await;
    }
    let inspect = t.app.ui.info.inspect.clone();
    t.check(
        inspect.as_ref().is_some_and(|i| i.owner == "core" && i.path.starts_with("docked")),
        format!("pointing at the toolbar inspects it ({:?})", inspect.map(|i| (i.id, i.owner))),
    );
    t.shot("devtools").await;
    t.click_ui("core:devtools.outlines").await;
    t.check(t.app.ui.info.outlines, "the outlines toggle turns on layout outlines");
    t.shot("outlines").await;
    t.click_ui("core:devtools.outlines").await;
    t.click_ui("core:devtools.gallery").await;
    t.frame().await;
    t.check(t.app.ui.find("core:gallery.panel").is_some(), "the kit gallery opens");
    t.shot("gallery").await;
    t.click_ui("core:devtools.gallery").await;
    t.key(KeyCode::F12).await;
    t.frame().await;
    t.check(t.app.ui.find("core:devtools.panel").is_none(), "F12 closes devtools");

    // ---------------------------------------------------------- render cost
    println!("\n# render cost");
    let zoom0 = t.app.cam.zoom;
    t.act(Action::Zoom(0.01, 800.0, 480.0)); // all the way out: the most cells
    t.frame().await;
    // CPU cost of issuing the frame's drawing (not the GPU, not vsync).
    let t0 = std::time::Instant::now();
    render(&mut t.app);
    let frame_ms = t0.elapsed().as_secs_f64() * 1e3;
    t.frame().await;
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let calls = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    println!("zoomed out ({:.1} px/cell): {calls} draw calls, render {frame_ms:.2} ms (CPU)", t.app.cam.zoom);
    t.check(calls > 0, "the renderer's draw calls can be counted");
    t.act(Action::Zoom(zoom0 / t.app.cam.zoom, 800.0, 480.0));
    t.frame().await;
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let calls = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    println!("normal zoom, HUD open: {calls} draw calls");

    // ---------------------------------------------------------- UI budget, live
    let (b, l, p) = (t.app.ui.info.build_us, t.app.ui.info.layout_us, t.app.ui.info.paint_us);
    println!(
        "\nui: build {b:.0} µs (when rebuilt) · layout {l:.0} µs · paint {p:.0} µs · {} nodes · {} rebuilds",
        t.app.ui.info.nodes, t.app.ui.builds
    );

    // ---------------------------------------------------------- weather
    println!("\n# weather (0184, 0194, 0195)");
    let queue: Vec<String> = match t.w().data.get("weather:forecast") {
        Some(Data::Table(q)) => q
            .values()
            .map(|e| match e.get("id") {
                Some(Data::Str(s)) => s.clone(),
                _ => String::new(),
            })
            .collect(),
        _ => Vec::new(),
    };
    t.check(queue.len() == 4, format!("the weather plugin keeps a forecast ({queue:?})"));
    t.check(t.app.ui.find("weather:readout").is_some(), "the top bar shows the weather");
    t.click_ui("weather:readout").await;
    t.frame().await;
    t.check(t.app.ui.find("weather:forecast.panel").is_some(), "clicking it opens the forecast");
    let rows = (2..=4).filter(|i| t.app.ui.find(&format!("weather:forecast.{i}")).is_some()).count();
    t.check(rows == queue.len() - 1, format!("the forecast lists what comes next ({rows} rows)"));
    let text = t.ui_text();
    t.check(text.contains("mean") && text.contains("day"), "and the temperature's breakdown");
    t.shot("forecast").await;
    t.click_ui("weather:readout").await;
    t.frame().await;
    t.check(t.app.ui.find("weather:forecast.panel").is_none(), "clicking again closes it");

    // Weather devtools: force a type through a command, skip ahead.
    t.key(KeyCode::F12).await;
    t.frame().await;
    t.check(t.app.ui.find("weather:devtools.panel").is_some(), "F12 shows the weather devtools");
    t.click_ui("weather:devtools.force.weather:storm").await;
    t.ticks(2);
    let head = match t.w().data.get("weather:forecast") {
        Some(Data::Table(q)) => q.values().next().and_then(|e| e.get("id").cloned()),
        _ => None,
    };
    t.check(head == Some(Data::Str("weather:storm".into())), format!("forcing a storm from devtools works ({head:?})"));
    // A day of the forced storm with nobody told to shelter would kill
    // colonists on some machines and not others: the day passes mild.
    let (temp, rain) = (
        defs.lookup("field", "temperature").unwrap() as usize,
        defs.lookup("field", "precipitation").unwrap() as usize,
    );
    t.app.sim.world.fields.set_ambient(temp, Some(16.0));
    t.app.sim.world.fields.set_ambient(rain, Some(0.0));
    let before = t.w().tick;
    t.click_ui("weather:devtools.advance.24").await;
    let skipped = t.w().tick - before;
    t.app.sim.world.fields.set_ambient(temp, None);
    t.app.sim.world.fields.set_ambient(rain, None);
    t.keep_well();
    t.check(skipped >= rim_sim::TICKS_PER_DAY, format!("+1 day runs the sim a day forward ({skipped} ticks)"));
    t.frame().await;
    t.shot("weather_devtools").await;
    t.key(KeyCode::F12).await;
    t.frame().await;

    // Pin the channels to see each kind of weather over the hut.
    let field = |t: &T, id: &str| t.w().defs.lookup("field", id).unwrap() as usize;
    let pins = ["precipitation", "temperature", "wind", "wind_dir", "cloud", "fog"];
    let set = |t: &mut T, v: [f64; 6]| {
        for (id, v) in pins.iter().zip(v) {
            let f = field(t, id);
            t.app.sim.world.fields.set_ambient(f, Some(v));
        }
    };
    // A closed 5x5 hut, so we can see that nothing falls indoors.
    let wall = defs.thing_id("wall").unwrap();
    let hut = open_square(t.w(), site.offset(8, 0), 5).map(|o| {
        let c = o.offset(2, 2);
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                if dx.abs() == 2 || dy.abs() == 2 {
                    let _ = t.app.sim.world.spawn_fixture(wall, c.offset(dx, dy), false);
                }
            }
        }
        c
    });
    t.ticks(2);
    t.check(hut.is_some_and(|c| t.w().map.indoors(c)), "a walled hut counts as indoors");
    t.focus(hut.unwrap_or(site));
    set(&mut t, [4.0, 12.0, 6.0, 30.0, 95.0, 0.0]);
    t.ticks(40);
    for _ in 0..3 {
        t.frame().await;
    }
    t.check(t.app.sky.particles() > 300, format!("rain falls ({} drops)", t.app.sky.particles()));
    println!(
        "weather visuals: {:.0} µs for {} particles, lighting {:.0} µs (CPU)",
        t.app.render_us.weather,
        t.app.sky.particles(),
        t.app.render_us.light
    );
    if hut.is_some() {
        t.check(t.app.sky.hidden > 0, format!("but not inside the hut ({} hidden)", t.app.sky.hidden));
    }
    t.shot("rain").await;
    set(&mut t, [3.0, -6.0, 4.0, 150.0, 90.0, 0.0]);
    t.ticks(40);
    for _ in 0..90 {
        t.frame().await;
    }
    t.shot("snow").await;
    set(&mut t, [0.0, 6.0, 0.5, 0.0, 60.0, 85.0]);
    t.ticks(40);
    t.frame().await;
    t.check(t.app.sky.particles() < 60, "fog: nothing falls");
    t.shot("fog").await;

    // Night, to see lighting and the labels on top of it.
    for id in pins {
        let f = field(&t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }
    while !(22.0..23.0).contains(&t.w().hour()) {
        t.ticks(100);
        t.keep_well();
    }
    t.focus(site.offset(3, 3));
    t.shot("night").await;
    // The grid darkens with the ground, and still shows (553bfb19).
    if let Some(o) = open_square(t.w(), site, 6) {
        let was = t.app.paused;
        t.app.paused = true;
        // No rain: falling streaks would move pixels between the shots.
        let rain = t.w().defs.lookup("field", "precipitation").unwrap() as usize;
        t.app.sim.world.fields.set_ambient(rain, Some(0.0));
        t.focus(o.offset(3, 3));
        let [rest, _, plan] = grid_shots(&mut t, o, defs.thing_id("wall").expect("walls")).await;
        let corner = t.app.cam.to_screen(o.x as f32 + 4.0, o.y as f32 + 4.0);
        let d = patch_diff(&rest, &plan, corner, 4.0);
        t.check(d > 0.3, format!("at night the grid still shows near the pointer ({d:.2})"));
        t.focus(site.offset(3, 3));
        t.app.paused = was;
        t.app.sim.world.fields.set_ambient(rain, None);
    }

    // A storm at night, mid-flash.
    set(&mut t, [8.0, 9.0, 16.0, 20.0, 100.0, 0.0]);
    t.ticks(40);
    t.frame().await;
    for _ in 0..3 {
        t.frame().await;
    }
    println!(
        "storm visuals: {:.0} µs for {} particles, lighting {:.0} µs (CPU)",
        t.app.render_us.weather,
        t.app.sky.particles(),
        t.app.render_us.light
    );
    // A flash casts shadows from where the bolt is, worked out as it
    // strikes and again as it fades: twice at most, however long it lasts.
    let runs = t.app.light.sun_runs;
    t.app.sky.strike();
    t.frame().await;
    // One frame: the flash is lit from its bolt from the frame it strikes.
    let lit = t.app.light.lit_by_flash()
        && t.app.light.sun_image().is_some_and(|img| {
            let px = |k: usize| img.bytes[k * 4];
            let n = img.bytes.len() / 4;
            (0..n).any(|k| px(k) > 230) && (0..n).any(|k| px(k) < 25)
        });
    t.check(lit, "at night a flash lights the ground from where the bolt is, and walls shade it");
    t.shot("storm").await;
    // The rain stops, so no second flash strikes while this one fades,
    // however slowly the frames come.
    t.app.sim.world.fields.set_ambient(field(&t, "precipitation"), Some(0.0));
    for _ in 0..120 {
        if t.app.sky.flash().strength == 0.0 && !t.app.light.lit_by_flash() {
            break;
        }
        t.frame().await;
    }
    let runs = t.app.light.sun_runs - runs;
    t.check(
        !t.app.light.lit_by_flash() && (1..=2).contains(&runs),
        format!("a flash works out its shadows at most twice, on and off ({runs} sun passes)"),
    );
    for id in pins {
        let f = field(&t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }

    // The weather section leaves the colony as it found it.
    t.keep_well();

    // ---------------------------------------------------------- 8f4f1de8 sun shadows
    println!("\n# the sun casts shadows, and a still sun costs nothing (8f4f1de8)");
    // Calm, through every lighting check to come: the storm above would go
    // on flashing, and a flash lights the ground from where its bolt is.
    let calm = [field(&t, "precipitation"), field(&t, "wind")];
    for f in calm {
        t.app.sim.world.fields.set_ambient(f, Some(0.0));
    }
    for _ in 0..60 {
        if t.app.sky.flash().strength == 0.0 && !t.app.light.lit_by_flash() {
            break;
        }
        t.frame().await;
    }
    // One wall, the sun pinned due south and low, so its shadow falls north:
    // 1 / tan 12° = 4.7 cells. The column it falls along is clear, and so is
    // the ground south of the wall a ray could meet something tall on.
    t.app.paused = true;
    let clear = |w: &World, p: IVec| {
        (-8..=11).all(|dy| {
            let c = p.offset(0, dy);
            w.map.inb(c) && w.map.passable(c) && w.map.fixture_at(c).is_none() && w.solid_at(c).is_none()
        })
    };
    let column = (0..60i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| site.offset(dx, dy))))
        .find(|&p| clear(t.w(), p));
    if let Some(p) = column {
        let standing = t.app.sim.world.spawn_fixture(wall, p, false);
        t.check(standing.is_some(), "the test wall stands");
        // Full daylight under a clear sky, whatever the hour the test has
        // reached: cloud softens a shadow's edge, and this is its length.
        let field = |t: &T, id: &str| t.w().defs.lookup("field", id).unwrap() as usize;
        let (cloud, light) = (field(&t, "cloud"), field(&t, "light"));
        t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
        t.app.sim.world.fields.set_ambient(light, Some(100.0));
        t.ticks(20);
        // The eye adapted to the night above would push this bright day past
        // white, where no shadow shows.
        t.app.light.adapt_now();
        t.app.light.pin_sun = Some((90.0, 12.0));
        t.focus(p.offset(0, -3));
        t.frame().await;
        // Walk north from the wall's north face a texel (half a cell) at a
        // time, down the middle of its column, to where the sun comes back.
        let (x, face) = (p.x as f32 + 0.5, p.y as f32);
        let length = (0..20)
            .map(|k| 0.25 + k as f32 * 0.5)
            .find(|d| t.app.light.sun_visibility(x, face - d).is_some_and(|v| v >= 0.5))
            .map_or(99.0, |d| d - 0.25);
        t.check((4.5..=5.5).contains(&length), format!("a wall's shadow at 12° is {length} cells long (4.7 by tan)"));
        let lit = t.app.light.sun_visibility(x, face - 7.0).unwrap_or(0.0);
        t.check(lit > 0.9, format!("and the sun reaches beyond it ({lit:.2})"));
        let runs = t.app.light.sun_runs;
        for _ in 0..5 {
            t.frame().await;
        }
        t.check(t.app.light.sun_runs == runs, "a still sun is worked out once, not every frame");
        let dpi = screen_dpi_scale();
        // A grabbed frame is GL's, bottom row first.
        let lum = |img: &Image, (x, y): (f32, f32)| {
            let (w, h) = (img.width() as u32, img.height() as u32);
            let (xi, yi) = (((x * dpi) as u32).min(w - 1), ((y * dpi) as u32).min(h - 1));
            let c = img.get_pixel(xi, h - 1 - yi);
            c.r + c.g + c.b
        };
        // Three spots across a cell, at `fy` down it; a pair of cells is
        // compared spot by spot and the middle ratio taken, so a colonist
        // standing on one spot doesn't decide a check.
        let spots = |t: &T, q: IVec, fy: f32| {
            [0.3f32, 0.5, 0.7].map(|fx| t.app.cam.to_screen(q.x as f32 + fx, q.y as f32 + fy))
        };
        let ratio = |img: &Image, a: [(f32, f32); 3], b: [(f32, f32); 3]| {
            let mut r = [0, 1, 2].map(|k| lum(img, a[k]) / lum(img, b[k]).max(1e-3));
            r.sort_by(f32::total_cmp);
            r[1]
        };
        // On screen, the shadow is north of the wall, not south of it: the
        // multiply reads the sun where the world is drawn.
        let shaded = |t: &T, img: &Image| ratio(img, spots(t, p.offset(0, -2), 0.5), spots(t, p.offset(0, 2), 0.5));
        let low = t.grab().await;
        t.app.light.pin_sun = Some((90.0, 60.0));
        let high = t.grab().await;
        let dark = shaded(&t, &low) / shaded(&t, &high).max(1e-3);
        t.check(dark < 0.8, format!("on screen the shadow falls north of the wall, away from the sun ({dark:.2})"));
        t.app.light.pin_sun = Some((90.0, 12.0));
        t.shot("sun_shadows").await;
        // The plan's contact shadow (0779def9): just under the wall at night,
        // gone where the sun reaches. The same spot is compared with ground
        // further south in each light, so the ground's own colour cancels out.
        t.focus(p.offset(0, 2));
        let (under, far) = (spots(&t, p.offset(0, 1), 0.1), spots(&t, p.offset(0, 4), 0.1));
        t.app.light.pin_sun = Some((90.0, 60.0));
        let day = t.grab().await;
        t.app.light.pin_sun = Some((90.0, -10.0));
        let night = t.grab().await;
        let contact = ratio(&night, under, far) / ratio(&day, under, far).max(1e-3);
        t.check(
            (0.6..0.9).contains(&contact),
            format!("the plan's contact shadow darkens under a wall at night and goes in the sun ({contact:.2})"),
        );
        t.app.light.pin_sun = None;
        t.app.sim.world.fields.set_ambient(cloud, None);
        t.app.sim.world.fields.set_ambient(light, None);
        if let Some(e) = standing {
            t.app.sim.world.despawn_thing(e);
        }
    } else {
        t.check(false, "a clear column of ground for the sun test");
    }
    t.app.paused = false;

    // ---------------------------------------------------------- 6fd6b13b firelight
    println!("\n# firelight is baked, and a new fire redoes only its own ground (6fd6b13b)");
    t.app.paused = true;
    let campfire = defs.thing_id("campfire").unwrap();
    // Two free cells six apart with free ground between: the fires compare
    // with themselves, so what grows around them doesn't matter.
    let free = |w: &World, p: IVec| w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
    let row = (0..60i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| site.offset(dx, dy))))
        .find(|&p| (0..=6).all(|dx| free(t.w(), p.offset(dx, 0))));
    if let Some(o) = row.map(|p| p.offset(-3, -7)) {
        let (a, c) = (o.offset(3, 7), o.offset(9, 7));
        let first = t.app.sim.world.spawn_fixture(campfire, a, false);
        t.ticks(2);
        t.light_settles().await;
        let glow = |t: &T, q: IVec| {
            t.app.light.fire_at(q.x as f32 + 0.5, q.y as f32 + 0.5).map_or(0.0, |v| v.iter().sum::<f32>())
        };
        let (at_a, between) = (glow(&t, a), glow(&t, o.offset(6, 7)));
        t.check(at_a > 0.3, format!("a campfire glows ({at_a:.2})"));
        // Six cells away, a second fire's glow overlaps the first's.
        let second = t.app.sim.world.spawn_fixture(campfire, c, false);
        t.ticks(2);
        t.light_settles().await;
        let draws = t.app.light.passes.iter().find(|p| p.name == "firelight").map_or(0, |p| p.draws);
        t.check(draws == 2, format!("the new fire redoes one area, not the map ({draws} draws)"));
        let spots = [a, c, o.offset(6, 7), o.offset(0, 7), o.offset(12, 7)];
        let partial: Vec<f32> = spots.iter().map(|&q| glow(&t, q)).collect();
        t.check(
            partial[1] > 0.3 && partial[2] > between + 0.05,
            format!(
                "the new one glows, and adds where they meet ({:.2}, {between:.2} to {:.2})",
                partial[1], partial[2]
            ),
        );
        // The same fires baked whole: a partial bake adds nothing twice.
        t.app.light.invalidate();
        t.frame().await;
        let whole: Vec<f32> = spots.iter().map(|&q| glow(&t, q)).collect();
        let off = partial.iter().zip(&whole).map(|(p, w)| (p - w).abs()).fold(0.0f32, f32::max);
        t.check(
            off < 0.02,
            format!("a partial bake matches a whole one (off by {off:.3}; first fire {at_a:.2} to {:.2})", whole[0]),
        );
        for e in [first, second].into_iter().flatten() {
            t.app.sim.world.despawn_thing(e);
        }
    } else {
        t.check(false, "free ground for two campfires");
    }
    // ---------------------------------------------------------- 2f13e01d indoors
    println!("\n# indoors: sunbeams through windows, and a room lit to its corners (2f13e01d)");
    t.app.paused = true;
    let field = |t: &T, id: &str| t.w().defs.lookup("field", id).unwrap() as usize;
    let (cloud, light) = (field(&t, "cloud"), field(&t, "light"));
    t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
    t.app.sim.world.fields.set_ambient(light, Some(100.0));
    let (window, stove) = (defs.thing_id("window").unwrap(), defs.thing_id("stove").unwrap());
    let deep = defs.lookup("terrain", "deep_water").unwrap();
    let mut placed = Vec::new();
    let mut flooded = Vec::new();
    // A free square `side` across, the nearest to `near`, with a cell of
    // free ground round it so one room doesn't wall in another.
    let square = |w: &World, near: IVec, side: i32| {
        let free =
            |p: IVec| w.map.inb(p) && w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
        (0..80i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| near.offset(dx, dy))))
            .find(|o| (-1..=side).all(|y| (-1..=side).all(|x| free(o.offset(x, y)))))
    };
    // A ring of walls `side` across, the middle of its west wall a window,
    // deep water or wall as asked, and a stove in the middle if asked.
    let mut build = |t: &mut T, near: IVec, side: i32, west: &str, lit: bool| {
        let o = square(t.w(), near, side)?;
        for dy in 0..side {
            for dx in 0..side {
                let edge = dx == 0 || dy == 0 || dx == side - 1 || dy == side - 1;
                let p = o.offset(dx, dy);
                let west = if dx == 0 && dy == side / 2 { west } else { "wall" };
                if !edge {
                    continue;
                }
                if west == "water" {
                    let m = &mut t.app.sim.world.map;
                    flooded.push((p, m.terrain[m.idx(p)]));
                    m.set_terrain(p, deep, defs.terrain[deep as usize].path_cost);
                } else {
                    let what = if west == "window" { window } else { wall };
                    placed.push(t.app.sim.world.spawn_fixture(what, p, false)?);
                }
            }
        }
        if lit {
            placed.push(t.app.sim.world.spawn_fixture(stove, o.offset(side / 2, side / 2), false)?);
        }
        Some(o)
    };
    let beamed = build(&mut t, site.offset(-24, 12), 5, "window", false);
    let dark = build(&mut t, site.offset(-24, 20), 5, "wall", true);
    let hall = build(&mut t, site.offset(-34, 12), 7, "wall", true);
    let moat = build(&mut t, site.offset(-34, 22), 5, "water", false);
    t.ticks(2);
    t.light_settles().await;
    let inside = |o: IVec, side: i32| (1..side - 1).flat_map(move |dy| (1..side - 1).map(move |dx| o.offset(dx, dy)));
    let texels = |q: IVec| {
        [0.25, 0.75].into_iter().flat_map(move |fy| [0.25, 0.75].map(|fx| (q.x as f32 + fx, q.y as f32 + fy)))
    };
    if let (Some(beamed), Some(dark), Some(hall), Some(moat)) = (beamed, dark, hall, moat) {
        let rooms = [(beamed, 5), (dark, 5), (hall, 7), (moat, 5)];
        let all_in = rooms.iter().all(|&(o, s)| t.w().map.indoors(o.offset(s / 2, s / 2)));
        t.check(all_in, "the huts, the hall and a hut closed by water are rooms");
        // Low in the west, the sun comes through the west window in a bar
        // across the floor; high in the south, it can't get in at all.
        let lit_inside = |light: &crate::light::Light, img: &Image, o: IVec| {
            inside(o, 5).flat_map(texels).filter_map(|(x, y)| light.sun_in(img, x, y)).fold(0.0f32, f32::max)
        };
        t.app.light.pin_sun = Some((180.0, 12.0));
        t.frame().await;
        let beam = t.app.light.sun_image().map_or(0.0, |img| lit_inside(&t.app.light, &img, beamed));
        t.check(beam > 0.5, format!("a low western sun throws a beam through the west window ({beam:.2})"));
        t.app.light.pin_sun = Some((90.0, 60.0));
        t.frame().await;
        let noon = t.app.light.sun_image().map_or(1.0, |img| lit_inside(&t.app.light, &img, beamed));
        t.check(noon < 0.05, format!("and none at noon from the south, where there is no window ({noon:.2})"));
        // A room with no window never sees the sun, from anywhere in the sky,
        // even where water rather than a wall closes it.
        let (mut leak, mut wet) = (0.0f32, 0.0f32);
        for az in (0..360).step_by(30) {
            for elev in [4.0, 12.0, 35.0] {
                t.app.light.pin_sun = Some((az as f64, elev));
                t.frame().await;
                if let Some(img) = t.app.light.sun_image() {
                    leak = leak.max(lit_inside(&t.app.light, &img, dark));
                    wet = wet.max(lit_inside(&t.app.light, &img, moat));
                } else {
                    (leak, wet) = (1.0, 1.0);
                }
            }
        }
        t.check(leak < 0.01, format!("a windowless room sees no sun from any angle ({leak:.2})"));
        t.check(wet < 0.01, format!("nor does one closed by water, not a wall ({wet:.2})"));
        t.app.light.pin_sun = None;
        // A stove lights a small hut to its corners; in a hall its corners
        // stay dim. Room fill is the difference.
        let glow = |t: &T, q: IVec| {
            t.app.light.fire_at(q.x as f32 + 0.5, q.y as f32 + 0.5).map_or(0.0, |c| c.iter().sum::<f32>())
        };
        let corner =
            |t: &T, o: IVec, side: i32| glow(t, o.offset(1, 1)) / glow(t, o.offset(side / 2, side / 2)).max(1e-3);
        let (small, big) = (corner(&t, dark, 5), corner(&t, hall, 7));
        t.check(small > 0.6, format!("a stove lights a 3×3 hut to its corners ({small:.2} of its middle)"));
        t.check(big < small * 0.5, format!("and a 5×5 hall's corners half as well at most ({big:.2})"));
        let bakes = t.app.light.bakes;
        for _ in 0..5 {
            t.frame().await;
        }
        t.check(t.app.light.bakes == bakes, "firelight and room fill are baked once, not every frame");
        t.shot("indoors").await;
        // A wall where no light reaches rebuilds the rooms but leaves every
        // fill as it was: nothing is baked. Opening the lit hut empties its
        // fill, and it is.
        let lamps: Vec<IVec> = t.w().fields.emitters_of(light).map(|(_, p, _, _)| p).collect();
        // Far from the wall's whole chunk, so this holds however finely the
        // occluders say what changed.
        let clear = |p: IVec| {
            let c = rim_sim::map::CHUNK;
            let (x0, y0) = (p.x.div_euclid(c) * c, p.y.div_euclid(c) * c);
            let gap = |l: &IVec| (x0 - l.x).max(l.x - (x0 + c - 1)).max((y0 - l.y).max(l.y - (y0 + c - 1)));
            lamps.iter().all(|l| gap(l) > crate::light::MAX_REACH as i32 + 1)
        };
        let free = |w: &World, p: IVec| w.map.inb(p) && w.map.passable(p) && w.map.fixture_at(p).is_none();
        let near = site.offset(40, -40);
        let far = (0..160i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| near.offset(dx, dy))))
            .find(|&p| free(t.w(), p) && clear(p));
        if let Some(far) = far {
            // Rooms rebuilt by hand, not by ticking: a tick can finish
            // building a fire somewhere, and that would rightly rebake.
            t.light_settles().await;
            let rebuilds = t.w().map.room_rebuilds;
            let bakes = t.app.light.bakes;
            placed.extend(t.app.sim.world.spawn_fixture(wall, far, false));
            t.app.sim.world.map.ensure_rooms();
            t.light_settles().await;
            t.check(
                t.w().map.room_rebuilds > rebuilds && t.app.light.bakes == bakes,
                format!(
                    "a wall far from any light rebuilds rooms, not firelight ({} bakes)",
                    t.app.light.bakes - bakes
                ),
            );
        } else {
            t.check(false, "free ground far from every light");
        }
        // Just past a light's reach, in a chunk that reaches it: only the
        // cells that changed count, so the light isn't redone (4b6c3a9c).
        let lights: Vec<(IVec, i32)> =
            t.w().fields.emitters_of(light).filter(|e| e.2 > 0.0).map(|(_, p, _, r)| (p, r as i32)).collect();
        // Past its reach and the cell a change is padded by for the filter.
        let past = |p: IVec| lights.iter().all(|&(l, r)| (l.x - p.x).abs().max((l.y - p.y).abs()) > r.max(1) + 2);
        let c = rim_sim::map::CHUNK;
        let shares_chunk = |p: IVec| {
            let (x0, y0) = (p.x.div_euclid(c) * c, p.y.div_euclid(c) * c);
            lights.iter().any(|&(l, r)| (x0 - l.x).max(l.x - (x0 + c - 1)).max((y0 - l.y).max(l.y - (y0 + c - 1))) <= r)
        };
        let stove_at = dark.offset(2, 2);
        let near = (0..24i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| stove_at.offset(dx, dy))))
            .find(|&p| {
                let w = t.w();
                w.map.inb(p) && w.map.passable(p) && w.map.fixture_at(p).is_none() && past(p) && shares_chunk(p)
            });
        if let Some(near) = near {
            t.light_settles().await;
            let bakes = t.app.light.bakes;
            placed.extend(t.app.sim.world.spawn_fixture(wall, near, false));
            t.app.sim.world.map.ensure_rooms();
            t.light_settles().await;
            t.check(
                t.app.light.bakes == bakes,
                format!(
                    "a wall just past a light's reach, in its chunk, redoes no firelight ({} bakes)",
                    t.app.light.bakes - bakes
                ),
            );
        } else {
            t.check(false, "free ground just past a light's reach");
        }
        let before = glow(&t, dark.offset(1, 1));
        let door = t.w().map.fixture_at(dark.offset(0, 2));
        if let Some(e) = door {
            placed.retain(|&p| p != e);
            t.app.sim.world.despawn_thing(e);
        }
        let bakes = t.app.light.bakes;
        t.ticks(2);
        t.light_settles().await;
        let after = glow(&t, dark.offset(1, 1));
        t.check(
            !t.w().map.indoors(dark.offset(2, 2)) && t.app.light.bakes > bakes && after < before - 0.05,
            format!("opening the lit hut rebakes it without its fill (corner {before:.2} to {after:.2})"),
        );
    } else {
        t.check(false, "clear ground for two huts and a hall");
    }
    for e in placed {
        t.app.sim.world.despawn_thing(e);
    }
    for (p, was) in flooded {
        t.app.sim.world.map.set_terrain(p, was, defs.terrain[was as usize].path_cost);
    }
    t.app.sim.world.fields.set_ambient(cloud, None);
    t.app.sim.world.fields.set_ambient(light, None);
    t.app.paused = false;

    // ---------------------------------------------------------- 153dda59 roofs take the sun
    println!("\n# roofs take the sun: a hipped shadow, and the slope facing the sun is bright (153dda59)");
    t.app.paused = true;
    t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
    t.app.sim.world.fields.set_ambient(light, Some(100.0));
    // An L of walls: an arm 8 across and 5 deep, and one 5 across running
    // 9 down from its west end, with clear ground 9 cells east of it.
    let in_l =
        |x: i32, y: i32| (0..8).contains(&x) && (0..5).contains(&y) || (0..5).contains(&x) && (0..9).contains(&y);
    // Open ground, or ground with only plants on it, which are cleared: a
    // block that size with nothing growing is rare on a wooded map.
    let plant =
        |w: &World, p: IVec| w.map.fixture_at(p).and_then(|e| w.thing(e)).is_some_and(|t| w.defs.thing(t.def).natural);
    let clear = |w: &World, o: IVec| {
        let free = |p: IVec| {
            w.map.inb(p)
                && w.map.passable(p)
                && w.solid_at(p).is_none()
                && w.map.item_at(p).is_none()
                && (w.map.fixture_at(p).is_none() || plant(w, p))
        };
        (-1..=10).all(|y| (-1..=17).all(|x| free(o.offset(x, y))))
    };
    t.app.light.adapt_now();
    let near = site.offset(30, -30);
    let l_at = (0..80i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| near.offset(dx, dy))))
        .find(|&o| clear(t.w(), o));
    let mut walls = Vec::new();
    if let Some(o) = l_at {
        for y in -1..=10 {
            for x in -1..=17 {
                if plant(t.w(), o.offset(x, y)) {
                    let e = t.w().map.fixture_at(o.offset(x, y)).unwrap();
                    t.app.sim.world.despawn_thing(e);
                }
            }
        }
        for y in 0..9 {
            for x in 0..8 {
                let edge = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)]
                    .iter()
                    .any(|&(dx, dy)| !in_l(x + dx, y + dy));
                if in_l(x, y) && edge {
                    walls.extend(t.app.sim.world.spawn_fixture(wall, o.offset(x, y), false));
                }
            }
        }
        t.ticks(2);
        let (arm, leg) = (o.offset(3, 2), o.offset(2, 6));
        t.frame().await;
        let one_house = t.w().map.indoors(arm)
            && t.app.roofs.house_at(t.w(), arm) != 0
            && t.app.roofs.house_at(t.w(), arm) == t.app.roofs.house_at(t.w(), leg);
        t.check(one_house, "an L of walls is one house under one roof");
        // Low in the west, the arm's roof throws a shadow east longer than
        // its walls alone would: 4 cells past its east wall is shaded,
        // though a storey-high box would leave it lit at this elevation.
        t.app.light.pin_sun = Some((180.0, 15.0));
        t.focus(o.offset(8, 4));
        t.app.cam.zoom = 20.0;
        t.frame().await;
        let (roofed, beyond) = t.app.light.sun_image().map_or((1.0, 0.0), |img| {
            let at = |dx: f32| t.app.light.sun_in(&img, o.x as f32 + 8.0 + dx, o.y as f32 + 2.5).unwrap_or(1.0);
            (at(4.0), at(7.5))
        });
        let wall_only = 1.0 / 15f32.to_radians().tan();
        t.check(
            roofed < 0.3 && beyond > 0.7,
            format!(
                "the roof's shadow runs past a wall's {wall_only:.1} cells: 4 cells out {roofed:.2}, 7.5 out {beyond:.2}"
            ),
        );
        t.shot("roof_shadow_dusk").await;
        // Zoomed out, where roofs are drawn: in the morning the east slope
        // is the bright one, in the evening the west.
        t.app.cam.zoom = 9.0;
        t.focus(o.offset(4, 4));
        // The pointer off the house, or its roof lifts.
        t.mouse = (4.0, 200.0);
        let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let mut slopes = Vec::new();
        for (name, az) in [("morning", 10.0), ("evening", 170.0)] {
            t.app.light.pin_sun = Some((az, 25.0));
            let img = t.grab().await;
            let slope = |t: &T, x: i32| {
                let (sx, sy) = t.app.cam.to_screen(o.x as f32 + x as f32 + 0.5, o.y as f32 + 2.5);
                lum(px(&img, (sx, sy)))
            };
            // On the arm's ridge row, the cells one in from its east wall
            // and its west wall: all one slope each.
            slopes.push((name, slope(&t, 6), slope(&t, 1)));
            t.shot(&format!("roof_{name}")).await;
        }
        let (m, e) = (slopes[0], slopes[1]);
        t.check(
            m.1 > m.2 * 1.2 && e.2 > e.1 * 1.2,
            format!(
                "the slope facing the sun is the bright one: morning east {:.2} west {:.2}, evening east {:.2} west {:.2}",
                m.1, m.2, e.1, e.2
            ),
        );
        t.app.light.pin_sun = None;
        t.app.cam.zoom = 40.0;
    } else {
        t.check(false, "clear ground for an L-shaped house");
    }
    for e in walls {
        t.app.sim.world.despawn_thing(e);
    }
    t.app.sim.world.fields.set_ambient(cloud, None);
    t.app.sim.world.fields.set_ambient(light, None);
    t.app.paused = false;
    for f in calm {
        t.app.sim.world.fields.set_ambient(f, None);
    }

    // ---------------------------------------------------------- 5a69f9c9 moving lights
    println!("\n# moving lights: a spreading fire glows at once and bakes at most once a second (5a69f9c9)");
    t.app.paused = true;
    t.light_settles().await;
    let flames = defs.thing_id("fire:flames").expect("the fire plugin's flames");
    // A run of open ground the fire spreads along, a cell a frame.
    let open = |w: &World, p: IVec| w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.floor_at(p).is_none();
    const RUN: i32 = 40;
    let run = (0..120i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| site.offset(dx, dy))))
        .find(|&p| (0..RUN).all(|dx| t.w().map.inb(p.offset(dx, 0)) && open(t.w(), p.offset(dx, 0))));
    let mut burning = Vec::new();
    if let Some(o) = run {
        t.focus(o.offset(RUN / 2, 0));
        let (bakes, since) = (t.app.light.bakes, get_time());
        let mut dim = f32::MAX;
        for dx in 0..RUN {
            burning.extend(t.app.sim.world.spawn_fixture(flames, o.offset(dx, 0), false));
            t.frame().await;
            // Baked or not yet, a new flame lights its own cell the frame it
            // appears.
            let (x, y) = (o.x as f32 + dx as f32 + 0.5, o.y as f32 + 0.5);
            let sum = |c: Option<[f32; 4]>| c.map_or(0.0, |c| c.iter().sum::<f32>());
            dim = dim.min(sum(t.app.light.fire_at(x, y)) + sum(t.app.light.moving_at(x, y)));
        }
        let (baked, took) = (t.app.light.bakes - bakes, get_time() - since);
        t.check(dim > 0.2, format!("every new flame glows the frame it appears (dimmest {dim:.2})"));
        t.check(
            (baked as f64) <= took.ceil() + 1.0,
            format!("{RUN} flames in {took:.1} s bake {baked} times: at most once a second"),
        );
        t.light_settles().await;
        t.check(t.app.light.moving_lit.1 == 0, "and once the bake catches up, none is left moving");
    } else {
        t.check(false, "a run of open ground for a fire");
    }
    // 64 lights moving round the view, each a little further out than the
    // last: the preset's 8 nearest cast shadows, and all of them glow.
    let (cx, cy) = (t.app.cam.x, t.app.cam.y);
    let ring = |k: usize| {
        let (r, a) = (3.0 + 0.2 * k as f32, k as f32 * 0.098);
        vec2(cx + r * a.cos(), cy + r * a.sin())
    };
    t.app.light.set_moving((0..64).map(|k| (ring(k), 4.0, 60.0)));
    t.frame().await;
    let (shadowed, all) = t.app.light.moving_lit;
    let far = ring(63);
    let glow = t.app.light.moving_at(far.x, far.y).map_or(0.0, |c| c.iter().sum::<f32>());
    t.check(
        (shadowed, all) == (8, 64) && glow > 0.2,
        format!("64 moving lights: {shadowed} cast shadows, {all} drawn, and past the cap they glow ({glow:.2})"),
    );
    t.shot("moving_lights").await;
    t.app.light.set_moving([]);
    for e in burning {
        t.app.sim.world.despawn_thing(e);
    }
    t.light_settles().await;
    t.app.paused = false;

    // ---------------------------------------------------------- 508ad373 lighting from the palette
    println!("\n# a lighting preset from the palette takes at once (508ad373)");
    // The binding has no key of its own; lend it one, as a player could.
    t.app.ui.rebind("core:lighting_low", Some("f9"));
    t.key(KeyCode::F9).await;
    t.frame().await;
    let q = t.app.light.setting.quality;
    t.check(
        t.app.light.setting.name() == "low" && q.sun_steps == 16 && !q.soft,
        format!("Lighting: low takes at once ({}, {} sun steps)", t.app.light.setting.name(), q.sun_steps),
    );
    t.app.ui.rebind("core:lighting_low", None);
    crate::apply_ui(&mut t.app, rim_ui::view::UiAction::Lighting("medium".into()));
    t.frame().await;
    t.check(t.app.light.setting.name() == "medium", "and back to medium");

    // ---------------------------------------------------------- fda56c8e camera by device
    println!("\n# the camera answers a mouse and a trackpad (fda56c8e)");
    t.clear_dock().await;
    t.focus(t.pawn(founder).pos);
    t.frame().await;
    let mid = (600.0, 400.0);
    let cam = |t: &T| (t.app.cam.x, t.app.cam.y, t.app.cam.zoom);
    // Let any earlier scroll's stickiness lapse.
    for _ in 0..30 {
        t.frame().await;
    }
    let before = cam(&t);
    t.input(RawInput {
        mouse: mid,
        scroll: crate::Scroll { notches: 0.0, travel: (12.0, -30.0) },
        ..Default::default()
    })
    .await;
    let after = cam(&t);
    t.check(
        (after.0 - (before.0 - 12.0 / before.2)).abs() < 1e-3 && (after.1 - (before.1 + 30.0 / before.2)).abs() < 1e-3,
        format!("a trackpad's travel pans one to one ({before:?} → {after:?})"),
    );
    t.check(after.2 == before.2, "and doesn't zoom");
    for _ in 0..30 {
        t.frame().await;
    }
    let before = cam(&t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    let after = cam(&t);
    t.check((after.2 / before.2 - 1.12).abs() < 1e-3, format!("a wheel notch zooms 12% ({:.3})", after.2 / before.2));
    let before = cam(&t);
    t.input(RawInput {
        mouse: mid,
        scroll: crate::Scroll { notches: 0.0, travel: (0.0, 40.0) },
        zoom_mod: true,
        ..Default::default()
    })
    .await;
    t.check(cam(&t).2 > before.2, "Cmd with a trackpad scroll zooms");
    // A round-numbered delta right after trackpad scrolling is still the trackpad.
    let before = cam(&t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 0.0, travel: (0.0, 5.0) }, ..Default::default() })
        .await;
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    t.check(cam(&t).2 == before.2, "a whole notch amid trackpad scrolling pans, not zooms");
    // Right-drag pans, and gives no order.
    for _ in 0..30 {
        t.frame().await;
    }
    t.app.order_flash = None;
    let before = cam(&t);
    t.input(RawInput { mouse: mid, right_pressed: true, right_down: true, ..Default::default() }).await;
    t.input(RawInput { mouse: (mid.0 + 50.0, mid.1), right_down: true, ..Default::default() }).await;
    t.input(RawInput { mouse: (mid.0 + 50.0, mid.1), right_released: true, ..Default::default() }).await;
    t.check(
        (cam(&t).0 - (before.0 - 50.0 / before.2)).abs() < 1e-3,
        format!("a right-drag pans the ground with the pointer ({before:?} → {:?})", cam(&t)),
    );
    t.check(t.app.order_flash.is_none() && t.app.ui.find("core:menu").is_none(), "and gives no order");
    // The setting pins it: with "pan", a wheel pans too.
    t.app.scroll_mode = crate::ScrollMode::Pan;
    let before = cam(&t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    t.check(cam(&t).2 == before.2 && cam(&t).1 != before.1, "with scroll set to pan, a wheel pans");
    t.app.scroll_mode = crate::ScrollMode::Auto;
    // A pinch: 10% apart zooms 10% about the pointer, the ground under it
    // staying put.
    let before = cam(&t);
    let under = t.app.cam.to_world(mid.0, mid.1);
    t.input(RawInput { mouse: mid, pinch: 0.1, ..Default::default() }).await;
    let after_under = t.app.cam.to_world(mid.0, mid.1);
    t.check(
        (cam(&t).2 / before.2 - 1.1).abs() < 1e-3
            && (after_under.0 - under.0).abs() < 1e-3
            && (after_under.1 - under.1).abs() < 1e-3,
        "a pinch zooms about the pointer",
    );
    let before = cam(&t);
    t.key(KeyCode::Equal).await;
    t.check(cam(&t).2 > before.2, "= zooms in");

    // ---------------------------------------------------------- 9aa55d96 safe right-click
    println!("\n# a right-click never takes a wall down (9aa55d96)");
    t.clear_dock().await;
    t.app.paused = true;
    let ours = {
        let w = t.w();
        w.ecs
            .query::<(Entity, &Thing, &Owner)>()
            .without::<&Blueprint>()
            .iter()
            .filter(|(_, th, o)| {
                o.0 == Faction::Player && w.defs.thing(th.def).build.is_some() && w.defs.thing(th.def).blocks
            })
            .map(|(e, th, _)| (e, th.pos))
            .next()
    };
    if let Some((wall, at)) = ours {
        crate::select(&mut t.app, vec![founder]);
        t.app.sim.push(Command::Draft { pawn: founder, on: false });
        t.ticks(1);
        t.focus(at);
        t.frame().await;
        let before = t.count::<(&Thing, &Designated)>();
        t.right_click(t.screen(at)).await;
        t.ticks(1);
        t.frame().await;
        t.check(
            t.count::<(&Thing, &Designated)>() == before && !matches!(t.pawn(founder).job, Job::Deconstruct { .. }),
            "a right-click on our wall takes nothing down",
        );
        t.check(t.app.ui.find("core:menu").is_some(), "it opens the orders menu instead");
        t.shot("orders_menu").await;
        t.check(t.ui_text().contains("\"Deconstruct\""), "its row says Deconstruct, the caption names the wall");
        // The only row that can run is Deconstruct: 1 picks it.
        t.key(KeyCode::Key1).await;
        t.ticks(1);
        t.check(
            t.w().ecs.get::<&Designated>(wall).is_ok() && matches!(t.pawn(founder).job, Job::Deconstruct { .. }),
            "picking Deconstruct from the menu gives it",
        );
        // The toast says what was ordered; Cmd/Ctrl+Z takes it back.
        t.frame().await;
        t.check(
            t.app.ui.find("core:undo").is_some() && t.ui_text().contains("will deconstruct wall"),
            "the order's toast says what it was",
        );
        t.shot("undo_toast").await;
        t.input(RawInput { mouse: t.mouse, pressed: vec!["ctrl+z".into()], ..Default::default() }).await;
        t.ticks(1);
        t.frame().await;
        t.check(
            t.w().ecs.get::<&Designated>(wall).is_err() && !matches!(t.pawn(founder).job, Job::Deconstruct { .. }),
            "Cmd/Ctrl+Z takes the deconstruct back: unmarked, and nobody on it",
        );
        t.check(t.app.ui.find("core:undo").is_none(), "and the toast goes");
        // Held on open ground: the menu, with Go here in it.
        // Ground the founder can walk to: on Auto they may have walled some off.
        let from = t.pawn(founder).pos;
        let ground = (1..8)
            .flat_map(|d| [at.offset(d, 0), at.offset(-d, 0), at.offset(0, d), at.offset(0, -d)])
            .find(|&p| t.w().map.passable(p) && t.w().map.can_reach(from, rim_sim::path::Goal::Cell(p)))
            .unwrap_or(at);
        t.right_hold(t.screen(ground)).await;
        t.frame().await;
        t.check(
            t.app.ui.find("core:menu").is_some() && t.ui_text().contains("Go here"),
            "holding right-click opens every order, going there among them",
        );
        t.key(KeyCode::Escape).await;
        t.check(t.app.ui.find("core:menu").is_none(), "Escape closes the menu");
    } else {
        t.check(false, "a wall of ours to right-click");
    }

    // ---------------------------------------------------------- 553bfb19 the grid
    // It shows while a tool is in hand, as a groove things stand on.
    println!("\n# the grid (553bfb19)");
    t.app.paused = true;
    t.clear_dock().await;
    // No rain: falling streaks would move pixels between the shots.
    let rain = t.w().defs.lookup("field", "precipitation").unwrap() as usize;
    t.app.sim.world.fields.set_ambient(rain, Some(0.0));
    let wall = defs.thing_id("wall").expect("walls");
    let o = open_square(t.w(), home, 6).expect("open ground near home");
    t.focus(o.offset(3, 3));
    t.app.cam.zoom = 28.0;
    // A built wall of its own, in the square's corner away from where the
    // grid is read: what stands on a cell hides the lines under it.
    let standing = t.app.sim.world.spawn_fixture(wall, o.offset(5, 0), false);
    let [rest, lens, plan] = grid_shots(&mut t, o, wall).await;
    // A corner two cells from the pointer: inside the lens, clear of the
    // cursor's own cell.
    let corner = t.app.cam.to_screen(o.x as f32 + 4.0, o.y as f32 + 4.0);
    let (dl, dp) = (patch_diff(&rest, &lens, corner, 4.0), patch_diff(&rest, &plan, corner, 4.0));
    t.check(dl > 0.5, format!("arming a tool puts ticks at the corners near the pointer ({dl:.2})"));
    t.check(dp > 0.5, format!("dragging draws the lines ({dp:.2})"));
    // Whatever stands on a cell hides the lines: a wall, since it never
    // sways (trees move on the wall clock, so their pixels never match).
    let d = patch_diff(&rest, &plan, t.screen(o.offset(5, 0)), 0.3 * t.app.cam.zoom);
    t.check(d < 0.5, format!("a wall hides the lines under it ({d:.2})"));
    if let Some(e) = standing {
        t.app.sim.world.despawn_thing(e);
    }
    for (name, img) in [("chalk-grid-rest", &rest), ("chalk-grid-lens", &lens), ("chalk-grid-plan", &plan)] {
        t.shots += 1;
        let path = t.dir.join(format!("{:02}_{name}.png", t.shots));
        img.export_png(path.to_str().unwrap());
        println!("shot  {}", path.display());
    }
    t.app.cam.zoom = 8.0;
    let [rest, _, plan] = grid_shots(&mut t, o, wall).await;
    let corner = t.app.cam.to_screen(o.x as f32 + 4.0, o.y as f32 + 4.0);
    let d = patch_diff(&rest, &plan, corner, 6.0);
    t.check(d < 0.2, format!("at 8 points a cell there's no grid ({d:.2})"));
    let shown = crate::grid::strength(&t.app);
    t.check(shown.lens == 0.0 && shown.plan == 0.0, format!("putting the tool down fades the grid out ({shown:?})"));
    // Measure (f5bc43e3): G turns on a counting grid whose fifth lines
    // show at any zoom, numbered along the pointer's row and column.
    t.app.cam.zoom = 6.0;
    t.focus(o.offset(3, 3));
    t.mouse = t.screen(o.offset(2, 2));
    for _ in 0..50 {
        t.frame().await;
    }
    let before = t.grab().await;
    t.key(KeyCode::G).await;
    t.check(t.app.measure, "G turns the measuring grid on");
    for _ in 0..30 {
        t.frame().await;
    }
    let measured = t.grab().await;
    // The fifth line nearest the middle, against a line two cells over,
    // read only where both run over open ground: rock and trees stand on
    // the grid and hide it.
    let (x0, y0, x1, y1) = draw::visible(&t.app);
    let every = crate::grid::MAJOR_EVERY;
    let major = ((x0 + x1) / 2).div_euclid(every) * every;
    let open = |t: &T, x: i32, y: i32| {
        let w = t.w();
        [x - 1, x].iter().all(|&cx| {
            let p = IVec::new(cx, y);
            w.map.inb(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none() && w.map.floor_at(p).is_none()
        })
    };
    let along = |x: i32, t: &T| {
        let (lx, _) = t.app.cam.to_screen(x as f32, 0.0);
        (y0 + 2..y1 - 2)
            .filter(|&y| y % every != 0 && open(t, major, y) && open(t, major + 2, y))
            .map(|y| patch_diff(&before, &measured, (lx, t.app.cam.to_screen(0.0, y as f32 + 0.5).1), 1.5))
            .fold(0.0f32, f32::max)
    };
    let (dm, dn) = (along(major, &t), along(major + 2, &t));
    t.check(dm > 0.5 && dn < 0.2, format!("at 6 points a cell only the fifth lines show ({dm:.2} against {dn:.2})"));
    t.app.cam.zoom = 28.0;
    t.focus(o.offset(3, 3));
    t.mouse = t.screen(o.offset(2, 2));
    for _ in 0..10 {
        t.frame().await;
    }
    let cell = t.app.cam.tile_at(t.mouse.0, t.mouse.1);
    let (_, row_top) = t.app.cam.to_screen(0.0, cell.y as f32);
    let (col_right, _) = t.app.cam.to_screen(cell.x as f32 + 1.0, 0.0);
    let caption = t.app.palette.caption;
    let (mut on_row, mut on_col) = (0, 0);
    for m in crate::overlay::scene(&t.app).marks {
        if let Mark::Label { at, .. } = m {
            on_row += ((at.1 - (row_top - caption - 5.0)).abs() < 0.5) as usize;
            on_col += ((at.0 - (col_right + 3.0)).abs() < 0.5) as usize;
        }
    }
    t.check(
        on_row > 0 && on_col > 0,
        format!("the fifth lines are numbered along the pointer's row ({on_row}) and column ({on_col})"),
    );
    t.shot("chalk-measure").await;
    t.key(KeyCode::G).await;
    t.check(!t.app.measure, "and G again turns it off");
    let opening = RawInput { mouse: t.mouse, pressed: vec!["ctrl+k".into()], ..Default::default() };
    t.input(opening).await;
    t.input(RawInput { mouse: t.mouse, chars: "measure".chars().collect(), ..Default::default() }).await;
    t.settle().await;
    t.check(t.ui_text().contains("Measure grid"), "the command palette finds the measuring grid");
    // The same keys close it.
    t.key(KeyCode::Escape).await;
    t.input(RawInput { mouse: t.mouse, pressed: vec!["ctrl+k".into()], ..Default::default() }).await;
    t.settle().await;
    t.check(!t.app.ui.is_open("core:palette"), "Ctrl+K closes the palette");
    t.app.cam.zoom = 28.0;
    t.app.paused = false;
    t.app.sim.world.fields.set_ambient(rain, None);

    // ---------------------------------------------------------- 86dcd0ca several selected
    // Last, since it adds colonists.
    println!("\n# several selected (86dcd0ca)");
    t.app.paused = true;
    t.clear_dock().await;
    t.key(KeyCode::Escape).await;
    let at = t.pawn(founder).pos;
    let human = defs.creature_id("human").expect("humans");
    let spots: Vec<IVec> = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1)]
        .iter()
        .map(|&(x, y)| at.offset(x, y))
        .filter(|&p| t.w().map.passable(p))
        .take(2)
        .collect();
    let mut squad = vec![founder];
    for p in spots {
        squad.push(t.app.sim.world.spawn_pawn(human, Faction::Player, p, None));
    }
    t.check(squad.len() == 3, format!("two colonists join the founder ({})", squad.len()));
    // Everyone starts undrafted, so R below shows who it reached.
    let everyone: Vec<Entity> = t.w().colonists().collect();
    for e in everyone {
        t.app.sim.push(Command::Draft { pawn: e, on: false });
    }
    t.ticks(1);
    t.focus(at);
    t.frame().await;
    // The box previews who it will pick before the button comes up (463983bb).
    let (a, b) = (at.offset(-2, -2), at.offset(2, 2));
    // Let any hover from before fade out first: it isn't the box's.
    t.mouse = t.screen(a);
    for _ in 0..12 {
        t.frame().await;
    }
    t.input(RawInput { mouse: t.screen(a), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: t.screen(b), ..Default::default() }).await;
    let marks = crate::overlay::scene(&t.app).marks;
    let rings = marks.iter().filter(|m| matches!(m, Mark::HoverRing { .. })).count();
    let chip = crate::overlay::drag_hint(&t.app);
    let boxed_up = marks.iter().any(|m| matches!(m, Mark::Marquee { dashed: false, .. }));
    // Whoever stands in the box: the three set up, and anyone the colony
    // gained since who has wandered in.
    let inside = crate::boxed_colonists(&t.app, a, b);
    let n = inside.len();
    t.check(
        boxed_up
            && squad.iter().all(|e| inside.contains(e))
            && rings == n
            && chip == Some(format!("5 × 5 · {n} colonists")),
        format!("a select drag's box previews who it picks ({rings} rings for {n} inside, {chip:?})"),
    );
    t.shot("chalk-box").await;
    t.input(RawInput { mouse: t.screen(b), left_released: true, ..Default::default() }).await;
    let boxed = crate::selection(&t.app);
    t.check(
        squad.iter().all(|e| boxed.contains(e)),
        format!("a drag with Select picks the colonists in the box ({} of {})", boxed.len(), squad.len()),
    );
    t.frame().await;
    t.check(t.app.ui.find("core:inspector.group").is_some(), "the inspector sums the group up");
    // Alt takes the boxed out; Shift puts them back, dropping nobody.
    let one = squad[2];
    let p = t.pawn(one).pos;
    // A cell beside them, so the two-cell box holds them and no one else.
    let alone = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|&(dx, dy)| p.offset(dx, dy))
        .find(|&q| crate::boxed_colonists(&t.app, p, q) == [one])
        .expect("a cell beside one colonist and no other");
    let (sa, sb) = (t.screen(p), t.screen(alone));
    for (pressed, released, at) in [(true, false, sa), (false, false, sb), (false, true, sb)] {
        t.input(RawInput {
            mouse: at,
            left_pressed: pressed,
            left_released: released,
            alt: true,
            ..Default::default()
        })
        .await;
    }
    let after = crate::selection(&t.app);
    t.check(
        after.len() + 1 == boxed.len() && !after.contains(&one),
        format!("an Alt-drag takes the boxed colonist out ({} of {} left)", after.len(), boxed.len()),
    );
    for (pressed, released, at) in [(true, false, sa), (false, false, sb), (false, true, sb)] {
        t.input(RawInput {
            mouse: at,
            left_pressed: pressed,
            left_released: released,
            shift: true,
            ..Default::default()
        })
        .await;
    }
    let again = crate::selection(&t.app);
    t.check(
        again.len() == boxed.len() && after.iter().all(|e| again.contains(e)),
        format!("a Shift-drag adds them back without dropping anyone ({})", again.len()),
    );
    t.shot("several_selected").await;
    // Each member gets a chalk ring; the inspector's at full strength (d83192ed).
    let marks = crate::overlay::scene(&t.app).marks;
    let rings: Vec<f32> =
        marks.iter().filter_map(|m| if let Mark::Ring { alpha, .. } = m { Some(*alpha) } else { None }).collect();
    let full = rings.iter().filter(|&&a| a == 1.0).count();
    t.check(
        rings.len() == boxed.len() && full == 1 && rings.iter().all(|&a| a == 1.0 || a == 0.7),
        format!("a group gets a ring each, one at full strength ({rings:?})"),
    );
    let chip = format!("{} selected", boxed.len());
    let chipped = marks.iter().any(|m| matches!(m, Mark::Chip(c) if c.text == chip));
    t.check(chipped, format!("and a chip reads '{chip}'"));
    t.shot("chalk-select-group").await;
    // A selected, hovered colonist: the hover ring on the body's edge, the
    // selection further out (bd7a158e).
    t.mouse = t.pawn_screen(founder);
    for _ in 0..8 {
        t.frame().await;
    }
    let marks = crate::overlay::scene(&t.app).marks;
    let p = &t.app.palette;
    let hover = marks.iter().find(|m| matches!(m, Mark::HoverRing { .. }));
    let center = |m: &Mark| match m {
        Mark::HoverRing { center, .. } | Mark::Ring { center, .. } => Some(*center),
        _ => None,
    };
    let hover_r = hover.and_then(|m| crate::overlay::ring_radius(p, m));
    let select_r = marks
        .iter()
        .filter(|m| matches!(m, Mark::Ring { .. }) && center(m) == hover.and_then(center))
        .find_map(|m| crate::overlay::ring_radius(p, m));
    let apart = hover_r.zip(select_r).is_some_and(|(h, s)| s - h >= p.stroke + p.firm);
    t.check(apart, format!("hover and selection rings sit apart ({hover_r:?}, {select_r:?})"));
    // Shift-click takes one out again: whoever is under the pointer.
    let at_screen = t.pawn_screen(squad[2]);
    let taken = crate::pawn_under(&t.app, at_screen.0, at_screen.1).expect("a colonist there");
    for (pressed, released) in [(false, false), (true, false), (false, true)] {
        let raw = RawInput {
            mouse: at_screen,
            left_pressed: pressed,
            left_released: released,
            shift: true,
            ..Default::default()
        };
        t.input(raw).await;
    }
    let left = crate::selection(&t.app);
    t.check(
        left.len() + 1 == boxed.len() && !left.contains(&taken),
        format!("shift-click takes a colonist out ({} of {} left)", left.len(), boxed.len()),
    );
    t.key(KeyCode::R).await;
    t.ticks(1);
    t.check(
        left.iter().all(|&e| t.pawn(e).drafted) && !t.pawn(taken).drafted,
        "R drafts every selected colonist, and only them",
    );
    // Open ground a few cells off, whichever way the map leaves some.
    let to = (3..12)
        .flat_map(|d| [at.offset(d, 0), at.offset(-d, 0), at.offset(0, d), at.offset(0, -d)])
        .find(|&p| t.w().map.passable(p))
        .expect("open ground nearby");
    let dest = t.screen(to);
    t.right_click(dest).await;
    t.ticks(1);
    t.check(
        left.iter().all(|&e| matches!(t.pawn(e).job, Job::MoveTo { .. })),
        "a right-click orders every selected colonist",
    );
    // A selection off screen leaves a chevron at the edge, clear of the
    // panels; a group that went together is one chevron; a click on it
    // brings the inspector's colonist back (b6d0a4cc).
    let picked = crate::selection(&t.app);
    crate::apply(&mut t.app, Action::Pan(40.0, 0.0));
    for _ in 0..4 {
        t.frame().await;
    }
    let chevrons = crate::overlay::offscreen(&t.app);
    let dpi = screen_dpi_scale();
    let clear = chevrons.iter().all(|o| !t.app.ui.covers(o.at.0 * dpi, o.at.1 * dpi));
    let pointed: usize = chevrons.iter().map(|o| o.of.len()).sum();
    let named =
        crate::overlay::scene(&t.app).marks.iter().any(|m| matches!(m, Mark::Chip(c) if c.text.ends_with(" cells")));
    t.check(
        pointed == picked.len() && chevrons.len() < picked.len() && clear && named,
        format!("a group off screen leaves one chevron, clear of the panels ({} for {})", chevrons.len(), picked.len()),
    );
    t.shot("chalk-offscreen").await;
    if let Some(o) = chevrons.first() {
        let lead = o.of[0];
        t.click(o.at).await;
        t.frame().await;
        let back = crate::draw::pawn_disc(&t.app, lead)
            .is_some_and(|((x, y), _)| (0.0..screen_width()).contains(&x) && (0.0..screen_height()).contains(&y));
        t.check(back, "clicking the chevron brings its colonist on screen");
    }
    t.key(KeyCode::Escape).await;
    t.check(t.app.selected.is_none() && t.app.group.is_empty(), "Escape clears the whole selection");

    // ---------------------------------------------------------- 337cb649 an urgent hunt shows
    // Pawns aren't in the chunk mesh, so a creature's urgent mark is drawn
    // with the pawns: a marked deer wears the amber disc, and loses it when
    // the hunt is called off.
    println!("\n# an urgent mark on a hunt target (337cb649)");
    // In daylight: marks are lit with the world, and the weather section
    // leaves it night, where amber and a brown deer both read near black and
    // firelight flickers across them from frame to frame.
    while !(11.0..14.0).contains(&t.w().hour()) {
        t.ticks(100);
    }
    t.app.paused = true;
    let deer_def = defs.creature_id("deer").expect("core's deer");
    let spot = (4..20)
        .flat_map(|d| [home.offset(d, d), home.offset(-d, d), home.offset(d, -d), home.offset(-d, -d)])
        .find(|&p| {
            t.w().map.passable(p) && t.w().pawns.iter().filter_map(|&e| t.w().pawn_pos(e)).all(|q| q.chebyshev(p) > 3)
        })
        .expect("open ground for a deer");
    let deer = t.app.sim.world.spawn_pawn(deer_def, Faction::Wild, spot, None);
    let hunt = defs.lookup("designation", "core:hunt").expect("core's hunt");
    t.app.sim.push(rim_sim::Command::Designate { designation: hunt, a: spot, b: spot });
    t.ticks(1);
    t.focus(spot);
    let urgent_px = |t: &T, img: &Image| {
        let (sx, sy) = t.pawn_screen(deer);
        let r = defs.creature(deer_def).size * t.app.cam.zoom;
        let (ux, uy) = draw::urgent_spot(sx, sy, r);
        let dpi = screen_dpi_scale();
        let (w, h) = (img.width() as u32, img.height() as u32);
        let (xi, yi) = (((ux * dpi) as u32).min(w - 1), ((uy * dpi) as u32).min(h - 1));
        let c = img.get_pixel(xi, h - 1 - yi);
        let m = draw::URGENT_MARK;
        (c.r - m.r).abs() + (c.g - m.g).abs() + (c.b - m.b).abs()
    };
    let img = t.grab().await;
    let calm = urgent_px(&t, &img);
    t.app.sim.push(rim_sim::Command::MarkUrgent { target: deer, on: true });
    t.ticks(1);
    let on = t.w().ecs.get::<&Urgent>(deer).is_ok();
    t.check(on, "a deer marked for hunting takes an urgent mark");
    let img = t.grab().await;
    let marked = urgent_px(&t, &img);
    t.shot("urgent_hunt").await;
    t.check(
        marked + 0.3 < calm,
        format!("the marked deer wears the amber mark ({marked:.2} from amber, {calm:.2} before)"),
    );
    let _ = t.app.sim.world.ecs.remove_one::<Designated>(deer);
    t.ticks(1);
    let off = t.w().ecs.get::<&Urgent>(deer).is_err();
    t.check(off, "calling the hunt off takes the mark away");
    let img = t.grab().await;
    let after = urgent_px(&t, &img);
    t.check(after > marked + 0.3, format!("and the amber goes from the map ({after:.2} from amber)"));
    t.app.paused = false;

    // ---------------------------------------------------------- 7ffd8d09 unreachable
    // A tree marked to chop, walled in: a notch on it, and gone once a
    // way in opens.
    println!("\n# unreachable jobs (7ffd8d09)");
    let oak = defs.thing_id("tree_oak").expect("oaks");
    let (wall, wood) = (defs.thing_id("wall").expect("walls"), defs.thing_id("wood").unwrap());
    let chop = defs.lookup("designation", "chop").unwrap();
    let o = open_square(t.w(), home, 3).expect("open ground for an island");
    let middle = o.offset(1, 1);
    let tree = t.app.sim.world.spawn_fixture(oak, middle, false).expect("a tree");
    let ring: Vec<IVec> = (0..3).flat_map(|y| (0..3).map(move |x| o.offset(x, y))).filter(|&c| c != middle).collect();
    let walls: Vec<Option<Entity>> =
        ring.iter().map(|&c| t.app.sim.world.spawn_fixture_of(wall, c, false, Some(wood))).collect();
    t.check(walls.iter().all(Option::is_some), "eight walls round the island's tree");
    t.app.sim.push(Command::Designate { designation: chop, a: middle, b: middle });
    t.ticks(1);
    t.focus(middle);
    t.app.cam.z = middle.z;
    t.input(RawInput { mouse: t.screen(middle), ..Default::default() }).await;
    t.frame().await;
    let notched = |t: &T| {
        let bottom_left = t.app.cam.to_screen(middle.x as f32, middle.y as f32 + 1.0);
        crate::overlay::scene(&t.app).marks.iter().any(|m| {
            matches!(m, Mark::Notch { at, .. } if (at.0 - bottom_left.0).abs() < 1.0 && (at.1 - bottom_left.1).abs() < 1.0)
        })
    };
    let marked = t.w().ecs.get::<&Designated>(tree).is_ok();
    t.check(marked && notched(&t), format!("a marked tree walled in carries a notch (marked {marked})"));
    let says = crate::overlay::scene(&t.app)
        .marks
        .iter()
        .any(|m| matches!(m, Mark::Chip(c) if c.text == "No one can reach this"));
    t.check(says, "hovering it says no one can reach it");
    t.shot("chalk-unreachable").await;
    // A way in: the wall to the left of the tree goes.
    if let Some(e) = walls[3] {
        t.app.sim.world.despawn_thing(e);
    }
    t.ticks(1);
    t.frame().await;
    t.check(!notched(&t), "a way in takes the notch away");
    for e in walls.into_iter().flatten() {
        if t.w().thing(e).is_some() {
            t.app.sim.world.despawn_thing(e);
        }
    }
    t.app.sim.world.despawn_thing(tree);

    // Water in basins (202b16c4), last: it runs the world on, which moves
    // what sections after it would find. A room dug beside water that never runs
    // out fills ring by ring from where it comes in, and is drawn so.
    {
        println!("\n# water drawn by depth (202b16c4)");
        let paused = t.app.paused;
        t.app.paused = true;
        let defs = t.w().defs.clone();
        let o = IVec::at(home.x + 20, home.y - 20, -1);
        let mut room = Vec::new();
        for y in 0..10 {
            for x in 0..14 {
                let p = o.offset(x, y);
                if let Some(leaves) = t.w().solid_at(p).and_then(|r| r.leaves_r) {
                    t.app.sim.world.map.set_terrain(p, leaves, defs.terrain[leaves as usize].path_cost);
                    room.push(p);
                }
            }
        }
        // A river running into its west wall.
        let river = defs.terrain.iter().position(|d| d.pours > 0).expect("water that pours") as rim_sim::defs::DefId;
        t.app.sim.world.map.set_terrain(o.offset(-1, 5), river, 0);
        let wet = |t: &T| room.iter().filter(|&&p| t.w().water_depth(p) > 0).count();
        t.ticks(6);
        t.app.cam.z = -1;
        t.focus(o.offset(7, 5));
        t.app.cam.zoom = 24.0;
        for _ in 0..12 {
            t.frame().await;
        }
        let early = wet(&t);
        t.check(
            early > 0 && early < room.len(),
            format!("the water comes in and spreads ({early} of {} cells)", room.len()),
        );
        t.shot("water_filling").await;
        t.ticks(400);
        for _ in 0..12 {
            t.frame().await;
        }
        let full = room.iter().all(|&p| t.w().water_depth(p) == rim_sim::water::FULL);
        t.check(full, "and fills the room to the brim");
        t.shot("water_full").await;
        t.app.cam.z = 0;
        t.app.paused = paused;
    }

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
