//! Speech: what pawns say, for renderers (DESIGN.md §11). Needs speak as
//! they run low, scripts speak with rim.say, and none of it touches the sim.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Pawn, NEED_MAX, SPEECH_CHARS, SPEECH_LOG};
use rim_sim::Sim;

fn lines(s: &Sim, e: Entity) -> Vec<String> {
    s.world.speech.iter().filter(|l| l.pawn == e).map(|l| l.text.clone()).collect()
}

/// Set pawn `e`'s need `id` to `frac` of full.
fn set_need(s: &mut Sim, e: Entity, id: &str, frac: f64) {
    let need = s.world.defs.needs.iter().position(|n| n.id.ends_with(id)).expect("the need") as u16;
    let mut p = s.world.ecs.get::<&mut Pawn>(e).unwrap();
    let n = p.needs.iter_mut().find(|n| n.0 == need).expect("the pawn has it");
    n.1 = (frac * NEED_MAX as f64) as i32;
}

#[test]
fn a_need_speaks_once_as_it_runs_low() {
    let mut s = Sim::new(&common::mods(), 21).unwrap();
    s.step();
    let founder = s.world.colonists().next().unwrap();
    let say =
        s.world.defs.needs.iter().find(|n| n.id.ends_with("food")).and_then(|n| n.say.clone()).expect("food speaks");
    // Just above the line: it drains across it on its own.
    set_need(&mut s, founder, "food", say.below + 0.002);
    for _ in 0..3000 {
        s.step();
        if !lines(&s, founder).is_empty() {
            break;
        }
    }
    let said = lines(&s, founder);
    assert_eq!(said.len(), 1, "one line as it crossed: {said:?}");
    assert!(say.lines.contains(&said[0]), "{said:?}");
    // Held low, it says nothing more.
    for _ in 0..3000 {
        set_need(&mut s, founder, "food", say.below / 2.0);
        s.step();
    }
    assert_eq!(lines(&s, founder).len(), 1, "nothing more while it stayed low");
}

#[test]
fn animals_say_nothing() {
    let mut s = Sim::new(&common::mods(), 21).unwrap();
    s.step();
    let deer = s.world.defs.creature_id("deer").unwrap();
    let at = s.world.colony_center().unwrap();
    let e = s.world.spawn_pawn(deer, rim_sim::world::Faction::Wild, at, None);
    if s.world.ecs.get::<&Pawn>(e).unwrap().needs.iter().any(|n| s.world.defs.need(n.0).say.is_some()) {
        set_need(&mut s, e, "food", 0.21);
        for _ in 0..3000 {
            s.step();
        }
        assert!(lines(&s, e).is_empty());
    }
}

#[test]
fn a_script_says_a_line_cut_to_length() {
    let script = r#"
local done = false
rim.every(5, function()
    if done then return end
    done = true
    local x, y = rim.colony_center()
    local id = rim.spawn_pawn("core:human", "player", x, y)
    rim.say(id, string.rep("word ", 60), 50, 7)
end)
"#;
    let dir = common::test_mods("speech-say", &["core"], &[("talker", &[("scripts/talk.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap();
    for _ in 0..20 {
        s.step();
    }
    let l = s.world.speech.back().expect("a line");
    assert_eq!((l.ticks, l.priority), (50, 7));
    assert!(l.text.chars().count() <= SPEECH_CHARS + 1 && l.text.ends_with('…'), "{:?}", l.text);
}

#[test]
fn speech_changes_nothing_a_save_holds() {
    let mut s = Sim::new(&common::mods(), 21).unwrap();
    s.step();
    let founder = s.world.colonists().next().unwrap();
    let before = Snapshot::capture(&s).hash();
    for i in 0..SPEECH_LOG + 10 {
        s.world.say(founder, &format!("line {i}"), 100, 2);
    }
    assert_eq!(Snapshot::capture(&s).hash(), before);
    assert_eq!(s.world.speech.len(), SPEECH_LOG, "the log keeps the newest");
    assert_eq!(s.world.speech.back().unwrap().text, format!("line {}", SPEECH_LOG + 9));
}
