//! The ground remembers the weather: the weather plugin's wetness and snow
//! stock fields, held to their tuning targets under pinned weather.

use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn sim() -> Sim {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, 3, &|m| m == "core" || m == "weather").expect("mods load");
    s.step();
    s
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap() as usize
}

fn value(s: &Sim, id: &str, p: IVec) -> f64 {
    s.world.fields.value(&s.world.defs, &s.world.map, field(s, id), p)
}

/// Weather held still: temperature °C, precipitation mm/h, cloud %, wind m/s.
fn weather(s: &mut Sim, temp: f64, precip: f64, cloud: f64, wind: f64) {
    for (id, v) in [("temperature", temp), ("precipitation", precip), ("cloud", cloud), ("wind", wind)] {
        let f = field(s, id);
        s.world.fields.set_ambient(f, Some(v));
    }
}

/// Only the fields move: the weather is pinned, so nothing else matters
/// and a week takes a moment.
fn run(s: &mut Sim, hours: f64) {
    let defs = s.world.defs.clone();
    for _ in 0..(hours * TICKS_PER_DAY as f64 / 24.0) as u64 {
        s.world.tick += 1;
        let clock = s.world.clock();
        s.world.fields.update(&defs, &mut s.world.map, clock);
    }
}

/// An open cell of a terrain, well away from water, fixtures and the colony.
fn probe(s: &mut Sim, terrain: &str) -> IVec {
    let t = s.world.defs.lookup("terrain", terrain).unwrap();
    let water = s.world.defs.terrain_tags.iter().position(|t| t == "water").unwrap();
    s.world.map.ensure_near();
    let home = s.world.colony_center().unwrap();
    let m = &s.world.map;
    (0..m.plane())
        .map(|i| m.pos(i))
        .filter(|&p| p.chebyshev(home) > 30 && (8..m.w - 8).contains(&p.x) && (8..m.h - 8).contains(&p.y))
        .find(|&p| {
            let i = m.idx(p);
            m.terrain[i] == t && m.near(water, i) >= 6 && m.fixture_at(p).is_none() && !m.covered(i)
        })
        .unwrap_or_else(|| panic!("no open {terrain} on this map"))
}

#[test]
fn rain_soaks_the_ground_and_it_dries_by_its_drainage() {
    let mut s = sim();
    let (grass, sand, marsh) = (probe(&mut s, "grass"), probe(&mut s, "sand"), probe(&mut s, "marsh"));
    let w = |s: &Sim, p| value(s, "weather:wetness", p);
    weather(&mut s, 15.0, 0.0, 40.0, 3.0);
    run(&mut s, 24.0);
    let (g0, s0, m0) = (w(&s, grass), w(&s, sand), w(&s, marsh));
    println!("base: grass {g0} sand {s0} marsh {m0}");
    weather(&mut s, 12.0, 2.4, 100.0, 6.0);
    run(&mut s, 24.0);
    let (g1, s1) = (w(&s, grass), w(&s, sand));
    println!("after a day of rain: grass {g1} sand {s1} marsh {}", w(&s, marsh));
    weather(&mut s, 18.0, 0.0, 10.0, 3.0);
    run(&mut s, 12.0);
    let s2 = w(&s, sand);
    println!("half a sunny day: grass {} sand {s2}", w(&s, grass));
    run(&mut s, 36.0);
    let g2 = w(&s, grass);
    println!("two sunny days: grass {g2} sand {}", w(&s, sand));
    weather(&mut s, 25.0, 0.0, 0.0, 8.0);
    run(&mut s, 24.0 * 5.0);
    let m2 = w(&s, marsh);
    println!("five hot windy days: marsh {m2} grass {} sand {}", w(&s, grass), w(&s, sand));

    assert!((70.0..=90.0).contains(&g1), "a day of rain takes grass to about 80%: {g1}");
    assert!(g2 <= g0 + 8.0, "two sunny days dry grass back: {g2} (base {g0})");
    assert!(s2 <= s0 + 8.0, "sand dries in half a day: {s2} (base {s0})");
    assert!(m2 >= 70.0 && m0 >= 70.0, "marsh stays at 70% or more: {m0} then {m2}");
}

