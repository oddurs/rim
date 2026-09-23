//! Fight symmetry check: N colonists vs M raiders, identical humans, many seeds.
//!   cargo run --release -p rim_sim --example duel -- --colonists 1 --raiders 1 --trials 200
use rim_sim::world::{Faction, Pawn};
use rim_sim::Sim;
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let (nc, nr, trials) = (arg("--colonists", 1), arg("--raiders", 1), arg("--trials", 200));
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let (mut cw, mut rw, mut draw) = (0, 0, 0);
    let (mut c_dead, mut r_dead, mut r_fled, mut c_hits_taken, mut r_hits_taken) = (0, 0, 0, 0i64, 0i64);
    for seed in 0..trials {
        let mut s = Sim::new(&mods, 1000 + seed).unwrap();
        let founder = s.world.colonists().next().unwrap();
        let start = s.world.pawn_pos(founder).unwrap();
        for e in s.world.pawns.clone() {
            let _ = s.world.ecs.despawn(e);
        }
        s.world.pawns.clear();
        let human = s.world.defs.creature_id("human").unwrap();
        let mut cols = vec![];
        let mut raids = vec![];
        for i in 0..nc {
            cols.push(s.world.spawn_pawn(human, Faction::Player, start.offset(i as i32, 0), None));
        }
        for i in 0..nr {
            raids.push(s.world.spawn_pawn(human, Faction::Hostile, start.offset(i as i32, 6), None));
        }
        for _ in 0..6000 {
            s.step();
        }
        let alive = |f: Faction| {
            s.world
                .pawns
                .iter()
                .filter(|&&e| s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == f && !p.dead))
                .count()
        };
        let (ca, ra) = (alive(Faction::Player), alive(Faction::Hostile));
        c_dead += nc as usize - ca;
        let fled = s.world.messages.iter().filter(|m| m.text.ends_with("fled.")).count();
        r_fled += fled;
        r_dead += nr as usize - ra - fled;
        for &e in &cols {
            if let Ok(p) = s.world.ecs.get::<&Pawn>(e) {
                c_hits_taken += 100 - p.hp as i64;
            }
        }
        for &e in &raids {
            if let Ok(p) = s.world.ecs.get::<&Pawn>(e) {
                r_hits_taken += 100 - p.hp as i64;
            }
        }
        if ra == 0 && ca > 0 {
            cw += 1
        } else if ca == 0 {
            rw += 1
        } else {
            draw += 1
        }
    }
    println!("{nc} colonists vs {nr} raiders, {trials} trials");
    println!("colony wins {cw} · raiders win {rw} · unresolved {draw}");
    println!("colonists dead {c_dead} · raiders dead {r_dead} · raiders fled {r_fled}");
    println!("damage on survivors: colonists {c_hits_taken} · raiders {r_hits_taken}");
}
