//! Finding where two runs of one game part (DESIGN.md §7): a draw that
//! differs at one tick, as one platform's float or order would, is found at
//! that tick, and the section it lives in is named.

use rim_sim::bisect::{self, Parting};
use rim_sim::rng::SPAWNS;
use rim_sim::Sim;
use std::path::Path;

fn game() -> Sim {
    Sim::build(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 4, &|m| m == "core", 64).unwrap()
}

/// One extra draw from the spawn stream at `at`, and nothing else.
fn drift(b: &mut Sim, at: u64) {
    while b.world.tick < at {
        b.step();
    }
    let _ = b.world.streams.stream(SPAWNS).next_u64();
}

#[test]
fn two_games_stepped_together_part_at_the_tick_a_draw_changed() {
    let (mut a, mut b) = (game(), game());
    let at = 1234;
    drift(&mut b, at);
    while a.world.tick < at {
        a.step();
    }
    let part = bisect::lockstep(&mut a, &mut b, at + 500).expect("they part");
    assert_eq!(part.tick, at, "{part:?}");
    assert!(part.sections.iter().any(|s| s.contains("world")), "the world section holds the streams: {part:?}");
    assert!(bisect::lockstep(&mut game(), &mut game(), 300).is_none(), "one game twice never parts");
}

#[test]
fn two_platforms_traces_part_at_the_first_tick_they_differ() {
    let (mut a, mut b) = (game(), game());
    let at = 1234;
    let (mut ta, mut tb) = (String::new(), String::new());
    for _ in 0..at + 20 {
        if b.world.tick == at {
            let _ = b.world.streams.stream(SPAWNS).next_u64();
        }
        a.step();
        b.step();
        if a.world.tick >= at - 10 {
            ta += &(bisect::trace_line(&a) + "\n");
            tb += &(bisect::trace_line(&b) + "\n");
        }
    }
    let part = bisect::traces(&ta, &tb).expect("they part");
    assert_eq!(
        part.tick,
        at + 1,
        "the draw lands at the end of tick {at}, so tick {} is the first state that differs",
        at + 1
    );
    assert!(!part.sections.is_empty());
    assert_eq!(bisect::traces(&ta, &ta), None::<Parting>);
}

/// Two saves of one game, one from a machine whose draw changed at tick
/// 1234: their logs agree through the log before it and part at the next,
/// naming the section.
#[test]
fn two_saves_part_at_the_first_log_that_disagrees() {
    use rim_sim::savefile::{read, SaveFile};
    let dir = std::env::temp_dir().join(format!("rim-bisect-saves-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let play = |name: &str, drift_at: Option<u64>| {
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        let mut s = game();
        let mut save = SaveFile::create(&path, &mut s).unwrap();
        for _ in 0..6 {
            for _ in 0..500 {
                s.step();
                if Some(s.world.tick) == drift_at {
                    let _ = s.world.streams.stream(SPAWNS).next_u64();
                }
            }
            save.log(&mut s).unwrap();
        }
        read(&path).unwrap().0.pop().unwrap()
    };
    let (a, b) = (play("a.rim", None), play("b.rim", Some(1234)));
    let (agreed, part) = bisect::logs(&a, &b).expect("they part");
    assert_eq!((agreed, part.tick), (1000, 1500), "{part:?}");
    assert!(part.sections.iter().any(|s| s.contains("world")), "{part:?}");
    assert_eq!(bisect::logs(&a, &a), None);
    let _ = std::fs::remove_dir_all(dir);
}
