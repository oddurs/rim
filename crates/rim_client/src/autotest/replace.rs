//! Replacing a piece in place.

use super::*;

/// replace in place
pub(super) async fn replace_in_place(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before replace_in_place");
    let founder = carry.founder.expect("set before replace_in_place");
    let home = carry.home.expect("set before replace_in_place");
    let wall = carry.wall.expect("set before replace_in_place");
    let hut = carry.hut.expect("set before replace_in_place");
    let stone = carry.stone.expect("set before replace_in_place");
    let spot = carry.spot.expect("set before replace_in_place");
    println!("\n# a wall planned over another is drawn hatched over it until they swap (DESIGN.md §6c)");
    let north = hut.offset(2, 0);
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
    let before = blueness(&img, t);
    t.app.sim.push(Command::Build { thing: wall, stuff: Some(stone), a: north, b: north, facing: 0 });
    t.ticks(1);
    let img = t.grab().await;
    let after = blueness(&img, t);
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
            let (cloud, light) = (field(t, "cloud"), field(t, "light"));
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
                let lit_below = glow(t);
                t.shot("light_below").await;
                t.key(KeyCode::RightBracket).await;
                t.light_settles().await;
                let lit_above = glow(t);
                // Up top, only what comes up the stairwell and the pits, at
                // most half (1104bf12).
                t.check(
                    lit_below > 0.3 && lit_above < lit_below * 0.5,
                    format!("a fire below lights its own level ({lit_below:.2}), the one above less ({lit_above:.2})"),
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
            println!("\n# light crosses a stairwell (1104bf12)");
            // A campfire beside the head of the stairs, up top: down the
            // stairwell, the level below is lit at its foot, less further off.
            let head = (-1..=1i32)
                .flat_map(|dy| (-1..=1i32).map(move |dx| top.offset(dx, dy)))
                .find(|&p| p != top && t.w().map.passable(p) && t.w().map.fixture_at(p).is_none());
            let fire = head.and_then(|p| t.app.sim.world.spawn_fixture(campfire, p, false));
            if let (Some(_), Some(e)) = (head, fire) {
                t.key(KeyCode::LeftBracket).await;
                t.light_settles().await;
                let glow = |t: &T, p: IVec| {
                    t.app.light.fire_at(p.x as f32 + 0.5, p.y as f32 + 0.5).map_or(0.0, |c| c.iter().sum::<f32>())
                };
                // Along the floor below, away from the stair's foot, the way
                // that keeps clear of every other opening, the pits.
                let openings: Vec<IVec> = t.w().map.air_cells(0).iter().map(|&i| t.w().map.pos(i as usize)).collect();
                let clear = |d: (i32, i32)| {
                    (1..=4).all(|k| {
                        let p = top.offset(d.0 * k, d.1 * k);
                        openings.iter().all(|o| (o.x - p.x).abs().max((o.y - p.y).abs()) > 6)
                    })
                };
                let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().find(|&d| clear(d)).unwrap_or((1, 0));
                let away: Vec<f32> = (0..3).map(|k| glow(t, top.offset(dx * 2 * k, dy * 2 * k))).collect();
                t.shot("stairwell_below").await;
                t.check(
                    away[0] > 0.1 && away[0] > away[1] && away[1] >= away[2],
                    format!("a fire at the head of the stairs lights their foot below, fading off ({away:.2?})"),
                );
                t.key(KeyCode::RightBracket).await;
                t.light_settles().await;
                t.shot("stairwell_above").await;
                t.app.sim.world.despawn_thing(e);
                t.key(KeyCode::LeftBracket).await;
                t.light_settles().await;
                let gone = glow(t, top);
                t.check(gone < 0.05, format!("and it was that fire: without it the foot is dark ({gone:.2})"));
                t.key(KeyCode::RightBracket).await;
                t.light_settles().await;
            } else {
                t.check(false, "room by the head of the stairs for a fire");
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
            // The world's brightness, in the middle of the screen: the panels
            // round its edges change with the level at once, and they are not
            // its light.
            let world = |img: &Image| {
                let (w, h) = (img.width(), img.height());
                let (mut sum, mut n) = (0.0f32, 0usize);
                for y in h * 3 / 10..h * 7 / 10 {
                    for x in w * 3 / 10..w * 7 / 10 {
                        let c = &img.bytes[(y * w + x) * 4..(y * w + x) * 4 + 3];
                        sum += (c[0] as f32 + c[1] as f32 + c[2] as f32) / 765.0;
                        n += 1;
                    }
                }
                sum / n.max(1) as f32
            };
            let mut settled = Vec::new();
            let mut steps = Vec::new();
            // Down, up, and down then straight back up in the middle of the
            // fade: the mix on screen fades out, it doesn't jump.
            let (down, up) = (KeyCode::LeftBracket, KeyCode::RightBracket);
            for keys in [vec![down], vec![up], vec![down, up]] {
                t.app.light.adapt_now();
                t.light_settles().await;
                let mut last = world(&t.grab().await);
                settled.push(last);
                let presses = keys.len();
                for (n, k) in keys.into_iter().enumerate() {
                    let pressed = crate::key_name(k).map(|n| vec![n.to_string()]).unwrap_or_default();
                    t.input(RawInput { mouse: t.mouse, keys: vec![k], pressed, ..Default::default() }).await;
                    // Another press to come: it lands two grabs in. A grab draws
                    // two frames, and the first after a press three, with the
                    // press's own: a frame's change is the grab's share.
                    for g in 0..if n + 1 < presses { 2 } else { 40 } {
                        let now = world(&t.grab().await);
                        steps.push((now - last).abs() / if g == 0 { 3.0 } else { 2.0 });
                        last = now;
                    }
                }
            }
            let bright = settled.iter().copied().fold(0.0f32, f32::max).max(1e-3);
            let (at, worst) =
                steps
                    .iter()
                    .map(|s| s / bright)
                    .enumerate()
                    .fold((0, 0.0f32), |(a, w), (k, s)| if s > w { (k, s) } else { (a, w) });
            // Which switch and which grab: 0 to 39 down, 40 to 79 up, then
            // down and back up; the first grab of each is the frame it lands.
            t.check(
                worst < 0.1,
                format!(
                    "changing level, no frame moves the brightness a tenth ({:.0}% at most, grab {at})",
                    worst * 100.0
                ),
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
                    pit(t, o, deep);
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
        let off = bubble_offset(t, founder);
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
}
