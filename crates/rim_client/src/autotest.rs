//! `rim --autotest [dir]` drives the real client through every control.
//!
//! It feeds synthetic raw input through the same `frame()` the game loop
//! uses: world clicks and drags pass through the UI's routing exactly as the
//! mouse's would, and UI controls are clicked by node id (`core:toolbar.
//! designate:chop`), not screen position. It checks the game state after
//! each step, saves screenshots to `dir` (default `target/autotest`), and
//! exits non-zero if any check failed.

use crate::{apply, draw, frame, render, Action, App, RawInput, Tool};
use macroquad::prelude::*;
use rim_sim::data::Data;
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::path::Goal;
use rim_sim::world::*;
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
        let path = self.dir.join(format!("{:02}_{name}.png", self.shots));
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
        let (x, y) = draw::pawn_pos(&self.pawn(e));
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
        self.input(RawInput { mouse: at, right_pressed: true, ..Default::default() }).await;
    }

    /// Press a key, then let the UI catch up: actions apply after the UI's
    /// frame, and trees rebuild at most every 50 ms without input.
    async fn key(&mut self, k: KeyCode) {
        let raw = RawInput { mouse: self.mouse, keys: vec![k], ..Default::default() };
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

    fn ui_text(&self) -> String {
        self.app.ui.snapshot()
    }

    fn count<Q: rim_sim::hecs::Query>(&self) -> usize {
        self.w().ecs.query::<Q>().iter().count()
    }
}

/// An open, empty square of `size` cells near `c`.
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
    let n_expected = 2 + defs.designations.len() + defs.things.iter().filter(|d| d.build.is_some()).count();
    t.check(keys.len() == n_expected, format!("one tool per designation and buildable def ({})", keys.len()));
    for k in &keys {
        let found = t.app.ui.find(&format!("core:toolbar.{k}")).is_some();
        t.check(found, format!("toolbar button for '{k}'"));
    }

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
    t.focus(tree.2);
    let (tx, ty) = t.screen(tree.2);
    t.input(RawInput { mouse: (tx, ty), ..Default::default() }).await;
    t.frame().await;
    let hint = order::resolve(t.w(), founder, tree.2, None).map(|o| o.label);
    t.check(hint.as_deref() == Some("Chop oak tree"), format!("the order resolves ({hint:?})"));
    t.check(t.app.ui.find("core:hint").is_some(), "the cursor label shows it (core:hint)");
    t.check(t.ui_text().contains("Chop oak tree"), "and says what the click will do");
    t.shot("order").await;

    t.right_click((tx, ty)).await;
    t.ticks(1); // commands apply on the next tick
    t.check(
        matches!(t.pawn(founder).job, Job::Harvest { target, .. } if target == tree.1),
        "right-click sends an undrafted colonist to chop",
    );
    t.check(t.app.order_flash.is_some(), "the order is acknowledged on the map");
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
    t.click_ui("core:toolbar.designate:chop").await;
    t.check(t.app.tool == Tool::Designate(chop), "clicking Chop selects the chop tool");
    t.drag(home.offset(-8, -8), home.offset(8, 8)).await;
    t.ticks(1);
    let designated = t.count::<(&Thing, &Designated)>();
    t.check(designated > 0, format!("dragging designates trees ({designated})"));
    let wrong = t.w().ecs.query::<(&Thing, &Designated)>().iter().filter(|(_, d)| d.0 != chop).count();
    t.check(wrong == 0, "only chop designations were made");

    let site = open_square(t.w(), home, 6).expect("open ground for a hut");
    let wall = defs.thing_id("wall").unwrap();
    t.click_ui("core:toolbar.build:wall").await;
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

    t.click_ui("core:toolbar.cancel").await;
    t.drag(site, site.offset(5, 0)).await;
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 14,
        format!("cancel removes the dragged row ({} left)", t.count::<&Blueprint>()),
    );
    t.click_ui("core:toolbar.build:wall").await;
    t.drag(site, site.offset(4, 0)).await;
    t.click_ui("core:toolbar.build:door").await;
    t.drag(site.offset(5, 0), site.offset(5, 0)).await;
    t.click_ui("core:toolbar.build:bed").await;
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
    for _ in 0..6000 {
        t.ticks(1);
        if !saw_interp {
            let p = t.pawn(founder);
            if p.next.is_some() && p.progress > 0 && p.progress < p.step_ticks {
                let (x, y) = draw::pawn_pos(&p);
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
        })
        .expect("a free row for three walls, with nothing at either end");
    for (i, m) in [wood, wood, stone].into_iter().enumerate() {
        t.app.sim.world.spawn_fixture_of(wall, row.offset(i as i32, 0), false, Some(m)).expect("a wall");
    }
    t.focus(row.offset(1, 0));
    let img = t.grab().await;
    let dpi = screen_dpi_scale();
    let z = t.app.cam.zoom;
    let px = |img: &Image, (x, y): (f32, f32)| {
        let (xi, yi) = ((x * dpi) as u32, (y * dpi) as u32);
        let c = img.get_pixel(xi.min(img.width() as u32 - 1), yi.min(img.height() as u32 - 1));
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

    // ---------------------------------------------------------- 0047 orders
    println!("\n# select, draft, move, attack (0047)");
    t.act(Action::Speed(1));
    t.focus(t.pawn(founder).pos);
    t.key(KeyCode::Escape).await;
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
    let spot = open_square(t.w(), site, 1).expect("room for a fire");
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
    if let Some(r) = t.ui_rect("core:toolbar.designate:chop") {
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
    let t0 = std::time::Instant::now();
    t.frame().await;
    let frame_ms = t0.elapsed().as_secs_f64() * 1e3;
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let calls = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    println!(
        "zoomed out ({:.1} px/cell): {calls} draw calls, frame {frame_ms:.1} ms (CPU, incl. present)",
        t.app.cam.zoom
    );
    t.check(calls > 0, "the renderer's draw calls can be counted");
    t.act(Action::Zoom(zoom0 / t.app.cam.zoom, 800.0, 480.0));
    t.frame().await;

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
        t.app.sky.cost_us,
        t.app.sky.particles(),
        t.app.sky.light_us
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
        t.app.sky.cost_us,
        t.app.sky.particles(),
        t.app.sky.light_us
    );
    t.app.sky.strike();
    t.shot("storm").await;
    for id in pins {
        let f = field(&t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }

    println!("\n{} passed, {} failed; screenshots in {}", t.passed, t.failed.len(), t.dir.display());
    for f in &t.failed {
        println!("  FAIL {f}");
    }
    std::process::exit(if t.failed.is_empty() { 0 } else { 1 });
}
