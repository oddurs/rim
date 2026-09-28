//! The camera and the right button.

use super::*;

/// 0045 camera
pub(super) async fn camera(t: &mut T, carry: &mut Carry) {
    let home = carry.home.expect("set before camera");
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
}

/// 0071 right-click orders
pub(super) async fn right_click_orders(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before right_click_orders");
    let founder = carry.founder.expect("set before right_click_orders");
    let home = carry.home.expect("set before right_click_orders");
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
    let on_tree = hovers(t);
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
    let none = hovers(t);
    t.check(none.is_empty(), format!("bare ground gets no hover ({none:?})"));
    match t.ui_rect("core:dock") {
        Some(r) => {
            t.mouse = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
            for _ in 0..12 {
                t.frame().await;
            }
            let none = hovers(t);
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
}

/// fda56c8e camera by device
pub(super) async fn camera_by_device(t: &mut T, carry: &mut Carry) {
    let founder = carry.founder.expect("set before camera_by_device");
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
    let before = cam(t);
    t.input(RawInput {
        mouse: mid,
        scroll: crate::Scroll { notches: 0.0, travel: (12.0, -30.0) },
        ..Default::default()
    })
    .await;
    let after = cam(t);
    t.check(
        (after.0 - (before.0 - 12.0 / before.2)).abs() < 1e-3 && (after.1 - (before.1 + 30.0 / before.2)).abs() < 1e-3,
        format!("a trackpad's travel pans one to one ({before:?} → {after:?})"),
    );
    t.check(after.2 == before.2, "and doesn't zoom");
    for _ in 0..30 {
        t.frame().await;
    }
    let before = cam(t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    let after = cam(t);
    t.check((after.2 / before.2 - 1.12).abs() < 1e-3, format!("a wheel notch zooms 12% ({:.3})", after.2 / before.2));
    let before = cam(t);
    t.input(RawInput {
        mouse: mid,
        scroll: crate::Scroll { notches: 0.0, travel: (0.0, 40.0) },
        zoom_mod: true,
        ..Default::default()
    })
    .await;
    t.check(cam(t).2 > before.2, "Cmd with a trackpad scroll zooms");
    // A round-numbered delta right after trackpad scrolling is still the trackpad.
    let before = cam(t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 0.0, travel: (0.0, 5.0) }, ..Default::default() })
        .await;
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    t.check(cam(t).2 == before.2, "a whole notch amid trackpad scrolling pans, not zooms");
    // Right-drag pans, and gives no order.
    for _ in 0..30 {
        t.frame().await;
    }
    t.app.order_flash = None;
    let before = cam(t);
    t.input(RawInput { mouse: mid, right_pressed: true, right_down: true, ..Default::default() }).await;
    t.input(RawInput { mouse: (mid.0 + 50.0, mid.1), right_down: true, ..Default::default() }).await;
    t.input(RawInput { mouse: (mid.0 + 50.0, mid.1), right_released: true, ..Default::default() }).await;
    t.check(
        (cam(t).0 - (before.0 - 50.0 / before.2)).abs() < 1e-3,
        format!("a right-drag pans the ground with the pointer ({before:?} → {:?})", cam(t)),
    );
    t.check(t.app.order_flash.is_none() && t.app.ui.find("core:menu").is_none(), "and gives no order");
    // The setting pins it: with "pan", a wheel pans too.
    t.app.scroll_mode = crate::ScrollMode::Pan;
    let before = cam(t);
    t.input(RawInput { mouse: mid, scroll: crate::Scroll { notches: 1.0, travel: (0.0, 0.0) }, ..Default::default() })
        .await;
    t.check(cam(t).2 == before.2 && cam(t).1 != before.1, "with scroll set to pan, a wheel pans");
    t.app.scroll_mode = crate::ScrollMode::Auto;
    // A pinch: 10% apart zooms 10% about the pointer, the ground under it
    // staying put.
    let before = cam(t);
    let under = t.app.cam.to_world(mid.0, mid.1);
    t.input(RawInput { mouse: mid, pinch: 0.1, ..Default::default() }).await;
    let after_under = t.app.cam.to_world(mid.0, mid.1);
    t.check(
        (cam(t).2 / before.2 - 1.1).abs() < 1e-3
            && (after_under.0 - under.0).abs() < 1e-3
            && (after_under.1 - under.1).abs() < 1e-3,
        "a pinch zooms about the pointer",
    );
    let before = cam(t);
    t.key(KeyCode::Equal).await;
    t.check(cam(t).2 > before.2, "= zooms in");
}

/// 9aa55d96 safe right-click
pub(super) async fn safe_right_click(t: &mut T, carry: &mut Carry) {
    let founder = carry.founder.expect("set before safe_right_click");
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
}
