//! Weather.

use super::*;

/// weather
pub(super) async fn weather(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before weather");
    let site = carry.site.expect("set before weather");
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
    // A day of the forced storm with nobody told to shelter would kill
    // colonists on some machines and not others: the day passes mild.
    let (temp, rain) = (
        defs.lookup("field", "temperature").unwrap() as usize,
        defs.lookup("field", "precipitation").unwrap() as usize,
    );
    t.app.sim.world.fields.set_ambient(temp, Some(16.0));
    t.app.sim.world.fields.set_ambient(rain, Some(0.0));
    let before = t.w().tick;
    t.click_ui("weather:devtools.advance.24").await;
    let skipped = t.w().tick - before;
    t.app.sim.world.fields.set_ambient(temp, None);
    t.app.sim.world.fields.set_ambient(rain, None);
    t.keep_well();
    t.check(skipped >= rim_sim::TICKS_PER_DAY, format!("+1 day runs the sim a day forward ({skipped} ticks)"));
    t.frame().await;
    t.shot("weather_devtools").await;
    t.key(KeyCode::F12).await;
    t.frame().await;

    // Pin the channels to see each kind of weather over the hut.
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
    set(t, [4.0, 12.0, 6.0, 30.0, 95.0, 0.0]);
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
    set(t, [3.0, -6.0, 4.0, 150.0, 90.0, 0.0]);
    t.ticks(40);
    for _ in 0..90 {
        t.frame().await;
    }
    t.shot("snow").await;
    set(t, [0.0, 6.0, 0.5, 0.0, 60.0, 85.0]);
    t.ticks(40);
    t.frame().await;
    t.check(t.app.sky.particles() < 60, "fog: nothing falls");
    t.shot("fog").await;

    // Night, to see lighting and the labels on top of it.
    for id in pins {
        let f = field(t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }
    while !(22.0..23.0).contains(&t.w().hour()) {
        t.ticks(100);
        t.keep_well();
    }
    t.focus(site.offset(3, 3));
    t.shot("night").await;
    // The grid darkens with the ground, and still shows (553bfb19).
    if let Some(o) = open_square(t.w(), site, 6) {
        let was = t.app.paused;
        t.app.paused = true;
        // No rain: falling streaks would move pixels between the shots.
        let rain = t.w().defs.lookup("field", "precipitation").unwrap() as usize;
        t.app.sim.world.fields.set_ambient(rain, Some(0.0));
        t.focus(o.offset(3, 3));
        let [rest, _, plan] = grid_shots(t, o, defs.thing_id("wall").expect("walls")).await;
        let corner = t.app.cam.to_screen(o.x as f32 + 4.0, o.y as f32 + 4.0);
        let d = patch_diff(&rest, &plan, corner, 4.0);
        t.check(d > 0.3, format!("at night the grid still shows near the pointer ({d:.2})"));
        t.focus(site.offset(3, 3));
        t.app.paused = was;
        t.app.sim.world.fields.set_ambient(rain, None);
    }

    // A storm at night, mid-flash.
    set(t, [8.0, 9.0, 16.0, 20.0, 100.0, 0.0]);
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
    // A flash casts shadows from where the bolt is, worked out as it
    // strikes and again as it fades: twice at most, however long it lasts.
    let runs = t.app.light.sun_runs;
    t.app.sky.strike();
    t.frame().await;
    // One frame: the flash is lit from its bolt from the frame it strikes.
    let lit = t.app.light.lit_by_flash()
        && t.app.light.sun_image().is_some_and(|img| {
            let px = |k: usize| img.bytes[k * 4];
            let n = img.bytes.len() / 4;
            (0..n).any(|k| px(k) > 230) && (0..n).any(|k| px(k) < 25)
        });
    t.check(lit, "at night a flash lights the ground from where the bolt is, and walls shade it");
    t.shot("storm").await;
    // The rain stops, so no second flash strikes while this one fades,
    // however slowly the frames come.
    t.app.sim.world.fields.set_ambient(field(t, "precipitation"), Some(0.0));
    for _ in 0..120 {
        if t.app.sky.flash().strength == 0.0 && !t.app.light.lit_by_flash() {
            break;
        }
        t.frame().await;
    }
    let runs = t.app.light.sun_runs - runs;
    t.check(
        !t.app.light.lit_by_flash() && (1..=2).contains(&runs),
        format!("a flash works out its shadows at most twice, on and off ({runs} sun passes)"),
    );
    for id in pins {
        let f = field(t, id);
        t.app.sim.world.fields.set_ambient(f, None);
    }

    // The weather section leaves the colony as it found it.
    t.keep_well();
    carry.wall = Some(wall);
}
