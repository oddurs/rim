//! Designating, building, cancelling, orders, stockpiles and stances.

use super::*;

/// 0046 designate / build / cancel
pub(super) async fn designate_build_cancel(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before designate_build_cancel");
    let founder = carry.founder.expect("set before designate_build_cancel");
    let home = carry.home.expect("set before designate_build_cancel");
    println!("\n# designate, build, cancel (0046)");
    let chop = defs.lookup("designation", "chop").unwrap();
    t.click_tool("designate:core:chop").await;
    t.check(t.app.tool == Tool::Designate(chop), "clicking Chop selects the chop tool");
    // Drag over the trees nearest home, wherever this map put them: an oak
    // with another in the drag's box, so the drag has one to newly mark
    // after the first is marked beforehand.
    let oak = defs.thing_id("tree_oak").unwrap();
    let oaks: Vec<IVec> = t.w().ecs.query::<&Thing>().iter().filter(|th| th.def == oak).map(|th| th.pos).collect();
    let near_tree = oaks
        .iter()
        .copied()
        .filter(|p| oaks.iter().any(|q| q != p && q.chebyshev(*p) <= 4))
        .min_by_key(|p| (p.octile(home), p.x, p.y));
    let (a, b) = match near_tree {
        Some(p) => (p.offset(-4, -4), p.offset(4, 4)),
        None => (home.offset(-8, -8), home.offset(8, 8)),
    };
    // Hover and drag preview what the order will mark (bb769d00): bare
    // grass gets a faint frame and no hint; one tree marked beforehand
    // keeps its dot; each tree the drag will newly mark gets a ring.
    let bare = (2..10)
        .flat_map(|r| (-r..=r).map(move |d| home.offset(d, r)))
        .find(|&p| t.w().map.passable(p) && t.w().map.fixture_at(p).is_none() && t.w().map.item_at(p).is_none());
    if let Some(p) = bare {
        t.mouse = t.screen(p);
        t.frame().await;
        t.frame().await;
        let faint =
            crate::overlay::scene(&t.app).marks.iter().any(|m| matches!(m, Mark::Frame { alpha, .. } if *alpha < 1.0));
        let hint = crate::overlay::drag_hint(&t.app);
        t.check(faint && hint.is_none(), format!("chop over bare ground: a faint frame and no hint ({hint:?})"));
    }
    if let Some(p) = near_tree {
        t.app.sim.push(Command::Designate { designation: chop, a: p, b: p });
        t.ticks(1);
    }
    t.input(RawInput { mouse: t.screen(a), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: t.screen(b), ..Default::default() }).await;
    let fresh: Vec<Entity> = t
        .app
        .order_preview
        .as_ref()
        .map(|op| {
            op.targets
                .iter()
                .filter_map(|t| if let rim_sim::command::Target::Thing(e) = t { Some(*e) } else { None })
                .collect()
        })
        .unwrap_or_default();
    let rings =
        crate::overlay::scene(&t.app).marks.iter().filter(|m| matches!(m, Mark::Target { cancel: false, .. })).count();
    let hint = crate::overlay::drag_hint(&t.app);
    let before: Vec<Entity> = t.w().ecs.query::<(Entity, &Designated)>().iter().map(|(e, _)| e).collect();
    // The count, whatever this map's trees are called.
    let expect = format!("Chop · {} ", fresh.len());
    t.check(!fresh.is_empty(), "a drag over trees not yet marked, for the rings");
    t.check(
        rings == fresh.len()
            && hint.as_deref().is_some_and(|h| h.starts_with(&expect))
            && fresh.iter().all(|e| !before.contains(e)),
        format!("a chop drag rings only the trees it will newly mark ({rings} rings, {hint:?})"),
    );
    t.shot("chalk-designate").await;
    t.input(RawInput { mouse: t.screen(b), left_released: true, ..Default::default() }).await;
    t.ticks(1);
    let now_marked: Vec<Entity> = t.w().ecs.query::<(Entity, &Designated)>().iter().map(|(e, _)| e).collect();
    let gained: Vec<Entity> = now_marked.iter().copied().filter(|e| !before.contains(e)).collect();
    let (mut g, mut f) = (gained.clone(), fresh.clone());
    g.sort();
    f.sort();
    t.check(g == f, format!("and the drag marks exactly the ringed trees ({} of {})", gained.len(), fresh.len()));
    let designated = t.count::<(&Thing, &Designated)>();
    t.check(designated > 0 || near_tree.is_none(), format!("dragging designates trees ({designated})"));
    // A click on a tree already marked is refused, and says why.
    if let Some(p) = near_tree {
        t.click(t.screen(p)).await;
        t.frame().await;
        let why = crate::overlay::drag_hint(&t.app);
        t.check(why.as_deref() == Some("Already marked"), format!("a click on a marked tree says so ({why:?})"));
        // Moved and clicked in one frame: the refusal is for the cell
        // clicked, not the one the last frame's preview was made for.
        if let Some(q) = bare {
            t.input(RawInput { mouse: t.screen(q), ..Default::default() }).await;
            t.input(RawInput { mouse: t.screen(p), left_pressed: true, left_released: true, ..Default::default() })
                .await;
            let why = t.app.refused.as_ref().map(|r| r.0.clone());
            t.check(
                why.as_deref() == Some("Already marked"),
                format!("a click that moved says why for its cell ({why:?})"),
            );
        }
        // Dropping the tool takes its refusal with it.
        t.right_click(t.screen(p)).await;
        let why = crate::overlay::drag_hint(&t.app);
        t.check(
            t.app.tool == Tool::Select && t.app.refused.is_none(),
            format!("a right click drops the tool and its refusal ({why:?})"),
        );
        t.app.tool = Tool::Designate(chop);
    }
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
    // A wall drag ghosts exactly the cells that get walls, and the hint
    // counts them with their cost (e8f313d6).
    let ghosts = crate::overlay::scene(&t.app).marks.iter().filter(|m| matches!(m, Mark::Ghost { .. })).count();
    let hint = crate::overlay::drag_hint(&t.app).unwrap_or_default();
    t.check(
        ghosts == 20 && hint.starts_with("20 walls") && t.ui_text().contains("20 walls"),
        format!("a wall drag ghosts its 20 cells and counts them by the pointer ({ghosts}, {hint:?})"),
    );
    t.shot("chalk-build-run").await;
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
    // A run through a tree and over a plan: the tree is cleared first
    // (a triangle), the plan blocks (a cross), and the rest goes up. What's
    // ghosted is what's planned (e8f313d6).
    let oak = defs.thing_id("tree_oak").unwrap();
    let run = {
        let w = t.w();
        let plans: Vec<IVec> = w.ecs.query::<(&Thing, &Blueprint)>().iter().map(|(th, _)| th.pos).collect();
        plans.iter().find_map(|&b| {
            // Along its row or its column, whichever this map has a tree on.
            (2..12)
                .flat_map(|d| [b.offset(-d, 0), b.offset(d, 0), b.offset(0, -d), b.offset(0, d)])
                .find(|&q| w.map.fixture_at(q).and_then(|f| w.thing(f)).is_some_and(|th| th.def == oak))
                .map(|q| (q, b))
        })
    };
    match run {
        Some((tree_at, plan_at)) => {
            t.click_tool("build:core:wall").await;
            t.focus(IVec::new((tree_at.x + plan_at.x) / 2, (tree_at.y + plan_at.y) / 2));
            t.input(RawInput { mouse: t.screen(tree_at), left_pressed: true, ..Default::default() }).await;
            t.input(RawInput { mouse: t.screen(plan_at), ..Default::default() }).await;
            let marks = crate::overlay::scene(&t.app).marks;
            let clears = marks.iter().filter(|m| matches!(m, Mark::Ghost { clears: true, .. })).count();
            let blocked = marks.iter().filter(|m| matches!(m, Mark::Blocked { .. })).count();
            let hint = crate::overlay::drag_hint(&t.app).unwrap_or_default();
            let (up, asked) = t.app.build_preview.as_ref().map_or((0, 0), |bp| bp.counts());
            let want = format!("{up} of {asked} walls");
            t.check(
                clears >= 1 && blocked >= 1 && hint.starts_with(&want),
                format!("a run through a tree and a plan: a triangle, a cross, and '{want}' ({clears}, {blocked}, {hint:?})"),
            );
            t.shot("chalk-build-blocked").await;
            let open: Vec<IVec> = t.app.build_preview.as_ref().map_or(Vec::new(), |bp| {
                bp.cells.iter().filter(|(_, pl)| *pl == rim_sim::command::Place::Open).map(|(p, _)| *p).collect()
            });
            let before: Vec<IVec> = t.w().ecs.query::<(&Thing, &Blueprint)>().iter().map(|(th, _)| th.pos).collect();
            t.input(RawInput { mouse: t.screen(plan_at), left_released: true, ..Default::default() }).await;
            t.ticks(1);
            let mut fresh: Vec<IVec> = t
                .w()
                .ecs
                .query::<(&Thing, &Blueprint)>()
                .iter()
                .map(|(th, _)| th.pos)
                .filter(|p| !before.contains(p))
                .collect();
            let mut open = open;
            fresh.sort_by_key(|p| (p.x, p.y));
            open.sort_by_key(|p| (p.x, p.y));
            let marked = t.w().map.fixture_at(tree_at).is_some_and(|f| t.w().ecs.get::<&Planned>(f).is_ok());
            t.check(
                fresh == open && marked,
                format!(
                    "the plans went up where the ghosts were, and the tree waits to be felled ({} of {})",
                    fresh.len(),
                    open.len()
                ),
            );
            // Take the run back, so the counts that follow are the hut's.
            for &p in fresh.iter().chain([tree_at].iter()) {
                t.app.sim.push(Command::Cancel { a: p, b: p });
            }
            t.ticks(1);
            t.key(KeyCode::Escape).await;
        }
        None => t.check(false, "a tree on a row with a plan, for a run through both"),
    }
    // A thing bigger than a cell ghosts its footprint, and T turns it.
    if let Some(pile) = defs.thing_id("primitive:woodpile").or_else(|| defs.thing_id("woodpile")) {
        let spot = open_square(t.w(), home, 3).expect("open ground for a woodpile");
        t.focus(spot);
        t.app.tool = Tool::Build(pile);
        t.mouse = t.screen(spot);
        let size = |t: &T| {
            crate::overlay::scene(&t.app).marks.iter().find_map(|m| {
                if let Mark::Ghost { rect, facing: Some(_), .. } = m {
                    Some((rect[2].round(), rect[3].round()))
                } else {
                    None
                }
            })
        };
        t.frame().await;
        t.frame().await;
        let before = size(t);
        t.key(KeyCode::T).await;
        t.frame().await;
        let after = size(t);
        t.check(
            before.zip(after).is_some_and(|(b, a)| b.0 == a.1 && b.1 == a.0 && b.0 != b.1),
            format!("T turns a woodpile's ghost ({before:?} to {after:?})"),
        );
        t.app.build_facing = 0;
        t.app.tool = Tool::Select;
    } else {
        t.check(false, "a woodpile, for a footprint bigger than a cell");
    }
    // A bed over a table: blocked, and the hint names what's in the way.
    let (bed, table) = (defs.thing_id("bed").expect("beds"), defs.thing_id("table").expect("tables"));
    let spot = open_square(t.w(), home, 3).expect("open ground for a table");
    match t.app.sim.world.spawn_fixture(table, spot, false) {
        Some(e) => {
            t.focus(spot);
            t.app.tool = Tool::Build(bed);
            t.mouse = t.screen(spot);
            t.frame().await;
            t.frame().await;
            let crossed = crate::overlay::scene(&t.app).marks.iter().any(|m| matches!(m, Mark::Blocked { .. }));
            let hint = crate::overlay::drag_hint(&t.app);
            t.check(
                crossed && hint.as_deref() == Some("Blocked by a table"),
                format!("a bed's ghost over a table is blocked, and says by what ({crossed}, {hint:?})"),
            );
            t.app.tool = Tool::Select;
            t.app.sim.world.despawn_thing(e);
        }
        None => t.check(false, "open ground for a table"),
    }
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
    // Turns are drawn round inside the corner cell (DESIGN.md §6h): the
    // drawn founder never leaves the cells the sim has them between.
    let (mut saw_round, mut strayed) = (false, None);
    // Long enough to chop, haul and build on any map: stop early once a wall
    // stands and the walking interpolation has been seen.
    let built_walls =
        |t: &T| t.w().ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|th| th.def == wall).count();
    for i in 0..12000 {
        if i >= 6000 && saw_interp && saw_round && built_walls(t) > 0 {
            break;
        }
        t.ticks(1);
        let p = t.pawn(founder);
        let Some(n) = p.next else { continue };
        let (x, y) = draw::pawn_pos(&t.app, founder, &p);
        if !saw_interp && p.progress > 0 && p.progress < p.step_ticks {
            saw_interp = (x.fract() - 0.5).abs() > 1e-3 || (y.fract() - 0.5).abs() > 1e-3;
        }
        let (lo, hi) = ((p.pos.x.min(n.x), p.pos.y.min(n.y)), (p.pos.x.max(n.x) + 1, p.pos.y.max(n.y) + 1));
        let within = (lo.0 as f32..=hi.0 as f32).contains(&x) && (lo.1 as f32..=hi.1 as f32).contains(&y);
        if !within && strayed.is_none() {
            strayed = Some(format!("({x:.2}, {y:.2}) stepping {:?} to {n:?}", p.pos));
        }
        let (lx, ly) = p.drawn_at(t.app.tick_frac());
        if !saw_round && (x - lx).abs().max((y - ly).abs()) > 0.02 {
            saw_round = true;
            let (cam, paused) = ((t.app.cam.x, t.app.cam.y, t.app.cam.zoom), t.app.paused);
            (t.app.cam.x, t.app.cam.y, t.app.cam.zoom, t.app.paused) = (x, y, 56.0, true);
            t.shot("round-turn").await;
            (t.app.cam.x, t.app.cam.y, t.app.cam.zoom, t.app.paused) = (cam.0, cam.1, cam.2, paused);
        }
    }
    t.check(saw_interp, "pawns are drawn between cells while walking");
    t.check(saw_round, "a turn is drawn round, off the straight steps");
    t.check(strayed.is_none(), format!("the drawn pawn stays in the cells it steps between ({strayed:?})"));

    // --------------------------------------------------- 535a1fb9 bodies
    println!("\n# pawns are plan figures, in one batch at three levels of detail (DESIGN.md §6h)");
    let (cam, paused) = ((t.app.cam.x, t.app.cam.y, t.app.cam.zoom), t.app.paused);
    t.app.paused = true;
    let at = t.pawn(founder).pos;
    t.focus(at);
    for (z, name) in [(7.0, "figures-dot"), (15.0, "figures-silhouette"), (40.0, "figures-full")] {
        t.app.cam.zoom = z;
        t.shot(name).await;
        let calls = t.app.figures.calls;
        t.check(calls == 1, format!("every pawn's figure is one draw call at {z} points a cell ({calls})"));
        // Handing the batch to GL is submission, as the chunk meshes' is:
        // on a software rasteriser it is drawing, not building the pass.
        let (gl, sent) = (t.app.render_us.gl, t.app.figures.gl_us);
        t.check(
            sent > 0.0 && gl >= sent,
            format!("the figures' GL time is counted as submission ({sent:.0} of {gl:.0} µs)"),
        );
    }
    (t.app.cam.x, t.app.cam.y, t.app.cam.zoom, t.app.paused) = (cam.0, cam.1, cam.2, paused);
    let built = t.w().ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|th| th.def == wall).count();
    t.check(built > 0, format!("the warrior chopped and built walls ({built})"));
    t.focus(site.offset(3, 3));
    t.shot("building").await;
    carry.site = Some(site);
    carry.wall = Some(wall);
}

