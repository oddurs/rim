//! How built things look: walls, joins, openings, roofs, rooms, fences, plans.

use super::*;

/// 0220 walls
pub(super) async fn walls(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before walls");
    let home = carry.home.expect("set before walls");
    let wall = carry.wall.expect("set before walls");
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
    let z = t.app.cam.zoom;
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
    carry.wood = Some(wood);
    carry.stone = Some(stone);
}

/// joins in quarters
pub(super) async fn joins_in_quarters(t: &mut T, carry: &mut Carry) {
    let home = carry.home.expect("set before joins_in_quarters");
    let wall = carry.wall.expect("set before joins_in_quarters");
    let wood = carry.wood.expect("set before joins_in_quarters");
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
    let post = slot.offset(1, 1);
    let fill = px(&img, at(t, post, 0.5, 0.5));
    let corner = px(&img, at(t, post, 2.0 / z, 2.0 / z));
    t.check(dist(corner, fill) > 0.12, format!("a lone post's corner is rounded off ({corner:?} vs fill {fill:?})"));
    // The corner shape's inner corner stays square: just inside it is wall.
    let l = slot.offset(4 * 2 + 1, 1);
    let inner = px(&img, at(t, l, 1.0 - 2.0 / z, 1.0 - 2.0 / z));
    t.check(dist(inner, fill) < 0.1, format!("an inner corner is square ({inner:?} vs fill {fill:?})"));
    // Where a run meets its neighbour there is no round and no seam.
    let run = slot.offset(4 + 1, 1);
    // Below the lit edge, and well inside where a round would cut.
    let joint = px(&img, at(t, run, 1.0 - 1.0 / z, 5.0 / z));
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
                best = best.max(px(&img, at(t, run, fx, fy)).iter().sum::<f32>());
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
    carry.slot = Some(slot);
    carry.zoom = Some(zoom);
}

/// openings turn to their wall
pub(super) async fn openings_turn_to_their_wall(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before openings_turn_to_their_wall");
    let wall = carry.wall.expect("set before openings_turn_to_their_wall");
    let wood = carry.wood.expect("set before openings_turn_to_their_wall");
    let slot = carry.slot.expect("set before openings_turn_to_their_wall");
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
    let jamb = px(&img, at(t, west, 0.5, 2.0 / z));
    let side = px(&img, at(t, west, 2.0 / z, 0.5));
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
                n += woody(px(&img, at(t, west, fx, fy))) as usize;
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
    carry.hut = Some(hut);
    carry.west = Some(west);
}

/// roofs from far away
pub(super) async fn roofs_from_far_away(t: &mut T, carry: &mut Carry) {
    let west = carry.west.expect("set before roofs_from_far_away");
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
}

/// room state
pub(super) async fn room_state(t: &mut T, carry: &mut Carry) {
    let wall = carry.wall.expect("set before room_state");
    let hut = carry.hut.expect("set before room_state");
    let wood = carry.wood.expect("set before room_state");
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
}

/// fences
pub(super) async fn fences(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before fences");
    let hut = carry.hut.expect("set before fences");
    let wood = carry.wood.expect("set before fences");
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
}

/// material patterns
pub(super) async fn material_patterns(t: &mut T, carry: &mut Carry) {
    let slot = carry.slot.expect("set before material_patterns");
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
    let body = px(&img, at(t, run_mid, 0.5, 0.5)).iter().sum::<f32>();
    let (before, across) = (darkest(t, &img, 0.96), darkest(t, &img, 1.04));
    t.check(
        before < body - 0.1 && across < body - 0.1,
        format!("a log course runs across the joint ({before:.2} and {across:.2} against the wood's {body:.2})"),
    );
    // Zoomed right out, a pattern would be noise: it isn't drawn.
    t.app.cam.zoom = 6.0;
    let img = t.grab().await;
    let row: Vec<f32> = (0..8).map(|i| px(&img, at(t, run_mid, 0.3 + i as f32 * 0.05, line_y)).iter().sum()).collect();
    let spread = row.iter().cloned().fold(f32::MIN, f32::max) - row.iter().cloned().fold(f32::MAX, f32::min);
    t.check(spread < 0.08, format!("no pattern zoomed out (brightness spread {spread:.2} across a wall)"));
    t.app.cam.zoom = 40.0;
    t.shot("patterns").await;
}

/// room labels
pub(super) async fn room_labels(t: &mut T, carry: &mut Carry) {
    let west = carry.west.expect("set before room_labels");
    let zoom = carry.zoom.expect("set before room_labels");
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
}

/// the plan style
pub(super) async fn the_plan_style(t: &mut T, carry: &mut Carry) {
    let home = carry.home.expect("set before the_plan_style");
    let wood = carry.wood.expect("set before the_plan_style");
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
}

/// house plans
pub(super) async fn house_plans(t: &mut T, carry: &mut Carry) {
    let home = carry.home.expect("set before house_plans");
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
}

/// 0215 materials
pub(super) async fn materials(t: &mut T, carry: &mut Carry) {
    let home = carry.home.expect("set before materials");
    let wall = carry.wall.expect("set before materials");
    let stone = carry.stone.expect("set before materials");
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
    carry.spot = Some(spot);
}
