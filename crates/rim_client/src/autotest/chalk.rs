//! Chalkline: the grid, a group selected, urgent hunts, unreachable jobs.

use super::*;

/// 553bfb19 the grid
pub(super) async fn the_grid(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before the_grid");
    let home = carry.home.expect("set before the_grid");
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
    let [rest, lens, plan] = grid_shots(t, o, wall).await;
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
    let [rest, _, plan] = grid_shots(t, o, wall).await;
    let corner = t.app.cam.to_screen(o.x as f32 + 4.0, o.y as f32 + 4.0);
    let d = patch_diff(&rest, &plan, corner, 6.0);
    t.check(d < 0.2, format!("at 8 points a cell there's no grid ({d:.2})"));
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
            w.map.inb(p)
                && w.solid_at(p).is_none()
                && w.map.fixture_at(p).is_none()
                && w.map.item_at(p).is_none()
                && w.map.floor_at(p).is_none()
        })
    };
    // Measuring lifts the pointer's own row and column the screen across:
    // the line read against is two cells from a fifth line, and off it.
    let at = t.app.cam.tile_at(t.mouse.0, t.mouse.1);
    let minor = [major + 2, major - 2].into_iter().find(|x| (x - at.x).abs() > 1).unwrap_or(major + 2);
    // Panels change on their own (alerts, the clock): only rows where both
    // lines are on the map count, off the pointer's row.
    let dpi = screen_dpi_scale();
    let sx = |x: i32| t.app.cam.to_screen(x as f32, 0.0).0;
    let sy = |y: i32| t.app.cam.to_screen(0.0, y as f32 + 0.5).1;
    let shown = |y: i32| [major, minor].iter().all(|&x| !t.app.ui.covers(sx(x) * dpi, sy(y) * dpi));
    let rows: Vec<i32> = (y0 + 2..y1 - 2)
        .filter(|&y| y % every != 0 && (y - at.y).abs() > 1 && open(t, major, y) && open(t, minor, y) && shown(y))
        .collect();
    // The median row: a tree's canopy swaying over a line in a row or two,
    // between the two shots, doesn't decide it.
    let along = |x: i32| {
        let mut d: Vec<f32> = rows.iter().map(|&y| patch_diff(&before, &measured, (sx(x), sy(y)), 1.5)).collect();
        d.sort_by(f32::total_cmp);
        d.get(d.len() / 2).copied().unwrap_or(0.0)
    };
    let (dm, dn) = (along(major), along(minor));
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
    // Escape closes it, and backs out of nothing behind it.
    let selected = t.app.selected;
    t.key(KeyCode::Escape).await;
    t.check(!t.app.ui.is_open("core:palette"), "Escape closes the palette");
    t.check(t.app.selected == selected, "and leaves the selection as it was");

    // Motion (bf3079fb): an urgent mark breathes; with reduce motion on, it
    // holds still.
    let oak = defs.thing_id("tree_oak").unwrap();
    let chop = defs.lookup("designation", "chop").unwrap();
    let urgent = {
        let w = t.w();
        let (x0, y0, x1, y1) = draw::visible(&t.app);
        (y0 + 2..y1 - 2)
            .flat_map(|y| (x0 + 2..x1 - 2).map(move |x| IVec::new(x, y)))
            .find_map(|p| w.map.fixture_at(p).filter(|&f| w.thing(f).is_some_and(|th| th.def == oak)))
    };
    match urgent {
        Some(tree) => {
            let p = t.w().thing(tree).unwrap().pos;
            t.app.sim.push(Command::Designate { designation: chop, a: p, b: p });
            t.ticks(1);
            t.app.sim.push(Command::MarkUrgent { target: tree, on: true });
            t.ticks(1);
            let breath = |t: &T| {
                crate::overlay::scene(&t.app).marks.iter().find_map(|m| {
                    if let Mark::Breathe { r, .. } = m {
                        Some(*r)
                    } else {
                        None
                    }
                })
            };
            let r0 = breath(t);
            for _ in 0..30 {
                t.frame().await;
            }
            let r1 = breath(t);
            t.check(r0.is_some() && r0 != r1, format!("an urgent mark's ring breathes ({r0:?}, {r1:?})"));
            t.shot("chalk-urgent").await;
            // An order's ring grows over a quarter of a second.
            let ack = |t: &T| {
                crate::overlay::scene(&t.app).marks.iter().find_map(|m| {
                    if let Mark::Ack { r, .. } = m {
                        Some(*r)
                    } else {
                        None
                    }
                })
            };
            t.app.order_flash = Some((p, t.app.chalk.now()));
            t.frame().await;
            let a0 = ack(t);
            for _ in 0..6 {
                t.frame().await;
            }
            let a1 = ack(t);
            t.check(a0.zip(a1).is_some_and(|(x, y)| y > x), format!("an order's ring grows ({a0:?}, {a1:?})"));
            t.input(RawInput { mouse: t.mouse, pressed: vec!["ctrl+k".into()], ..Default::default() }).await;
            t.input(RawInput { mouse: t.mouse, chars: "reduce motion: on".chars().collect(), ..Default::default() })
                .await;
            t.key(KeyCode::Enter).await;
            t.check(t.app.reduce_motion, "the palette's Reduce motion: on stills the map");
            let r0 = breath(t);
            for _ in 0..30 {
                t.frame().await;
            }
            let r1 = breath(t);
            t.check(r0.is_some() && r0 == r1, format!("and the urgent ring holds still ({r0:?}, {r1:?})"));
            t.app.order_flash = Some((p, t.app.chalk.now()));
            t.frame().await;
            let a0 = ack(t);
            for _ in 0..6 {
                t.frame().await;
            }
            let a1 = ack(t);
            t.check(a0.is_some() && a0 == a1, format!("and an order's ring is still ({a0:?}, {a1:?})"));
            t.app.reduce_motion = false;
            t.app.sim.push(Command::MarkUrgent { target: tree, on: false });
            t.app.sim.push(Command::Cancel { a: p, b: p });
            t.ticks(1);
        }
        None => t.check(false, "a tree on screen to mark urgent"),
    }
    t.app.cam.zoom = 28.0;
    t.app.paused = false;
    t.app.sim.world.fields.set_ambient(rain, None);
}

