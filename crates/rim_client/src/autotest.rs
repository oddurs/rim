//! `rim --autotest [dir]` drives the real client through every Castaway
//! control, using the same `Action`s that keyboard and mouse input produce.
//! It checks the game state after each step, saves screenshots to `dir`
//! (default `target/autotest`), and exits non-zero if any check failed.

use crate::{apply, draw, toolbar_y, Action, App, Tool};
use macroquad::prelude::*;
use rim_sim::hecs::Entity;
use rim_sim::world::*;
use rim_sim::IVec;
use std::path::PathBuf;

struct T {
    app: App,
    dir: PathBuf,
    shots: usize,
    passed: usize,
    failed: Vec<String>,
}

impl T {
    async fn frame(&mut self) {
        draw::world(&self.app);
        draw::hud(&self.app);
        next_frame().await;
    }

    async fn shot(&mut self, name: &str) {
        draw::world(&self.app);
        draw::hud(&self.app);
        let img = get_screen_data();
        self.shots += 1;
        let path = self.dir.join(format!("{:02}_{name}.png", self.shots));
        img.export_png(path.to_str().unwrap());
        println!("shot  {}", path.display());
        next_frame().await;
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

    fn click(&mut self, (x, y): (f32, f32)) {
        self.act(Action::LeftDown(x, y));
        self.act(Action::LeftUp(x, y));
    }

    fn click_button(&mut self, label: &str) {
        let b = self.app.buttons.iter().find(|b| b.label == label).unwrap_or_else(|| panic!("no button {label}"));
        let at = (b.rect.x + b.rect.w / 2.0, toolbar_y() + b.rect.h / 2.0);
        self.click(at);
    }

    fn drag(&mut self, a: IVec, b: IVec) {
        let (ax, ay) = self.screen(a);
        let (bx, by) = self.screen(b);
        self.act(Action::LeftDown(ax, ay));
        self.act(Action::LeftUp(bx, by));
    }

    fn count<Q: rim_sim::hecs::Query>(&self) -> usize {
        self.w().ecs.query::<Q>().iter().count()
    }

    fn focus(&mut self, p: IVec) {
        self.app.cam.x = p.x as f32 + 0.5;
        self.app.cam.y = p.y as f32 + 0.5;
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
    let mut t = T { app, dir, shots: 0, passed: 0, failed: Vec::new() };
    for _ in 0..3 {
        t.frame().await;
    }
    let defs = t.w().defs.clone();
    let founder = t.w().colonists().next().expect("a founder");
    let home = t.pawn(founder).pos;
    t.shot("start").await;

    // ---------------------------------------------------------- 0046 toolbar
    println!("\n# toolbar (0046)");
    let mut expected = vec!["Select".to_string()];
    expected.extend(defs.designations.iter().map(|d| d.label.clone()));
    expected.extend(defs.things.iter().filter(|d| d.build.is_some()).map(|d| d.label.clone()));
    expected.push("Cancel".into());
    let labels: Vec<String> = t.app.buttons.iter().map(|b| b.label.clone()).collect();
    t.check(labels.len() == expected.len(), format!("one button per designation and buildable def ({})", labels.len()));
    for e in &expected {
        t.check(labels.contains(e), format!("button for '{e}'"));
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

    // ---------------------------------------------------------- 0046 designate / build / cancel
    println!("\n# designate, build, cancel (0046)");
    let chop = defs.lookup("designation", "chop").unwrap();
    t.click_button("Chop");
    t.check(t.app.tool == Tool::Designate(chop), "clicking Chop selects the chop tool");
    t.drag(home.offset(-8, -8), home.offset(8, 8));
    t.ticks(1);
    let designated = t.count::<(&Thing, &Designated)>();
    t.check(designated > 0, format!("dragging designates trees ({designated})"));
    let wrong = t.w().ecs.query::<(&Thing, &Designated)>().iter().filter(|(_, (_, d))| d.0 != chop).count();
    t.check(wrong == 0, "only chop designations were made");

    let site = open_square(t.w(), home, 6).expect("open ground for a hut");
    let wall = defs.thing_id("wall_wood").unwrap();
    t.click_button("wooden wall");
    t.check(t.app.tool == Tool::Build(wall), "clicking wooden wall selects the wall tool");
    t.drag(site, site.offset(5, 5));
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 20,
        format!("wall drag places the 20-cell outline ({})", t.count::<&Blueprint>()),
    );
    t.check(t.w().map.fixture_at(site.offset(2, 2)).is_none(), "the inside of the outline stays empty");

    t.click_button("Cancel");
    t.drag(site, site.offset(5, 0));
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 14,
        format!("cancel removes the dragged row ({} left)", t.count::<&Blueprint>()),
    );
    t.click_button("wooden wall");
    t.drag(site, site.offset(4, 0));
    t.click_button("wooden door");
    t.drag(site.offset(5, 0), site.offset(5, 0));
    t.click_button("bed");
    t.drag(site.offset(2, 2), site.offset(2, 2));
    t.ticks(1);
    t.check(
        t.count::<&Blueprint>() == 21,
        format!("walls, a door and a bed are planned ({})", t.count::<&Blueprint>()),
    );
    t.act(Action::RightClick(600.0, 500.0));
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
    let built = t.w().ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|(_, th)| th.def == wall).count();
    t.check(built > 0, format!("the warrior chopped and built walls ({built})"));

