//! `rim seeds`: find maps worth keeping, look at one, and play many
//! (DESIGN.md §7b). Runs without a window.

use rim_sim::seeds::{self, Where};
use rim_sim::Sim;
use std::path::PathBuf;

const USAGE: &str = "usage: rim seeds <find|show|soak|sweep> [options]

  find  --where \"water near start\" [--n 3] [--from 1]
        Seeds whose map answers yes, from --from on. A question is
        '<feature> near start', 'no <feature> near start' or
        '<feature> within N of start'; a feature is a terrain id, a terrain
        tag or a thing id.
  show  SEED --out map.png [--scale 3]
        The map as a picture: terrain, what stands on it, colonists in
        white, the start ringed in red.
  soak  --seed SEED [--days 2]
        Plays one seed, saves and loads it. Exits 1 on a panic, a script
        error or a save that doesn't load as it was.
  sweep --date YYYYMMDD [--n 200] [--days 2] [--jobs 4] [--report FILE]
        Soaks the night's seeds, made from the date, in parallel. Each
        failure is a line of FILE: seed, tick, reason, the command that
        plays it again.

Every command takes --mods DIR (default ./mods).";

struct Opts {
    rest: Vec<String>,
    mods: PathBuf,
    get: std::collections::BTreeMap<String, String>,
}

fn opts(args: &[String], flags: &[&str]) -> Result<Opts, String> {
    let mut o = Opts { rest: Vec::new(), mods: PathBuf::from("mods"), get: Default::default() };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--mods" {
            o.mods = it.next().map(PathBuf::from).ok_or("--mods needs a folder")?;
        } else if flags.contains(&a.as_str()) {
            let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
            o.get.insert(a.clone(), v.clone());
        } else if a.starts_with('-') {
            return Err(format!("unknown option {a}"));
        } else {
            o.rest.push(a.clone());
        }
    }
    Ok(o)
}

fn num(o: &Opts, key: &str, default: u64) -> Result<u64, String> {
    match o.get.get(key) {
        Some(v) => v.parse().map_err(|_| format!("{key} {v}: not a number")),
        None => Ok(default),
    }
}

/// `rim seeds`: returns the process exit code.
pub fn seeds(args: &[String]) -> i32 {
    let run = match args.first().map(String::as_str) {
        Some("find") => find(&args[1..]),
        Some("show") => show(&args[1..]),
        Some("soak") => soak(&args[1..]),
        Some("sweep") => sweep(&args[1..]),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            return 0;
        }
        _ => Err(String::new()),
    };
    match run {
        Ok(code) => code,
        Err(e) if e.is_empty() => {
            eprintln!("{USAGE}");
            2
        }
        Err(e) => {
            eprintln!("rim seeds: {e}\n\n{USAGE}");
            2
        }
    }
}

fn days_text(days: u64) -> String {
    if days == 1 {
        "1 day".into()
    } else {
        format!("{days} days")
    }
}

fn find(args: &[String]) -> Result<i32, String> {
    let o = opts(args, &["--where", "--n", "--from"])?;
    let q = Where::parse(o.get.get("--where").ok_or("find needs --where")?)?;
    let (n, from) = (num(&o, "--n", 3)?, num(&o, "--from", 1)?);
    let mut found = 0;
    for seed in from.. {
        let sim = Sim::with_mods(&o.mods, seed, &|_| true)?;
        let Some(start) = seeds::start_of(&sim) else { continue };
        if let Some(at) = q.check(&sim.world, start) {
            let how = if q.no {
                format!("no {} within {}", q.feature, q.within)
            } else {
                format!(
                    "{} at ({}, {}), {} from the start",
                    q.feature,
                    at.x,
                    at.y,
                    (at.x - start.x).abs().max((at.y - start.y).abs())
                )
            };
            println!("seed {seed}: {how}");
            found += 1;
            if found == n {
                return Ok(0);
            }
        }
        if seed - from >= 100_000 {
            break;
        }
    }
    eprintln!("rim seeds find: {found} of {n} in 100000 seeds");
    Ok(1)
}

fn rgb(hex: &str) -> [u8; 3] {
    let h = hex.trim_start_matches('#');
    let byte = |i: usize| h.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok()).unwrap_or(128);
    [byte(0), byte(2), byte(4)]
}