/// 86dcd0ca several selected
pub(super) async fn several_selected(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before several_selected");
    let founder = carry.founder.expect("set before several_selected");
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
    // A boxed colonist and a cell beside them, so the two-cell box holds
    // them and no one else. A box goes by where a colonist is drawn, which
    // is a cell on from the sim's mid-step, and others may stand close.
    let drawn = |t: &T, e: Entity| {
        let pawn = t.pawn(e);
        let (x, y) = draw::pawn_pos(&t.app, e, &pawn);
        IVec::at(x.floor() as i32, y.floor() as i32, pawn.pos.z)
    };
    let (one, p, alone) = boxed
        .iter()
        .flat_map(|&e| {
            let p = drawn(t, e);
            [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| (e, p, p.offset(dx, dy)))
        })
        .find(|&(e, p, q)| crate::boxed_colonists(&t.app, p, q) == [e])
        .expect("a boxed colonist with a cell beside them and no one else");
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
    // Where they were sent, together: a cell apart at most, where they stood
    // is two, and chevrons merge within a cell's width.
    for _ in 0..600 {
        if left.iter().all(|&e| !matches!(t.pawn(e).job, Job::MoveTo { .. })) {
            break;
        }
        t.ticks(1);
    }
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
}

/// 337cb649 an urgent hunt shows
pub(super) async fn an_urgent_hunt_shows(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before an_urgent_hunt_shows");
    let home = carry.home.expect("set before an_urgent_hunt_shows");
    // Pawns aren't in the chunk mesh, so a creature's urgent mark is drawn
    // with the pawns: a marked deer wears the amber disc, and loses it when
    // the hunt is called off.
    println!("\n# an urgent mark on a hunt target (337cb649)");
    // In daylight: marks are lit with the world, and the weather section
    // leaves it night, where amber and a brown deer both read near black and
    // firelight flickers across them from frame to frame.
    while !(11.0..14.0).contains(&t.w().hour()) {
        t.ticks(100);
        t.keep_well();
    }
    t.app.paused = true;
    // Full daylight under a clear sky, whatever this map's midday brings:
    // the mark is lit with the world, and cloud takes the amber down.
    let lit: Vec<(usize, f64)> = [("cloud", 0.0), ("light", 100.0), ("fog", 0.0), ("precipitation", 0.0)]
        .iter()
        .filter_map(|&(id, v)| defs.lookup("field", id).map(|f| (f as usize, v)))
        .collect();
    for &(f, v) in &lit {
        t.app.sim.world.fields.set_ambient(f, Some(v));
    }
    t.ticks(2);
    t.app.light.adapt_now();
    let deer_def = defs.creature_id("deer").expect("core's deer");
    let spot = (4..20)
        .flat_map(|d| [home.offset(d, d), home.offset(-d, d), home.offset(d, -d), home.offset(-d, -d)])
        .find(|&p| {
            // Outdoors: a hut's shade would take the amber down with the deer.
            t.w().map.passable(p)
                && !t.w().map.indoors(p)
                && t.w().pawns.iter().filter_map(|&e| t.w().pawn_pos(e)).all(|q| q.chebyshev(p) > 3)
        })
        .expect("open ground outdoors for a deer");
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
        let (w, h) = (img.width() as i32, img.height() as i32);
        let (cx, cy) = ((ux * dpi) as i32, (uy * dpi) as i32);
        let m = draw::URGENT_MARK;
        // The nearest to amber in a few pixels round the spot: one pixel can
        // land on the disc's rim.
        (-2..=2)
            .flat_map(|dy| (-2..=2).map(move |dx| ((cx + dx).clamp(0, w - 1), (cy + dy).clamp(0, h - 1))))
            .map(|(x, y)| {
                let c = img.get_pixel(x as u32, (h - 1 - y) as u32);
                (c.r - m.r).abs() + (c.g - m.g).abs() + (c.b - m.b).abs()
            })
            .fold(f32::MAX, f32::min)
    };
    let img = t.grab().await;
    let calm = urgent_px(t, &img);
    t.app.sim.push(rim_sim::Command::MarkUrgent { target: deer, on: true });
    t.ticks(1);
    let on = t.w().ecs.get::<&Urgent>(deer).is_ok();
    t.check(on, "a deer marked for hunting takes an urgent mark");
    let img = t.grab().await;
    let marked = urgent_px(t, &img);
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
    let after = urgent_px(t, &img);
    t.check(after > marked + 0.3, format!("and the amber goes from the map ({after:.2} from amber)"));
    for &(f, _) in &lit {
        t.app.sim.world.fields.set_ambient(f, None);
    }
    t.app.paused = false;
}