#[test]
fn snow_builds_in_the_cold_and_a_thaw_clears_it() {
    let mut s = sim();
    let grass = probe(&mut s, "grass");
    let snow = |s: &Sim| value(s, "weather:snow", grass);
    // A winter week: snowing a third of each day, otherwise cold and grey.
    for _ in 0..7 {
        weather(&mut s, -5.0, 2.4, 100.0, 5.0);
        run(&mut s, 8.0);
        weather(&mut s, -6.0, 0.0, 70.0, 3.0);
        run(&mut s, 16.0);
    }
    let deep = snow(&s);
    println!("a winter week: {deep} cm");
    // A spring thaw: mild days, cold nights, no rain.
    let mut gone = None;
    for h in 0..24 * 6 {
        let t = if h % 24 < 12 { 11.0 } else { 3.0 };
        weather(&mut s, t, 0.0, 40.0, 3.0);
        run(&mut s, 1.0);
        if gone.is_none() && snow(&s) == 0.0 {
            gone = Some(h + 1);
        }
    }
    println!("thawed after {gone:?} hours, wetness {}", value(&s, "weather:wetness", grass));
    assert!((20.0..=40.0).contains(&deep), "a winter week of snow gives 20 to 40 cm: {deep}");
    assert!(gone.is_some_and(|h| h <= 96), "a spring thaw clears it within four days: {gone:?}");
}

/// A roof keeps rain and snow off: inside a walled hut the floor stays dry
/// and clear while the ground outside soaks and whitens.
#[test]
fn an_enclosed_room_stays_dry_and_free_of_snow() {
    let mut s = sim();
    // A hut in the middle of a clearing; walls hold a roof four cells out,
    // so the open ground is measured seven away.
    let o = probe(&mut s, "grass").offset(-7, -7);
    let (grass, wall) = (s.world.defs.lookup("terrain", "grass").unwrap(), s.world.defs.thing_id("wall").unwrap());
    for y in 0..15 {
        for x in 0..15 {
            let p = o.offset(x, y);
            if !s.world.map.inb(p) {
                continue;
            }
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            s.world.map.set_terrain(p, grass, 100);
            if (5..10).contains(&x) && (5..10).contains(&y) && (x == 5 || y == 5 || x == 9 || y == 9) {
                s.world.spawn_fixture(wall, p, false).expect("a wall");
            }
        }
    }
    s.world.map.ensure_rooms();
    let (inside, outside) = (o.offset(7, 7), o.offset(7, 0));
    let i = s.world.map.idx(outside);
    assert!(s.world.map.indoors(inside) && !s.world.map.covered(i));
    weather(&mut s, 10.0, 3.0, 100.0, 5.0);
    run(&mut s, 24.0);
    let w = |s: &Sim, p| value(s, "weather:wetness", p);
    assert!(w(&s, outside) > 70.0 && w(&s, inside) <= 40.0, "outside {} inside {}", w(&s, outside), w(&s, inside));
    weather(&mut s, -6.0, 3.0, 100.0, 5.0);
    run(&mut s, 24.0);
    let snow = |s: &Sim, p| value(s, "weather:snow", p);
    assert!(
        snow(&s, outside) > 10.0 && snow(&s, inside) == 0.0,
        "outside {} inside {}",
        snow(&s, outside),
        snow(&s, inside)
    );
    // Both are map overlays, so the O cycle and the hover card show them.
    let d = &s.world.defs;
    assert!(d.fields[field(&s, "weather:wetness")].overlay && d.fields[field(&s, "weather:snow")].overlay);
}

/// On the real map, with levels below: the ground's two fields keep to
/// the surface and work each cell out hourly, which is what makes them
/// cheap (kept on every level at half-hourly they cost 0.29 ms a tick).
/// Checked by what makes it so, not timed: the time is printed.
#[test]
fn the_ground_is_cheap() {
    let mut s = sim();
    weather(&mut s, 5.0, 2.0, 90.0, 4.0);
    let defs = s.world.defs.clone();
    for id in ["weather:wetness", "weather:snow"] {
        let fd = &defs.fields[field(&s, id)];
        assert_eq!(fd.levels, rim_sim::defs::StockLevels::Surface, "{id} keeps to the surface");
        assert!(fd.period_minutes >= 60.0, "{id} is worked out hourly or less often");
    }
    s.world.fields.step_stock(&defs, &s.world.map, s.world.clock());
    assert_eq!(s.world.fields.layers[field(&s, "weather:wetness")].stock.len(), s.world.map.plane());
    let n = 2 * defs.fields[field(&s, "weather:wetness")].period;
    let mut fastest = f64::MAX;
    for _ in 0..3 {
        let t0 = std::time::Instant::now();
        for _ in 0..n {
            s.world.tick += 1;
            let clock = s.world.clock();
            s.world.fields.step_stock(&defs, &s.world.map, clock);
        }
        fastest = fastest.min(t0.elapsed().as_secs_f64() * 1e3 / n as f64);
    }
    let m = &s.world.map;
    // A report, not a check: alone it is about 0.01 ms.
    println!("wetness and snow on {}×{}: {fastest:.4} ms a tick", m.w, m.h);
}
