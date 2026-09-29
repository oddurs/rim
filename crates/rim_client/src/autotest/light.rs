//! Sun, fire and moving lights, and the lighting presets.

use super::*;

/// 8f4f1de8 sun shadows
pub(super) async fn sun_shadows(t: &mut T, carry: &mut Carry) {
    let site = carry.site.expect("set before sun_shadows");
    let wall = carry.wall.expect("set before sun_shadows");
    println!("\n# the sun casts shadows, and a still sun costs nothing (8f4f1de8)");
    // Calm, through every lighting check to come: the storm above would go
    // on flashing, and a flash lights the ground from where its bolt is.
    let calm = [field(t, "precipitation"), field(t, "wind")];
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
            dry(w, c) && w.map.passable(c) && w.map.fixture_at(c).is_none() && w.solid_at(c).is_none()
        })
    };
    // Away from any fire: at night its light on the ground under the wall
    // would cancel the contact shadow read there.
    let lights: Vec<IVec> = t
        .w()
        .defs
        .lookup("field", "light")
        .map(|f| t.w().fields.emitters_of(f as usize).map(|(_, q, _, _)| q).collect())
        .unwrap_or_default();
    let column = (0..60i32)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| site.offset(dx, dy))))
        .find(|&p| clear(t.w(), p) && lights.iter().all(|q| q.chebyshev(p) > 10));
    if let Some(p) = column {
        let standing = t.app.sim.world.spawn_fixture(wall, p, false);
        t.check(standing.is_some(), "the test wall stands");
        // Full daylight under a clear sky, whatever the hour the test has
        // reached: cloud softens a shadow's edge, and this is its length.
        let (cloud, light) = (field(t, "cloud"), field(t, "light"));
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
        let dark = shaded(t, &low) / shaded(t, &high).max(1e-3);
        t.check(dark < 0.8, format!("on screen the shadow falls north of the wall, away from the sun ({dark:.2})"));
        t.app.light.pin_sun = Some((90.0, 12.0));
        t.shot("sun_shadows").await;
        // The plan's contact shadow (0779def9): just under the wall at night,
        // gone where the sun reaches. The same spot is compared with ground
        // further south in each light, so the ground's own colour cancels out.
        t.focus(p.offset(0, 2));
        let (under, far) = (spots(t, p.offset(0, 1), 0.1), spots(t, p.offset(0, 4), 0.1));
        t.app.light.pin_sun = Some((90.0, 60.0));
        let day = t.grab().await;
        t.app.light.pin_sun = Some((90.0, -10.0));
        let night = t.grab().await;
        let contact = ratio(&night, under, far) / ratio(&day, under, far).max(1e-3);
        t.check(
            (0.6..0.9).contains(&contact),
            format!("the plan's contact shadow darkens under a wall at night and goes in the sun ({contact:.2})"),
        );
        // 1a17d685: a mod's green moon, applied as the loader applies a
        // mod's defs: its daylight term compiled, its body resolved, and
        // nothing in the engine or here that knows it. It chooses to light
        // the sim too, as core's moon doesn't. At midnight it outshines
        // core's moon at any phase, takes the one shadow slot the preset
        // has, and colours the night.
        println!("\n# a mod's green moon lights the night green and casts its own shadows (1a17d685)");
        t.app.light.pin_sun = None;
        t.app.sim.world.fields.set_ambient(light, None);
        // On to the next midnight (tick 0 is 06:00), a few hours at most.
        // Time only runs forward: what later sections keep by the tick
        // would take a clock set back for one that stopped.
        let (tick, tpd) = (t.w().tick, rim_sim::TICKS_PER_DAY);
        t.app.sim.world.tick = (tick + tpd / 4).div_ceil(tpd) * tpd - tpd / 4;
        t.ticks(20);
        t.app.light.adapt_now();
        t.focus(p.offset(0, -2));
        // The mean colour of the view's middle, as fractions of its sum.
        let hue = |img: &Image| {
            let (w, h) = (img.width() as u32, img.height() as u32);
            let mut c = [0.0f32; 3];
            for y in (h * 3 / 10..h * 7 / 10).step_by(7) {
                for x in (w * 3 / 10..w * 7 / 10).step_by(7) {
                    let px = img.get_pixel(x, y);
                    c = [c[0] + px.r, c[1] + px.g, c[2] + px.b];
                }
            }
            let sum = c.iter().sum::<f32>().max(1e-3);
            c.map(|v| v / sum)
        };
        let core_night = hue(&t.grab().await);
        // The world gets defs of its own with the mod's in them, and the
        // core-only ones back after.
        let before = t.app.sim.world.defs.clone();
        let applied = (|| -> Result<(), String> {
            let defs = std::sync::Arc::make_mut(&mut t.app.sim.world.defs);
            let daylight = defs.lookup("field", "daylight").ok_or("no daylight")? as usize;
            let term: rim_sim::terms::TermsDef = toml::from_str(
                "[green_moon]\nscale = 6.0\nof = [{ input = \"hour\", curve = [[0, 1.0], [4, 1.0], [5, 0.0], [20, 0.0], [21, 1.0], [24, 1.0]] }]",
            )
            .map_err(|e| e.to_string())?;
            let fields = |id: &str| defs.lookup("field", id).map(|f| f as usize);
            let term = rim_sim::terms::Terms::compile(&term, "test", &fields, &mut Vec::new())?;
            defs.fields[daylight].terms.terms.extend(term.terms);
            let mut body: rim_sim::defs::SkyBodyDef = toml::from_str(
                "id = \"test:green_moon\"\nfield = \"core:daylight\"\nterm = \"green_moon\"\ntransit = 0.0\ncolor = \"#7dffa0\"\nangular_size = 1.0",
            )
            .map_err(|e| e.to_string())?;
            let fields = |id: &str| defs.lookup("field", id).map(|f| f as usize);
            body.resolve(&defs.fields, &fields, &mut Vec::new())?;
            defs.sky_bodies.push(body);
            Ok(())
        })();
        t.check(applied.is_ok(), format!("the green moon's defs apply: {applied:?}"));
        t.ticks(20);
        t.app.light.adapt_now();
        t.frame().await;
        let slots: Vec<String> =
            t.app.light.shadow_bodies().iter().map(|&i| t.w().defs.sky_bodies[i].id.clone()).collect();
        t.check(slots == ["test:green_moon"], format!("it casts the night's shadows, over core's moon ({slots:?})"));
        // Highest at midnight, on the equator's line at latitude 45: 45° up,
        // due south, so its shadow falls north of the wall, a cell long.
        let (x, face) = (p.x as f32 + 0.5, p.y as f32);
        let shade = t.app.light.sun_visibility(x, face - 0.5).unwrap_or(1.0);
        let open = t.app.light.sun_visibility(x, face - 7.0).unwrap_or(0.0);
        t.check(shade < 0.5 && open > 0.9, format!("its own shadow behind the wall ({shade:.2}, open {open:.2})"));
        let green_night = hue(&t.grab().await);
        t.check(
            green_night[1] > core_night[1] + 0.01,
            format!("and the night turns green (green {:.3} of the light, was {:.3})", green_night[1], core_night[1]),
        );
        t.shot("green_moon").await;
        t.app.sim.world.defs = before;
        t.ticks(20);
        // efd56e49: the sun is where the sim has it, so its shadow swings
        // through the year. The same hour, 09:00, on the days in the year
        // ahead where the sun stands highest and lowest then, in the order
        // they come: the clock only runs forward.
        println!("\n# at the same hour a shadow points another way at midsummer and midwinter (efd56e49)");
        // A wall of its own, with nothing else within four cells: shade that
        // lands on a neighbour's top would read as no shadow at all.
        let open = |w: &World, q: IVec| {
            (-4..=4).all(|dy| {
                (-4..=4).all(|dx| {
                    let c = q.offset(dx, dy);
                    w.map.inb(c) && w.map.passable(c) && w.map.fixture_at(c).is_none() && w.solid_at(c).is_none()
                })
            })
        };
        let spot_at = (0..60i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| site.offset(dx, dy))))
            .find(|&q| open(t.w(), q));
        let sun = t.w().defs.sky_bodies.iter().position(|b| b.id == "core:sun");
        t.check(sun.is_some(), "core's sun is a sky body");
        let lone = spot_at.and_then(|q| t.app.sim.world.spawn_fixture(wall, q, false).map(|e| (q, e)));
        t.check(lone.is_some(), "a lone wall stands in the open for it");
        if let (Some(sun), Some((q, lone))) = (sun, lone) {
            let (tpd, year) = (rim_sim::TICKS_PER_DAY, t.w().defs.calendar.year_days as u64);
            let (was, first) = (t.w().tick, t.w().tick / tpd + 1);
            // Tick 0 is 06:00, so 09:00 is an eighth of a day in.
            let nine = |d: u64| (first + d) * tpd + tpd / 8;
            let altitude = |t: &T, d| {
                let defs = &t.w().defs;
                rim_sim::sky::state(&defs.sky_bodies[sun], &defs.calendar, nine(d)).altitude
            };
            let by = |a: &u64, b: &u64| altitude(t, *a).total_cmp(&altitude(t, *b));
            let (high, low) = ((0..year).max_by(by).unwrap_or(0), (0..year).min_by(by).unwrap_or(0));
            t.focus(q);
            t.ticks(2);
            let (cx, cy) = (q.x as f32 + 0.5, q.y as f32 + 0.5);
            // A spot in the wall's shadow, as the sun stands: away from it,
            // past the wall's own half cell and inside the shadow's length.
            let spot = |(az, alt): (f64, f64)| {
                let (away, long) = ((az + 180.0).to_radians() as f32, (1.0 / alt.to_radians().tan()).min(3.0) as f32);
                let d = 0.5 + 0.5 * long;
                (cx + away.cos() * d, cy + away.sin() * d)
            };
            let mut seen = Vec::new();
            for day in if high < low { [high, low] } else { [low, high] } {
                t.app.sim.world.tick = nine(day);
                t.ticks(20);
                t.app.light.adapt_now();
                for _ in 0..3 {
                    t.frame().await;
                }
                let light = crate::sky::Air::read(t.w()).light;
                let at = t.app.light.bodies(t.w(), light).0.get(sun).map_or((0.0, -90.0), |b| b.at);
                seen.push((day == high, at, t.app.light.sun_image()));
            }
            let vis = |img: &Option<Image>, (x, y): (f32, f32)| {
                img.as_ref().and_then(|i| t.app.light.sun_in(i, x, y)).unwrap_or(-1.0)
            };
            let (a, b) = (&seen[0], &seen[1]);
            let (sa, sb) = (spot(a.1), spot(b.1));
            let (own_a, own_b, cross_a, cross_b) = (vis(&a.2, sa), vis(&b.2, sb), vis(&b.2, sa), vis(&a.2, sb));
            let turned = ((a.1 .0 - b.1 .0 + 540.0).rem_euclid(360.0) - 180.0).abs();
            let says = seen
                .iter()
                .map(|&(h, (az, alt), _)| {
                    format!("{}: sun at {az:.0}°, {alt:.0}° up", if h { "midsummer" } else { "midwinter" })
                })
                .collect::<Vec<_>>()
                .join("; ");
            t.check(
                turned > 10.0 && (0.0..0.5).contains(&own_a) && (0.0..0.5).contains(&own_b) && (cross_a > 0.5 || cross_b > 0.5),
                format!(
                    "the shadow swings {turned:.0}°: each season's shadow spot is dark ({own_a:.2}, {own_b:.2}) and one lit in the other ({cross_a:.2}, {cross_b:.2}); {says}"
                ),
            );
            t.shot("season_shadows").await;
            t.app.sim.world.despawn_thing(lone);
            // On to the date and hour it was, a year later: later sections
            // keep their season and time of day, and the clock only runs
            // forward.
            let years = (t.w().tick - was).div_ceil(year * tpd);
            t.app.sim.world.tick = was + years * year * tpd;
            t.ticks(20);
        }
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
    carry.calm = Some(calm);
}

