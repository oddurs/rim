//! Indoors, and roofs in the sun.

use super::*;

/// 2f13e01d indoors
pub(super) async fn indoors(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before indoors");
    let site = carry.site.expect("set before indoors");
    let wall = carry.wall.expect("set before indoors");
    println!("\n# indoors: sunbeams through windows, and a room lit to its corners (2f13e01d)");
    t.app.paused = true;
    let (cloud, light) = (field(t, "cloud"), field(t, "light"));
    t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
    t.app.sim.world.fields.set_ambient(light, Some(100.0));
    let (window, stove) = (defs.thing_id("window").unwrap(), defs.thing_id("stove").unwrap());
    let deep = defs.lookup("terrain", "deep_water").unwrap();
    let mut placed = Vec::new();
    let mut flooded = Vec::new();
    // A free square `side` across, the nearest to `near`, with a cell of
    // free ground round it so one room doesn't wall in another, and `open`
    // cells more to the west, where a low sun comes in.
    let square = |w: &World, near: IVec, side: i32, open: i32| {
        let free =
            |p: IVec| dry(w, p) && w.map.passable(p) && w.map.fixture_at(p).is_none() && w.map.item_at(p).is_none();
        (0..80i32).flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| near.offset(dx, dy)))).find(|o| {
            // West of it, only what casts a shadow matters, and plants
            // are cleared.
            let low = |p: IVec| {
                w.map.inb(p)
                    && w.solid_at(p).is_none()
                    && w.map.fixture_at(p).and_then(|f| w.thing(f)).is_none_or(|th| w.defs.thing(th.def).natural)
            };
            (-1..=side)
                .all(|y| (-1..=side).all(|x| free(o.offset(x, y))) && (-1 - open..-1).all(|x| low(o.offset(x, y))))
        })
    };
    // A ring of walls `side` across, the middle of its west wall a window,
    // deep water or wall as asked, and a stove in the middle if asked.
    let mut build = |t: &mut T, near: IVec, side: i32, west: &str, lit: bool| {
        // A window's low sun needs the ground west of it clear: 1 / tan 12°
        // is 4.7 cells of a wall's shadow.
        let open = if west == "window" { 6 } else { 0 };
        let o = square(t.w(), near, side, open)?;
        for p in (0..side).flat_map(|y| (-1 - open..-1).map(move |x| o.offset(x, y))) {
            if let Some(e) = t.w().map.fixture_at(p) {
                t.app.sim.world.despawn_thing(e);
            }
        }
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
    // The window last, so no other room stands in its sun.
    let dark = build(t, site.offset(-24, 20), 5, "wall", true);
    let hall = build(t, site.offset(-34, 12), 7, "wall", true);
    let moat = build(t, site.offset(-34, 22), 5, "water", false);
    let beamed = build(t, site.offset(-24, 12), 5, "window", false);
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
        let (small, big) = (corner(t, dark, 5), corner(t, hall, 7));
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
        let before = glow(t, dark.offset(1, 1));
        let door = t.w().map.fixture_at(dark.offset(0, 2));
        if let Some(e) = door {
            placed.retain(|&p| p != e);
            t.app.sim.world.despawn_thing(e);
        }
        let bakes = t.app.light.bakes;
        t.ticks(2);
        t.light_settles().await;
        let after = glow(t, dark.offset(1, 1));
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
    carry.light = Some(light);
    carry.cloud = Some(cloud);
}

/// 153dda59 roofs take the sun
pub(super) async fn roofs_take_the_sun(t: &mut T, carry: &mut Carry) {
    let site = carry.site.expect("set before roofs_take_the_sun");
    let wall = carry.wall.expect("set before roofs_take_the_sun");
    let light = carry.light.expect("set before roofs_take_the_sun");
    let cloud = carry.cloud.expect("set before roofs_take_the_sun");
    let calm = carry.calm.expect("set before roofs_take_the_sun");
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
            slopes.push((name, slope(t, 6), slope(t, 1)));
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
}