    // Force a bush into regrowth so all three overlays are on screen.
    let bush = defs.thing_id("berry_bush").unwrap();
    let near_bush = t
        .w()
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|(_, th)| th.def == bush)
        .min_by_key(|(e, th)| (th.pos.octile(site), e.id()))
        .map(|(e, th)| (e, th.pos));
    if let Some((b, _)) = near_bush {
        let ready_at = t.w().tick + 100_000;
        let _ = t.app.sim.world.ecs.insert_one(b, Regrow { ready_at });
    }
    t.focus(site.offset(3, 3));
    t.shot("building").await;
    if let Some((_, bp)) = near_bush {
        t.focus(bp);
        t.shot("regrowing_bush").await;
    }

    // ---------------------------------------------------------- 0047 orders
    println!("\n# select, draft, move, attack (0047)");
    t.act(Action::Speed(1));
    t.focus(t.pawn(founder).pos);
    t.act(Action::Escape);
    t.act(Action::Escape);
    t.check(t.app.selected.is_none(), "escape clears the selection");
    let at = t.pawn_screen(founder);
    t.click(at);
    t.check(t.app.selected == Some(founder), "clicking a colonist selects them");
    t.act(Action::ToggleDraft);
    t.ticks(1);
    t.check(t.pawn(founder).drafted, "R drafts the selected colonist");

    let here = t.pawn(founder).pos;
    let dest = (3..12)
        .flat_map(|r| [here.offset(r, 0), here.offset(-r, 0), here.offset(0, r), here.offset(0, -r)])
        .find(|p| t.w().map.passable(*p) && t.w().map.region_at(*p) == t.w().map.region_at(here))
        .expect("somewhere to walk");
    let (dx, dy) = t.screen(dest);
    t.act(Action::RightClick(dx, dy));
    t.ticks(1);
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
    t.act(Action::RightClick(rs.0, rs.1));
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
    t.act(Action::ToggleDraft);
    t.ticks(2);
    t.check(!t.pawn(founder).drafted, "R again undrafts");

    // ---------------------------------------------------------- 0048 HUD
    println!("\n# messages, colonist bar, clock, speed (0048)");
    t.act(Action::TogglePause);
    t.check(t.app.paused, "space pauses");
    t.act(Action::TogglePause);
    t.check(!t.app.paused, "space resumes");
    for (s, want) in [(1, 1), (3, 3), (6, 6)] {
        t.act(Action::Speed(s));
        t.check(t.app.speed == want, format!("speed key sets {want}x"));
    }
    let clock = draw::clock_text(&t.app);
    t.check(clock.starts_with(&format!("Day {}", t.w().day() + 1)), format!("top bar shows the day: '{clock}'"));
    let hh = t.w().hour() as u32;
    t.check(clock.contains(&format!("{hh:02}:")), "top bar shows the hour");
    t.check(clock.contains("6x"), "top bar shows the speed");

    let kinds = [MsgKind::Info, MsgKind::Good, MsgKind::Threat, MsgKind::Bad];
    for k in kinds {
        t.app.sim.world.message(format!("autotest {k:?} message"), k);
    }
    let colors: Vec<[u8; 4]> = kinds.iter().map(|k| draw::message_color(*k).into()).collect();
    let distinct = (0..4).all(|i| (0..4).all(|j| i == j || colors[i] != colors[j]));
    t.check(distinct, "each message kind has its own colour");

    let cols: Vec<Entity> = t.w().colonists().collect();
    t.act(Action::Escape);
    t.act(Action::Escape);
    if let Some((e, r)) = draw::colonist_rects(&t.app).into_iter().next() {
        t.click((r.x + r.w / 2.0, r.y + r.h / 2.0));
        t.check(t.app.selected == Some(e), "clicking the colonist bar selects that colonist");
    }
    t.act(Action::NextColonist);
    t.check(t.app.selected.is_some_and(|s| cols.contains(&s)), "tab cycles colonists");
    t.shot("messages").await;

    // ---------------------------------------------------------- 0049 profiler
    println!("\n# profiler (0049)");
    t.act(Action::ToggleProfiler);
    t.check(t.app.show_profiler, "F3 opens the profiler");
    t.ticks(1200);
    let names: Vec<String> = t.app.sim.profile.entries.iter().map(|e| e.0.clone()).collect();
    for sys in ["tick", "pawns", "needs", "regions", "rooms", "wealth"] {
        t.check(names.iter().any(|n| n == sys), format!("profiler times system '{sys}'"));
    }
    t.check(names.iter().any(|n| n == "mod:core"), "profiler times core's scripts");
    t.check(t.w().pf.searches > 0 && t.w().pf.expanded > 0, "pathfinder searches and nodes are counted");
    t.shot("profiler").await;
    t.act(Action::ToggleProfiler);

    // Night, to see lighting.
    while !(22.0..23.0).contains(&t.w().hour()) {
        t.ticks(100);
    }
    t.focus(site.offset(3, 3));
    t.shot("night").await;

    println!("\n{} passed, {} failed; screenshots in {}", t.passed, t.failed.len(), t.dir.display());
    for f in &t.failed {
        println!("  FAIL {f}");
    }
    std::process::exit(if t.failed.is_empty() { 0 } else { 1 });
}
