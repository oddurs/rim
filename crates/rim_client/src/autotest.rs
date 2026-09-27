//! `rim --autotest [dir]` drives the real client through every control.
//!
//! It feeds synthetic raw input through the same `frame()` the game loop
//! uses: world clicks and drags pass through the UI's routing exactly as the
//! mouse's would, and UI controls are clicked by node id (`core:toolbar.
//! designate:core:chop`), not screen position. It checks the game state after
//! each step, saves screenshots to `dir` (default `target/autotest`), and
//! exits non-zero if any check failed.

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
    // Select, cancel, stockpile and clear zone, besides one per def.
    let n_expected = 4 + markable + defs.things.iter().filter(|d| d.build.is_some()).count();
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
    t.click_tool("build:door").await;
    t.drag(site.offset(5, 0), site.offset(5, 0)).await;
    t.click_tool("build:bed").await;
    t.drag(site.offset(2, 2), site.offset(2, 2)).await;
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 21,
        format!("walls, a door and a bed are planned ({})", t.count::<&Blueprint>()),
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
    let (cx, cy) = t.screen(row);
    let seam = px(&img, (cx + z / 2.0, cy));
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
    let dest = (3..12)
        .flat_map(|r| [here.offset(r, 0), here.offset(-r, 0), here.offset(0, r), here.offset(0, -r)])
        .find(|p| t.w().map.passable(*p) && t.w().map.region_at(*p) == t.w().map.region_at(here))
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
    if let Some(p) = tree {
        t.drag(p, p).await;
    }
    let spot = open_square(t.w(), home.offset(10, 10), 2).unwrap_or(home.offset(10, 10));
    t.click_tool("build:core:wall").await;
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
    // One colonist on Auto opens to their plan; the stance step reads the board.
    t.check(t.app.ui.find("core:work.show_board").is_some(), "one colonist on Auto opens to their plan");
    t.shot("work_plan").await;
    t.click_ui("core:work.show_board").await;
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
    t.check(t.app.overlay.is_none(), "O again turns the overlay off");
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
    let before = t.w().tick;
    t.click_ui("weather:devtools.advance.24").await;
    let skipped = t.w().tick - before;
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
    }
    t.focus(site.offset(3, 3));
    t.shot("night").await;

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
    t.app.sky.strike();
    t.shot("storm").await;
    for id in pins {
        let f = field(&t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }

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
    t.drag(at.offset(-2, -2), at.offset(2, 2)).await;
    let boxed = crate::selection(&t.app);
    t.check(
        squad.iter().all(|e| boxed.contains(e)),
        format!("a drag with Select picks the colonists in the box ({} of {})", boxed.len(), squad.len()),
    );
    t.frame().await;
    t.check(t.app.ui.find("core:inspector.group").is_some(), "the inspector sums the group up");
    t.shot("several_selected").await;
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
    let to = (3..12).map(|d| at.offset(d, 0)).find(|&p| t.w().map.passable(p)).expect("open ground east");
    let dest = t.screen(to);
    t.right_click(dest).await;
    t.ticks(1);
    t.check(
        left.iter().all(|&e| matches!(t.pawn(e).job, Job::MoveTo { .. })),
        "a right-click orders every selected colonist",
    );
    t.key(KeyCode::Escape).await;
    t.check(t.app.selected.is_none() && t.app.group.is_empty(), "Escape clears the whole selection");

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