/// 0047 orders
pub(super) async fn orders(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before orders");
    let founder = carry.founder.expect("set before orders");
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
}

/// ecd54de8 stockpiles
pub(super) async fn stockpiles(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before stockpiles");
    let home = carry.home.expect("set before stockpiles");
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
    let before = count(t);
    let (a, b) = (spot.offset(0, 2), spot.offset(1, 6));
    t.input(RawInput { mouse: t.screen(a), left_pressed: true, ..Default::default() }).await;
    t.input(RawInput { mouse: t.screen(b), ..Default::default() }).await;
    let chip = crate::overlay::drag_hint(&t.app);
    t.shot("chalk-zone-paint").await;
    t.input(RawInput { mouse: t.screen(b), left_released: true, ..Default::default() }).await;
    t.ticks(1);
    let gained = count(t) - before;
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
    // A cell of the zone with nothing on it: a click on what was hauled
    // there, or who stands there, picks that instead.
    let bare = (0..3).flat_map(|y| (0..4).map(move |x| spot.offset(x, y))).find(|&p| {
        let w = t.w();
        w.zones.at(&w.map, p).map(|z| z.id) == zone
            && w.map.item_at(p).is_none()
            && w.map.fixture_at(p).is_none()
            && w.pawns.iter().all(|&e| w.pawn_pos(e) != Some(p))
    });
    t.click(t.screen(bare.unwrap_or(spot.offset(1, 1)))).await;
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
}

/// 0cb48faf stances
pub(super) async fn stances(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before stances");
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
}