/// 7ffd8d09 unreachable
pub(super) async fn unreachable(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before unreachable");
    let home = carry.home.expect("set before unreachable");
    // A tree marked to chop, walled in: a notch on it, and gone once a
    // way in opens.
    println!("\n# unreachable jobs (7ffd8d09)");
    let oak = defs.thing_id("tree_oak").expect("oaks");
    let (wall, wood) = (defs.thing_id("wall").expect("walls"), defs.thing_id("wood").unwrap());
    let chop = defs.lookup("designation", "chop").unwrap();
    // Free ground round the island too, so the way in opens onto it.
    let o = open_square(t.w(), home, 5).expect("open ground for an island").offset(1, 1);
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
    t.check(marked && notched(t), format!("a marked tree walled in carries a notch (marked {marked})"));
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
    t.check(!notched(t), "a way in takes the notch away");
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
        // Dry rock all round, under ground with no open water on it: rock
        // that seeps, or a lake above, would fill the room before the river.
        let dry = |w: &World, o: IVec| {
            (-1..=10).all(|y| {
                (-2..=14).all(|x| {
                    let p = o.offset(x, y);
                    let above = IVec::at(p.x, p.y, 0);
                    w.solid_at(p).is_some_and(|r| r.seeps == 0) && dry(w, above)
                })
            })
        };
        let near = IVec::at(home.x + 20, home.y - 20, -1);
        let o = (0..40i32)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| near.offset(dx, dy))))
            .find(|&o| dry(t.w(), o))
            .expect("dry rock below for a basin");
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
        let early = wet(t);
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
}
