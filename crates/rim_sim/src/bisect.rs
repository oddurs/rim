//! Where two runs of the same game part (DESIGN.md §7): the first tick and
//! the snapshot sections that differ, from two games stepped side by side,
//! from two saves' logs, or from two platforms' traces of one day.

use crate::savefile::{differing, section_hashes, EpochRead};
use crate::Sim;
use std::collections::BTreeMap;

/// The first tick two runs differ at, and the sections that do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parting {
    pub tick: u64,
    pub sections: Vec<String>,
}

/// Steps `a` and `b` together until `until`, comparing their state after
/// every tick; the first tick they differ at, with its sections. Two games
/// that already differ part at the tick they're at.
pub fn lockstep(a: &mut Sim, b: &mut Sim, until: u64) -> Option<Parting> {
    loop {
        if a.world.state_hash() != b.world.state_hash() {
            let sections = differing(&section_hashes(a), &section_hashes(b));
            return Some(Parting { tick: a.world.tick, sections });
        }
        if a.world.tick >= until {
            return None;
        }
        a.step();
        b.step();
    }
}

/// Two saves of one game: the last log tick where every section agreed, and
/// the first log where one didn't. The part lies between them.
pub fn logs(a: &EpochRead, b: &EpochRead) -> Option<(u64, Parting)> {
    // Where the epoch began: its first snapshot, or a new game's tick 0.
    let mut agreed = a.snapshots.first().map_or(0, |s| s.header.tick);
    for (x, y) in a.logs.iter().zip(&b.logs) {
        if x.tick != y.tick {
            return Some((
                agreed,
                Parting { tick: x.tick.min(y.tick), sections: vec!["(the logs are at different ticks)".into()] },
            ));
        }
        let sections = differing(&x.hashes, &y.hashes);
        if !sections.is_empty() {
            return Some((agreed, Parting { tick: x.tick, sections }));
        }
        agreed = x.tick;
    }
    None
}

/// One tick of a trace: `tick 1234 engine:pawn=0123abcd weather:data=...`.
pub fn trace_line(sim: &Sim) -> String {
    let hashes: Vec<String> = section_hashes(sim).iter().map(|(n, h)| format!("{n}={h:016x}")).collect();
    format!("tick {} {}", sim.world.tick, hashes.join(" "))
}

fn parse_trace(text: &str) -> BTreeMap<u64, BTreeMap<String, u64>> {
    text.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            (words.next()? == "tick").then_some(())?;
            let tick = words.next()?.parse().ok()?;
            let hashes = words
                .filter_map(|w| {
                    let (n, h) = w.split_once('=')?;
                    Some((n.to_string(), u64::from_str_radix(h, 16).ok()?))
                })
                .collect();
            Some((tick, hashes))
        })
        .collect()
}

/// Two platforms' traces of the same day: the first tick both have that
/// differs, and its sections.
pub fn traces(a: &str, b: &str) -> Option<Parting> {
    let (a, b) = (parse_trace(a), parse_trace(b));
    a.iter().find_map(|(tick, x)| {
        let y = b.get(tick)?;
        let sections = differing(x, y);
        (!sections.is_empty()).then_some(Parting { tick: *tick, sections })
    })
}
