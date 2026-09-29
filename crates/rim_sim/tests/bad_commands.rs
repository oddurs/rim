//! A command is untrusted input: a co-op peer or a hand-edited log can send
//! any def id, and none may panic the sim (6bf967c9).

mod common;

use rim_sim::command::Command;
use rim_sim::filter::FilterEdit;
use rim_sim::zone::StoreRef;
use rim_sim::Sim;

const BAD: u16 = u16::MAX;

#[test]
fn no_command_with_an_unknown_def_id_panics_the_sim() {
    let base = Sim::new(&common::mods(), 1).expect("mods load");
    let c = base.world.colony_center().expect("a colony");
    let (a, b) = (c.offset(-2, -2), c.offset(2, 2));
    let wood = base.world.defs.thing_id("wood").unwrap();
    let wall = base.world.defs.thing_id("wall").unwrap();
    let edits = [
        FilterEdit::Thing { thing: BAD, on: true },
        FilterEdit::Category { category: BAD, on: true },
        FilterEdit::Material { material: BAD, on: true },
    ];
    let mut commands = vec![
        Command::Designate { designation: BAD, a, b },
        Command::Build { thing: BAD, stuff: None, a, b, facing: 0 },
        Command::Build { thing: wall, stuff: Some(BAD), a, b, facing: 0 },
        Command::Build { thing: wall, stuff: Some(wood), a, b, facing: 200 },
        Command::PlacePlan { plan: BAD, at: c, facing: 0, stuff: None },
        Command::PlacePlan { plan: 0, at: c, facing: 0, stuff: Some(BAD) },
        Command::GrowZone { a, b, zone: None, plant: BAD },
        Command::ZonePlant { zone: 1, plant: BAD },
        Command::SetRuleEnabled { rule: BAD, on: false },
        Command::SetStance { stance: BAD },
        Command::SetRolePriority { role: 0, work: BAD, level: Some(1) },
    ];
    for edit in edits {
        commands.push(Command::ZoneAllow { zone: 1, thing: BAD, on: true });
        commands.push(Command::StoreFilter { store: StoreRef::Zone(1), edit });
    }
    let mut panicked = Vec::new();
    for cmd in commands {
        let mut s = Sim::new(&common::mods(), 1).expect("mods load");
        s.push(Command::Stockpile { a, b, zone: None });
        s.step();
        let shown = format!("{cmd:?}");
        s.push(cmd);
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.step())).is_err() {
            panicked.push(shown);
        }
    }
    assert!(panicked.is_empty(), "these panicked the sim:\n{}", panicked.join("\n"));
}
