//! Priority rules and stances: priorities that change with the hour, the
//! season, a colonist's needs or the colony's stance, and say why.

mod common;

use rim_sim::data::Data;
use rim_sim::defs::DefId;
use rim_sim::rules::{explain, Part};
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Pawn, NEED_MAX};
use rim_sim::{Command, Sim};

fn part(label: &str, delta: i32) -> Part {
    Part { label: label.into(), delta }
}

fn explained(s: &Sim, work: &str) -> (u8, Vec<Part>) {
    let pawn = s.world.colonists().next().unwrap();
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    explain(&s.world.defs, &s.world.rules, &p, s.world.defs.lookup("work_type", work).unwrap())
}

fn stance(s: &Sim, id: &str) -> DefId {
    s.world.defs.lookup("stance", id).unwrap()
}

#[test]
fn effective_priorities_explain_themselves() {
    let mut s = Sim::new(&common::mods(), 1).unwrap();
    assert_eq!(s.world.stance, Some(stance(&s, "core:normal")), "a colony starts in the first stance");
    assert_eq!(explained(&s, "core:build"), (3, vec![part("base", 3)]));

    s.push(Command::SetStance { stance: stance(&s, "core:siege") });
    s.step();
    assert_eq!(explained(&s, "core:build"), (1, vec![part("base", 3), part("Siege", -2)]));
    assert_eq!(explained(&s, "core:hunt"), (0, vec![part("base", 3), part("Siege", -3)]), "set to never");
    assert_eq!(explained(&s, "core:mine"), (4, vec![part("base", 3), part("Siege", 1)]));

    // Past the first level: the rule held but couldn't move it.
    let pawn = s.world.colonists().next().unwrap();
    let build = s.world.defs.lookup("work_type", "core:build").unwrap();
    s.push(Command::SetPriority { pawn, work: build, level: 1 });
    s.step();
    assert_eq!(explained(&s, "core:build"), (1, vec![part("base", 1), part("Siege", 0)]));
    // A player's never survives a shift.
    let mine = s.world.defs.lookup("work_type", "core:mine").unwrap();
    s.push(Command::SetPriority { pawn, work: mine, level: 0 });
    s.step();
    assert_eq!(explained(&s, "core:mine"), (0, vec![part("base", 0), part("Siege", 0)]));

    for w in &s.world.defs.work_types {
        let (v, parts) = explained(&s, &w.id);
        assert_eq!(parts.iter().map(|p| p.delta).sum::<i32>(), v as i32, "{}: the parts sum to the value", w.id);
    }
}

/// A stance a mod adds with data alone, a rule on the hour and one on a
/// colonist's need.
fn modded(name: &str) -> std::path::PathBuf {
    common::test_mods(
        name,
        &["core"],
        &[(
            "drills",
            &[(
                "defs/stances.toml",
                r#"
[[stance]]
id = "drill"
label = "Drill"
order = 40

[[priority_rule]]
id = "drill"
when = { stance = "drill" }
set = { "core:haul" = 1 }

[[priority_rule]]
id = "night_hauling"
label = "Night"
when = { hours = [22, 6] }
shift = { "core:haul" = 1 }

[[priority_rule]]
id = "tired"
label = "Tired"
when = { need = "core:rest", below = 0.5 }
shift = { "core:chop" = 1 }
"#,
            )],
        )],
    )
}

