//! Auto, the mechanism (DESIGN.md §4d): a planned work role whose levels a
//! script planner sets each in-game hour. The engine stores the plan, lets
//! a level change only when two plans agree, refuses never and pinned
//! cells, and explains each planned level with the planner's reason.

mod common;

use rim_sim::rules::{explain, PartKind};
use rim_sim::snapshot::Snapshot;
use rim_sim::world::Pawn;
use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::path::PathBuf;

const HOUR: u64 = TICKS_PER_DAY / 24;

/// Core, and a mod with a planned role first and a planner that reads its
/// orders from script data: `{ [work] = level }`, or 0 / a pinned cell to
/// test refusals.
fn planned(name: &str) -> PathBuf {
    let defs = r#"
[[work_role]]
id = "auto"
label = "Auto"
order = -10
planner = "planning:test"
"#;
    let script = r#"
rim.planner("test", function(board)
    local want = rim.get_data("planning:want") or {}
    local plans = {}
    for _, c in board.colonists do
        if c.member then
            local cells = {}
            for work, level in want do
                cells[work] = { level = level, reason = "wanted at " .. level }
            end
            plans[c.id] = cells
        end
    end
    rim.set_data("planning:seen", { members = #board.colonists, work = #board.work, levels = board.levels })
    return plans
end)
"#;
    common::test_mods(name, &["core"], &[("planning", &[("defs/roles.toml", defs), ("scripts/main.luau", script)])])
}

fn want(s: &mut Sim, cells: &[(&str, i64)]) {
    let mut t = std::collections::BTreeMap::new();
    for (w, l) in cells {
        t.insert(rim_sim::data::Key::Str(w.to_string()), rim_sim::data::Data::Int(*l));
    }
    s.world.data.insert("planning:want".into(), rim_sim::data::Data::Table(t));
}

/// A level before the rules: what the plan, a pin or a role set, without
/// core's standing orders shifting it.
fn level(s: &Sim, w: &str) -> u8 {
    let pawn = s.world.colonists().next().unwrap();
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    s.world.base_priority(&p, s.world.defs.lookup("work_type", w).unwrap())
}

fn hours(s: &mut Sim, n: u64) {
    for _ in 0..n * HOUR {
        s.step();
    }
}

#[test]
fn colonists_start_in_the_planned_role_and_the_planner_reads_the_board() {
    let dir = planned("auto-board");
    let mut s = Sim::new(&dir, 1).unwrap();
    let pawn = s.world.colonists().next().unwrap();
    assert!(s.world.is_planned(&s.world.ecs.get::<&Pawn>(pawn).unwrap()), "the first role by order is Auto");
    hours(&mut s, 1);
    let seen = format!("{:?}", s.world.data.get("planning:seen"));
    assert!(seen.contains("\"levels\"") && seen.contains("Int(4)"), "the planner got the board: {seen}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_planned_level_changes_on_the_second_agreeing_plan() {
    let dir = planned("auto-two");
    let mut s = Sim::new(&dir, 1).unwrap();
    want(&mut s, &[("core:haul", 1)]);
    hours(&mut s, 1);
    assert_eq!(level(&s, "core:haul"), 1, "a work type with no plan takes the first");
    want(&mut s, &[("core:haul", 4)]);
    hours(&mut s, 1);
    assert_eq!(level(&s, "core:haul"), 1, "one plan isn't enough to move it");
    hours(&mut s, 1);
    assert_eq!(level(&s, "core:haul"), 4, "two in a row are");
    // A plan that wavers never moves it.
    for l in [2, 3, 2, 3] {
        want(&mut s, &[("core:haul", l)]);
        hours(&mut s, 1);
    }
    assert_eq!(level(&s, "core:haul"), 4, "no two plans agreed");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_planner_can_say_neither_never_nor_over_a_pin() {
    let dir = planned("auto-refuse");
    let mut s = Sim::new(&dir, 1).unwrap();
    let pawn = s.world.colonists().next().unwrap();
    let chop = s.world.defs.lookup("work_type", "core:chop").unwrap();
    s.push(Command::SetPriority { pawn, work: chop, level: 2 });
    want(&mut s, &[("core:hunt", 0), ("core:chop", 4), ("core:mine", 1)]);
    s.world.messages.clear();
    hours(&mut s, 1);
    assert_eq!(level(&s, "core:hunt"), 3, "never refused: the default stands");
    assert_eq!(level(&s, "core:chop"), 2, "the pin stands");
    assert_eq!(level(&s, "core:mine"), 1, "the rest of the plan applies");
    let said: Vec<String> = s.world.messages.iter().map(|m| m.text.clone()).collect();
    assert!(said.iter().any(|m| m.contains("planning:test") && m.contains("core:hunt")), "{said:?}");
    assert!(said.iter().any(|m| m.contains("core:chop") && m.contains("pinned")), "{said:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_planned_level_explains_itself_and_leaving_drops_the_plan() {
    let dir = planned("auto-leave");
    let mut s = Sim::new(&dir, 1).unwrap();
    want(&mut s, &[("core:mine", 1)]);
    hours(&mut s, 1);
    let pawn = s.world.colonists().next().unwrap();
    let mine = s.world.defs.lookup("work_type", "core:mine").unwrap();
    {
        let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
        let (v, parts) = explain(&s.world, &p, mine);
        assert_eq!(v, 1);
        assert_eq!(parts[1].kind, PartKind::Plan);
        assert_eq!(parts[1].label, "wanted at 1", "the planner's reason");
    }
    let hunt = s.world.defs.lookup("work_type", "core:hunt").unwrap();
    s.push(Command::SetPriority { pawn, work: hunt, level: 4 });
    let builder = s.world.work_roles.iter().position(|r| r.label == "Builder").unwrap() as u16;
    s.push(Command::AssignWorkRole { pawn, role: builder });
    s.step();
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    assert!(p.plan.is_empty() && p.proposal.is_empty(), "the plan was Auto's");
    assert_eq!(p.own_priority(hunt), Some(4), "pins stay");
    drop(p);
    assert_eq!(level(&s, "core:mine"), 2, "Builder's now");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_planned_roles_levels_are_not_the_players_to_edit() {
    let dir = planned("auto-edit");
    let mut s = Sim::new(&dir, 1).unwrap();
    let haul = s.world.defs.lookup("work_type", "core:haul").unwrap();
    let auto = s.world.work_roles.iter().position(|r| r.planner.is_some()).unwrap();
    s.push(Command::SetRolePriority { role: auto as u16, work: haul, level: Some(1) });
    s.step();
    let r = &s.world.work_roles[auto];
    assert!(r.priorities.is_empty() && !r.edited, "{r:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn plans_survive_a_load_and_replay_the_same() {
    let dir = planned("auto-load");
    let run = |save_at: Option<u64>| {
        let mut s = Sim::new(&dir, 5).unwrap();
        for t in 0..8 * HOUR {
            match t / HOUR {
                0 => want(&mut s, &[("core:haul", 1), ("core:mine", 2)]),
                3 => want(&mut s, &[("core:haul", 3), ("core:mine", 2)]),
                _ => {}
            }
            if Some(t) == save_at {
                s = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
            }
            s.step();
        }
        s.world.state_hash()
    };
    let h = run(None);
    assert_eq!(h, run(None), "two runs agree");
    assert_eq!(h, run(Some(4 * HOUR + 7)), "a load between two agreeing plans carries on the same");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_mod_replaces_the_planner_by_patching_the_role() {
    let dir = planned("auto-patch");
    let other = r#"
rim.planner("mine", function(board)
    local plans = {}
    for _, c in board.colonists do
        if c.member then
            plans[c.id] = { ["core:build"] = { level = 1, reason = "mine says build" } }
        end
    end
    return plans
end)
"#;
    let patch = "[[patch]]\ntarget = \"work_role/planning:auto\"\nset = { planner = \"other:mine\" }\n";
    let extra = dir.join("other");
    std::fs::create_dir_all(extra.join("scripts")).unwrap();
    std::fs::create_dir_all(extra.join("defs")).unwrap();
    std::fs::write(extra.join("scripts/main.luau"), other).unwrap();
    std::fs::write(extra.join("defs/patch.toml"), patch).unwrap();
    std::fs::write(
        extra.join("mod.toml"),
        "id = \"other\"\nname = \"Other\"\nversion = \"0.1.0\"\napi = \"0.6\"\ndepends = [\"planning\"]\n",
    )
    .unwrap();
    let mut s = Sim::new(&dir, 1).unwrap();
    want(&mut s, &[("core:haul", 1)]);
    hours(&mut s, 1);
    assert_eq!(level(&s, "core:build"), 1, "the patched planner ran");
    assert_eq!(level(&s, "core:haul"), 3, "the original didn't");
    let _ = std::fs::remove_dir_all(dir);
}