/// 6fd6b13b firelight
pub(super) async fn firelight(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before firelight");
    let site = carry.site.expect("set before firelight");
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
        let (at_a, between) = (glow(t, a), glow(t, o.offset(6, 7)));
        t.check(at_a > 0.3, format!("a campfire glows ({at_a:.2})"));
        // Six cells away, a second fire's glow overlaps the first's.
        let second = t.app.sim.world.spawn_fixture(campfire, c, false);
        t.ticks(2);
        t.light_settles().await;
        let draws = t.app.light.passes.iter().find(|p| p.name == "firelight").map_or(0, |p| p.draws);
        t.check(draws == 2, format!("the new fire redoes one area, not the map ({draws} draws)"));
        let spots = [a, c, o.offset(6, 7), o.offset(0, 7), o.offset(12, 7)];
        let partial: Vec<f32> = spots.iter().map(|&q| glow(t, q)).collect();
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
        let whole: Vec<f32> = spots.iter().map(|&q| glow(t, q)).collect();
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
}

/// 5a69f9c9 moving lights
pub(super) async fn moving_lights(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before moving_lights");
    let site = carry.site.expect("set before moving_lights");
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
        let (bakes, since) = (t.app.light.bakes, t.clock);
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
        let (baked, took) = (t.app.light.bakes - bakes, t.clock - since);
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
}