#[test]
fn a_mod_adds_a_stance_and_rules_with_data_alone() {
    let dir = modded("stances-data");
    let mut s = Sim::new(&dir, 1).unwrap();
    let drill = stance(&s, "drills:drill");
    s.push(Command::SetStance { stance: drill });
    s.step();
    assert_eq!(explained(&s, "core:haul").0, 1);
    assert_eq!(explained(&s, "core:haul").1[1], part("Drill", -2));

    // The need rule is the colonist's own.
    let pawn = s.world.colonists().next().unwrap();
    let rest = s.world.defs.lookup("need", "core:rest").unwrap();
    let set_rest = |s: &mut Sim, v: i32| {
        let mut p = s.world.ecs.get::<&mut Pawn>(pawn).unwrap();
        p.needs.iter_mut().find(|n| n.0 == rest).unwrap().1 = v;
    };
    set_rest(&mut s, NEED_MAX);
    assert_eq!(explained(&s, "core:chop").0, 3);
    set_rest(&mut s, NEED_MAX / 4);
    assert_eq!(explained(&s, "core:chop"), (4, vec![part("base", 3), part("Tired", 1)]));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rules_are_worked_out_only_when_their_inputs_change() {
    // Core's rules read only the stance: a day passes without a look.
    let mut s = Sim::new(&common::mods(), 1).unwrap();
    let n = s.world.rules.evaluations;
    for _ in 0..rim_sim::TICKS_PER_DAY {
        s.step();
    }
    assert_eq!(s.world.rules.evaluations, n, "nothing a rule reads changed");
    s.push(Command::SetStance { stance: stance(&s, "core:harvest") });
    s.step();
    assert_eq!(s.world.rules.evaluations, n + 1, "the stance changed");

    // An hour rule: once an hour, and it holds across midnight.
    let dir = modded("stances-hours");
    let mut s = Sim::new(&dir, 1).unwrap();
    let n = s.world.rules.evaluations;
    let mut night = Vec::new();
    for _ in 0..rim_sim::TICKS_PER_DAY {
        // The rules are worked out at the start of a step, for its hour.
        let h = s.world.hour() as u32;
        s.step();
        let on = explained(&s, "core:haul").1.iter().any(|p| p.label == "Night");
        night.push((h, on));
    }
    assert_eq!(s.world.rules.evaluations - n, 23, "once an hour: a day's 24, the first worked out at the start");
    assert!(night.iter().all(|&(h, on)| on == !(6..22).contains(&h)), "night is 22:00 to 06:00");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn stance_changes_replay_and_survive_a_load() {
    let run = |save_at: Option<u64>| {
        let mut s = Sim::new(&common::mods(), 7).unwrap();
        let (siege, harvest) = (stance(&s, "core:siege"), stance(&s, "core:harvest"));
        for t in 0..3000u64 {
            match t {
                500 => s.push(Command::SetStance { stance: siege }),
                1500 => s.push(Command::SetStance { stance: harvest }),
                _ => {}
            }
            if Some(t) == save_at {
                s = Snapshot::capture(&s).restore(&common::mods(), &|_| true).unwrap();
            }
            s.step();
        }
        (s.world.state_hash(), s.world.stance)
    };
    let a = run(None);
    assert_eq!(a, run(None), "two runs agree");
    assert_eq!(a, run(Some(1000)), "a load in siege carries on the same");
    assert_eq!(a.1, run(None).1);
}

#[test]
fn a_stance_from_a_removed_mod_falls_back_to_the_first() {
    let dir = modded("stances-removed");
    let mut s = Sim::new(&dir, 1).unwrap();
    s.push(Command::SetStance { stance: stance(&s, "drills:drill") });
    s.step();
    let (back, notes) = Snapshot::capture(&s).restore_noting(&dir, &|m| m != "drills").unwrap();
    assert_eq!(back.world.stance, back.world.defs.lookup("stance", "core:normal"));
    assert!(notes.iter().any(|n| n.contains("drills:drill")), "{notes:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn scripts_read_priorities_with_the_rules_and_set_the_stance() {
    let dir = common::test_mods(
        "stance-script",
        &["core"],
        &[(
            "alarm",
            &[(
                "scripts/main.luau",
                r#"
rim.every(10, function()
    if rim.tick() >= 20 and rim.stance() == "core:normal" then
        rim.set_stance("core:siege")
    end
    local id = rim.get_data("alarm:who")
    local parts = rim.priority_parts(id, "core:build")
    local sum = 0
    for _, p in parts do
        sum += p.delta
    end
    rim.set_data("alarm:check", { build = rim.priority(id, "core:build"), sum = sum })
end)
"#,
            )],
        )],
    );
    let mut s = Sim::new(&dir, 1).unwrap();
    let pawn = s.world.colonists().next().unwrap();
    s.world.data.insert("alarm:who".into(), Data::Int(pawn.to_bits().get() as i64));
    for _ in 0..41 {
        s.step();
    }
    assert_eq!(s.world.stance, Some(stance(&s, "core:siege")));
    let check = format!("{:?}", s.world.data.get("alarm:check"));
    assert!(check.contains("\"build\"") && check.matches("Int(1)").count() == 2, "{check}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_rule_names_what_exists() {
    let bad = |name: &str, body: &str| {
        let dir = common::test_mods(name, &["core"], &[("bad", &[("defs/r.toml", body)])]);
        let err = Sim::new(&dir, 1).err().expect("refused");
        let _ = std::fs::remove_dir_all(dir);
        err
    };
    let e = bad("rule-season", "[[priority_rule]]\nid = \"x\"\nwhen = { season = [\"monsoon\"] }\n");
    assert!(e.contains("no season 'monsoon'"), "{e}");
    let e = bad("rule-work", "[[priority_rule]]\nid = \"x\"\nshift = { nap = 1 }\n");
    assert!(e.contains("unknown work_type"), "{e}");
    let e = bad("rule-need", "[[priority_rule]]\nid = \"x\"\nwhen = { need = \"core:rest\" }\n");
    assert!(e.contains("`below` or `above`"), "{e}");
    let e = bad("rule-frac", "[[priority_rule]]\nid = \"x\"\nwhen = { need = \"core:rest\", below = 30 }\n");
    assert!(e.contains("from 0 to 1"), "{e}");
}

/// A shift of any size moves a priority to the end of the scale, never
/// around it.
#[test]
fn a_huge_shift_goes_to_the_end_of_the_scale() {
    let dir = common::test_mods(
        "rule-huge",
        &["core"],
        &[(
            "huge",
            &[(
                "defs/r.toml",
                "[[priority_rule]]\nid = \"x\"\nshift = { \"core:haul\" = 2147483647, \"core:chop\" = -2147483648 }\n",
            )],
        )],
    );
    let s = Sim::new(&dir, 1).unwrap();
    assert_eq!(explained(&s, "core:haul").0, 4, "last");
    assert_eq!(explained(&s, "core:chop").0, 1, "first");
    let _ = std::fs::remove_dir_all(dir);
}

/// The first work a colonist takes up with a wall to raise and trees to
/// fell, chop set sooner than build.
fn first_work(stance_id: Option<&str>) -> &'static str {
    use rim_sim::world::Job;
    // Core alone: with the stone age on, the oaks would wait for an axe.
    let mut s = Sim::with_mods(&common::mods(), 2, &|m| m == "core").unwrap();
    let defs = s.world.defs.clone();
    let pawn = s.world.colonists().next().unwrap();
    let c = s.world.pawn_pos(pawn).unwrap();
    let wood = defs.thing_id("wood").unwrap();
    s.world.place_item(wood, c.offset(1, 1), 60);
    let chop = defs.lookup("work_type", "core:chop").unwrap();
    s.push(Command::SetPriority { pawn, work: chop, level: 2 });
    let chop_d = defs.lookup("designation", "chop").unwrap();
    s.push(Command::Designate { designation: chop_d, a: c.offset(-15, -15), b: c.offset(15, 15) });
    let wall = defs.thing_id("wall").unwrap();
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-6, 4), b: c.offset(-2, 4) });
    if let Some(id) = stance_id {
        s.push(Command::SetStance { stance: stance(&s, id) });
    }
    for _ in 0..2000 {
        s.step();
        match s.world.ecs.get::<&Pawn>(pawn).unwrap().job {
            Job::Harvest { .. } => return "chop",
            Job::Construct { .. } | Job::Deliver { .. } => return "build",
            _ => {}
        }
    }
    "idle"
}

#[test]
fn a_stance_changes_what_colonists_do() {
    assert_eq!(first_work(None), "chop", "chop 2 before build 3");
    assert_eq!(first_work(Some("core:siege")), "build", "a siege raises build to 1 and drops chop to 3");
}