fn show(args: &[String]) -> Result<i32, String> {
    let o = opts(args, &["--out", "--scale"])?;
    let seed: u64 = o.rest.first().ok_or("show needs a seed")?.parse().map_err(|_| "the seed is a number")?;
    let out = o.get.get("--out").ok_or("show needs --out FILE.png")?;
    let scale = num(&o, "--scale", 3)?.clamp(1, 16) as u32;
    let sim = Sim::with_mods(&o.mods, seed, &|_| true)?;
    let (w, m) = (&sim.world, &sim.world.map);
    let (mw, mh) = (m.w as u32, m.h as u32);
    let mut px = vec![0u8; (mw * scale * mh * scale * 4) as usize];
    let mut paint = |x: u32, y: u32, c: [u8; 3]| {
        for dy in 0..scale {
            for dx in 0..scale {
                let i = (((y * scale + dy) * mw * scale + x * scale + dx) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    };
    for i in 0..m.plane() {
        let p = m.pos(i);
        let mut c = rgb(&w.defs.terrain[m.terrain[i] as usize].color);
        if let Some(t) = [m.fixture_at(p), m.item_at(p)].into_iter().flatten().find_map(|e| w.thing(e)) {
            c = rgb(&w.defs.thing(t.def).color);
        }
        paint(p.x as u32, p.y as u32, c);
    }
    for e in w.colonists() {
        if let Some(p) = w.pawn_pos(e) {
            paint(p.x as u32, p.y as u32, [255, 255, 255]);
        }
    }
    // The start: a red ring three cells out, so it shows at any scale.
    if let Some(s) = seeds::start_of(&sim) {
        for d in -3i32..=3 {
            for (x, y) in [(s.x + d, s.y - 3), (s.x + d, s.y + 3), (s.x - 3, s.y + d), (s.x + 3, s.y + d)] {
                if m.inb(rim_sim::IVec::new(x, y)) {
                    paint(x as u32, y as u32, [230, 30, 30]);
                }
            }
        }
    }
    let img = macroquad::texture::Image { bytes: px, width: (mw * scale) as u16, height: (mh * scale) as u16 };
    img.export_png(out);
    println!("seed {seed}: {mw}×{mh} cells, {out}");
    Ok(0)
}

fn soak(args: &[String]) -> Result<i32, String> {
    let o = opts(args, &["--seed", "--days"])?;
    let seed = o.get.get("--seed").ok_or("soak needs --seed")?.parse().map_err(|_| "the seed is a number")?;
    let days = num(&o, "--days", 2)?;
    Ok(match seeds::soak(&o.mods, &|_| true, seed, days) {
        Ok(hash) => {
            println!("seed {seed}: {}, saved and loaded, state hash {hash:016x}", days_text(days));
            0
        }
        Err(f) => {
            println!("seed {seed}: failed at tick {}: {}", f.tick, f.reason);
            1
        }
    })
}

fn sweep(args: &[String]) -> Result<i32, String> {
    let o = opts(args, &["--date", "--n", "--days", "--jobs", "--report"])?;
    let date = num(&o, "--date", 0)?;
    if date == 0 {
        return Err("sweep needs --date YYYYMMDD".into());
    }
    let (n, days) = (num(&o, "--n", 200)? as usize, num(&o, "--days", 2)?);
    let jobs = num(&o, "--jobs", std::thread::available_parallelism().map_or(4, |n| n.get() as u64))?.max(1) as usize;
    let all = seeds::date_seeds(date, n);
    println!("sweep {date}: {n} seeds, {} each, {jobs} at a time (the first is {})", days_text(days), all[0]);
    let mods = o.mods.clone();
    let chunks: Vec<Vec<u64>> = (0..jobs).map(|j| all.iter().skip(j).step_by(jobs).copied().collect()).collect();
    let mut failed: Vec<(u64, seeds::Failure)> = std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let mods = mods.clone();
                s.spawn(move || {
                    chunk
                        .into_iter()
                        .filter_map(|seed| seeds::soak(&mods, &|_| true, seed, days).err().map(|f| (seed, f)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    failed.sort_by_key(|f| f.0);
    let mut report = String::new();
    for (seed, f) in &failed {
        let line =
            format!("{seed}\t{}\t{}\t{}", f.tick, f.reason.replace(['\t', '\n'], " "), seeds::repro(*seed, days));
        println!("FAIL {line}");
        report += &line;
        report.push('\n');
    }
    if let Some(path) = o.get.get("--report") {
        std::fs::write(path, report).map_err(|e| format!("{path}: {e}"))?;
    }
    println!("sweep {date}: {} of {n} passed", n - failed.len());
    Ok(i32::from(!failed.is_empty()))
}