/// 08a5d182: flat is the default, and draws the sim's light in one pass.
pub(super) async fn flat_is_the_default(t: &mut T) {
    println!("\n# flat is the default lighting: the sim's light field, one multiply (08a5d182)");
    t.check(crate::quality::Setting::default().flat(), "a new game lights flat");
    t.app.light.set(crate::quality::Setting::Flat);
    t.app.light.adapt_now();
    for _ in 0..3 {
        t.frame().await;
    }
    let passes: Vec<(&str, bool)> = t.app.light.passes.iter().map(|p| (p.name, p.ran)).collect();
    let marched = passes.iter().any(|&(n, ran)| ran && matches!(n, "firelight" | "moving" | "sun"));
    let drew = passes.iter().any(|&(n, ran)| ran && n == "multiply");
    t.check(drew && !marched, format!("it draws one multiply and no bake, march or sun pass ({passes:?})"));
    t.shot("flat_lighting").await;
    // Today's look for the sections after this one.
    t.app.light.set(crate::quality::Setting::Shadows);
    t.frame().await;
}

/// f05c5fa1: the sky body's shadows are shapes. A staircase of walls casts
/// one straight diagonal edge, at every zoom, where a shadow marched over
/// the cells would step a cell at a time.
pub(super) async fn shadows_are_shapes(t: &mut T, carry: &mut Carry) {
    let site = carry.site.expect("set before shadows_are_shapes");
    let wall = carry.wall.expect("set before shadows_are_shapes");
    println!("\n# a staircase of walls casts one straight shadow edge at every zoom (f05c5fa1)");
    t.app.light.set(crate::quality::Setting::Flat);
    t.app.paused = true;
    // Dry, outdoor ground with nothing on it that casts, away from any
    // fire: 10 cells across, and 6 more to the south, where anything
    // standing would throw its own shadow north into the measure.
    let lights: Vec<IVec> = t
        .w()
        .defs
        .lookup("field", "light")
        .map(|f| t.w().fields.emitters_of(f as usize).map(|(_, q, _, _)| q).collect())
        .unwrap_or_default();
    let open = |w: &World, c: IVec| {
        dry(w, c)
            && w.map.passable(c)
            && !w.map.indoors(c)
            && crate::occluders::occluder_at(w, w.map.idx(c), None) == crate::occluders::Occluder::Open
    };
    let clear = |w: &World, o: IVec| {
        (0..16).all(|y| (0..10).all(|x| open(w, o.offset(x, y))))
            && lights.iter().all(|q| q.chebyshev(o.offset(5, 5)) > 12)
    };
    let (mw, mh) = (t.w().map.w, t.w().map.h);
    // The nearest such ground to the colony, anywhere on the map.
    let found = (0..mh - 16)
        .flat_map(|y| (0..mw - 10).map(move |x| IVec { x, y, z: site.z }))
        .filter(|&o| clear(t.w(), o))
        .min_by_key(|o| o.chebyshev(site));
    let Some(o) = found else {
        t.check(false, "open ground for the staircase");
        t.app.paused = false;
        return;
    };
    // Two cells wide, rising to the north-east from the square's bottom left.
    let cells: Vec<IVec> = (0..5).flat_map(|i| [o.offset(1 + i, 8 - i), o.offset(2 + i, 8 - i)]).collect();
    let walls: Vec<Entity> = cells.iter().filter_map(|&c| t.app.sim.world.spawn_fixture(wall, c, false)).collect();
    t.check(walls.len() == cells.len(), format!("the staircase stands ({} of {} walls)", walls.len(), cells.len()));
    let (cloud, light) = (field(t, "cloud"), field(t, "light"));
    t.app.sim.world.fields.set_ambient(cloud, Some(0.0));
    t.app.sim.world.fields.set_ambient(light, Some(100.0));
    t.ticks(20);
    t.app.light.adapt_now();
    t.light_settles().await;
    let zoom = t.app.cam.zoom;
    let lum = |img: &Image, at: (f32, f32)| px(img, at).iter().sum::<f32>();
    for (name, z) in [("close", 48.0), ("mid", 20.0), ("far", 8.0)] {
        t.app.cam.zoom = z;
        t.focus(o.offset(4, 5));
        // Due south and low, so the shadow falls north; and overhead, where
        // it has no length, to divide the ground's own colour out.
        t.app.light.pin_sun = Some((90.0, 89.0));
        let flat = t.grab().await;
        t.app.light.pin_sun = Some((90.0, 20.0));
        let low = t.grab().await;
        let cam = &t.app.cam;
        let ratio = |x: f32, y: f32| {
            let at = cam.to_screen(x, y);
            lum(&low, at) / lum(&flat, at).max(1e-3)
        };
        // South of the staircase the sun reaches in both: what a lit ratio is.
        let open = (0..8).map(|k| ratio(o.x as f32 + 1.5 + k as f32 * 0.5, o.y as f32 + 9.6)).sum::<f32>() / 8.0;
        // Up each column across the staircase's diagonal (its north-west
        // edge runs centre to centre from x 1.5 to 5.5; past that, the top
        // wall's tip is flat), from the top of its highest wall to where the
        // shadow ends: past its tip, it is lit.
        let mut ends = Vec::new();
        for k in 0..13 {
            let x = o.x as f32 + 2.0 + k as f32 * 0.25;
            let top = (o.y + 8 - (x.floor() as i32 - o.x - 1)) as f32;
            let r = |d: f32| ratio(x, top - d) / open;
            let foot = (1..20).map(|j| r(j as f32 * 0.1)).fold(1.0f32, f32::min);
            let lit = 1.0 - 0.15 * (1.0 - foot);
            let end = (1..160).map(|j| j as f32 / 16.0).find(|&d| r(d) > lit && r(d + 0.25) > lit);
            ends.push(end.map(|d| top - d));
        }
        let ys: Vec<f32> = ends.iter().flatten().copied().collect();
        let steps: Vec<f32> = ys.windows(2).map(|p| p[1] - p[0]).collect();
        let slope = steps.iter().sum::<f32>() / (steps.len().max(1) as f32 * 0.25);
        let worst = steps.iter().map(|d| (d + 0.25).abs()).fold(0.0f32, f32::max);
        // A pixel of the screen, in cells, is all a step may miss by past
        // the blur: a stair would miss by a whole cell.
        let slack = 0.3 + 2.0 / z;
        t.check(
            ys.len() == ends.len() && (-1.15..-0.85).contains(&slope) && worst < slack,
            format!(
                "{name} ({z} px a cell): the shadow's edge runs straight up the staircase, slope {slope:.2} (-1), steps off by at most {worst:.2} cells (under {slack:.2}), {} of {} columns found",
                ys.len(),
                ends.len()
            ),
        );
        t.shot(&format!("shadow_shapes_{name}")).await;
    }
    t.app.cam.zoom = zoom;
    t.app.light.pin_sun = None;
    t.app.sim.world.fields.set_ambient(cloud, None);
    t.app.sim.world.fields.set_ambient(light, None);
    for e in walls {
        t.app.sim.world.despawn_thing(e);
    }
    t.ticks(2);
    t.app.light.set(crate::quality::Setting::Shadows);
    t.app.paused = false;
}
