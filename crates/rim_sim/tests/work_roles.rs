//! Work roles (DESIGN.md §4d): partial sets of levels colonists belong to,
//! one each. A colonist's level starts at their role's (or the work type's
//! default), their pin beats it, and the rules come last.

mod common;

use rim_sim::command::RoleSource;
use rim_sim::defs::DefId;
use rim_sim::rules::{explain, PartKind};
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{Command, Sim};

fn work(s: &Sim, id: &str) -> DefId {
    s.world.defs.lookup("work_type", id).unwrap()
}

fn role(s: &Sim, label: &str) -> u16 {
    s.world.work_roles.iter().position(|r| r.label == label).unwrap_or_else(|| panic!("no role {label}")) as u16
}

fn level(s: &Sim, pawn: rim_sim::hecs::Entity, w: &str) -> u8 {
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    rim_sim::rules::effective(&s.world, &p, work(s, w))
}

/// Two colonists, core only.
fn pair() -> (Sim, rim_sim::hecs::Entity, rim_sim::hecs::Entity) {
    let mut s = Sim::with_mods(&common::mods(), 1, &|m| m == "core").unwrap();
    let a = s.world.colonists().next().unwrap();
    let human = s.world.defs.creature_id("human").unwrap();
    let at = s.world.pawn_pos(a).unwrap().offset(1, 0);
    let b = s.world.spawn_pawn(human, Faction::Player, at, None);
    common::hands(&mut s);
    (s, a, b)
}

#[test]
fn core_seeds_its_roles_and_colonists_start_in_the_first() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let labels: Vec<&str> = s.world.work_roles.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, ["Auto", "Hand", "Builder", "Forager", "Crafter"], "core's, then the crafting mod's");
    let pawn = s.world.colonists().next().unwrap();
    assert_eq!(s.world.ecs.get::<&Pawn>(pawn).unwrap().work_role, Some(role(&s, "Auto")), "Auto comes first by order");
    let builder = &s.world.work_roles[role(&s, "Builder") as usize];
    assert_eq!(builder.level(work(&s, "crafting:craft")), Some(4), "the crafting mod patched core's builder");
    assert!(!builder.edited && builder.def.as_deref() == Some("core:builder"));
}

#[test]
fn editing_a_role_moves_every_member_without_a_pin() {
    let (mut s, a, b) = pair();
    let (builder, haul) = (role(&s, "Builder"), work(&s, "core:haul"));
    s.push(Command::AssignWorkRole { pawn: a, role: builder });
    s.push(Command::AssignWorkRole { pawn: b, role: builder });
    s.push(Command::SetPriority { pawn: b, work: haul, level: 3 });
    s.step();
    assert_eq!((level(&s, a, "core:haul"), level(&s, b, "core:haul")), (2, 3), "Builder hauls Soon; b pinned Later");
    assert_eq!(level(&s, a, "core:build"), 1);

    s.push(Command::SetRolePriority { role: builder, work: haul, level: Some(1) });
    s.step();
    assert_eq!(level(&s, a, "core:haul"), 1, "the role moved a");
    assert_eq!(level(&s, b, "core:haul"), 3, "b's pin beats the role");
    assert!(s.world.work_roles[builder as usize].edited);

    // Leaving a work type to the default.
    s.push(Command::SetRolePriority { role: builder, work: haul, level: None });
    s.step();
    assert_eq!(level(&s, a, "core:haul"), 3, "core's default");
}

#[test]
fn moving_to_another_role_keeps_pins() {
    let (mut s, a, _) = pair();
    let mine = work(&s, "core:mine");
    s.push(Command::SetPriority { pawn: a, work: mine, level: 4 });
    s.push(Command::AssignWorkRole { pawn: a, role: role(&s, "Builder") });
    s.step();
    assert_eq!(level(&s, a, "core:mine"), 4, "the pin, not Builder's Soon");
    assert_eq!(level(&s, a, "core:build"), 1, "the rest is Builder's");
    // A role that isn't there changes nothing.
    s.push(Command::AssignWorkRole { pawn: a, role: 99 });
    s.step();
    assert_eq!(s.world.ecs.get::<&Pawn>(a).unwrap().work_role, Some(role(&s, "Builder")));
}

#[test]
fn explain_names_the_default_the_role_the_pin_and_the_rules() {
    let (mut s, a, _) = pair();
    let (builder, mine) = (role(&s, "Builder"), work(&s, "core:mine"));
    s.push(Command::AssignWorkRole { pawn: a, role: builder });
    s.push(Command::SetPriority { pawn: a, work: mine, level: 3 });
    s.push(Command::SetStance { stance: s.world.defs.lookup("stance", "core:siege").unwrap() });
    s.step();
    let p = s.world.ecs.get::<&Pawn>(a).unwrap();
    let (value, parts) = explain(&s.world, &p, mine);
    let kinds: Vec<PartKind> = parts.iter().map(|p| p.kind).collect();
    assert_eq!(kinds, [PartKind::Default, PartKind::Role, PartKind::Pin, PartKind::Rule]);
    assert_eq!(parts[1].label, "Builder");
    assert_eq!(parts[2].label, p.name);
    assert_eq!(parts.iter().map(|p| p.delta).sum::<i32>(), value as i32, "the parts sum to the level");
    assert_eq!(value, 4, "Later, then Siege +1");
}

#[test]
fn a_role_made_from_a_colonist_keeps_only_what_differs() {
    let (mut s, a, b) = pair();
    let (build, haul) = (work(&s, "core:build"), work(&s, "core:haul"));
    s.push(Command::SetPriority { pawn: a, work: build, level: 1 });
    s.push(Command::SetPriority { pawn: a, work: haul, level: 2 });
    s.push(Command::SetPriority { pawn: a, work: work(&s, "core:mine"), level: 3 });
    s.push(Command::CreateWorkRole { label: "  Crew  ".into(), from: RoleSource::Pawn(a) });
    s.step();
    let crew = role(&s, "Crew");
    let r = &s.world.work_roles[crew as usize];
    assert_eq!(r.priorities, vec![(build, 1), (haul, 2)], "mine at Later is core's default, so it's left out");
    assert!(r.def.is_none() && r.edited, "the player's own");
    s.push(Command::AssignWorkRole { pawn: b, role: crew });
    s.push(Command::CreateWorkRole { label: "".into(), from: RoleSource::Role(crew) });
    s.step();
    assert_eq!(level(&s, b, "core:build"), 1);
    assert!(s.world.work_roles.iter().all(|r| !r.label.is_empty()), "a nameless role isn't made");
}

/// A role mod `probe` defines, hauling at `haul`.
fn probe(name: &str, haul: u8) -> std::path::PathBuf {
    let defs = format!(
        "[[work_role]]\nid = \"porter\"\nlabel = \"Porter\"\norder = 5\npriorities = {{ \"core:haul\" = {haul} }}\n"
    );
    common::test_mods(name, &["core"], &[("probe", &[("defs/roles.toml", &defs)])])
}

#[test]
fn a_role_follows_its_mod_on_load_until_the_player_edits_it() {
    let (before, after) = (probe("roles-v1", 2), probe("roles-v2", 1));
    let mut s = Sim::new(&before, 1).unwrap();
    let pawn = s.world.colonists().next().unwrap();
    let porter = role(&s, "Porter");
    assert_eq!(porter, 4, "after core's four");
    s.push(Command::AssignWorkRole { pawn, role: porter });
    s.step();
    assert_eq!(level(&s, pawn, "core:haul"), 2);

    let back = Snapshot::capture(&s).restore(&after, &|_| true).unwrap();
    assert_eq!(level(&back, pawn, "core:haul"), 1, "unedited: it follows the mod's update");

    s.push(Command::SetRolePriority { role: porter, work: work(&s, "core:mine"), level: Some(4) });
    s.step();
    let back = Snapshot::capture(&s).restore(&after, &|_| true).unwrap();
    assert_eq!(level(&back, pawn, "core:haul"), 2, "edited: the player's copy stays");
    assert_eq!(level(&back, pawn, "core:mine"), 4);
    let _ = std::fs::remove_dir_all(before);
    let _ = std::fs::remove_dir_all(after);
}

#[test]
fn a_save_from_before_roles_loads_everyone_into_the_first() {
    let (mut s, a, b) = pair();
    s.push(Command::SetPriority { pawn: a, work: work(&s, "core:chop"), level: 1 });
    s.step();
    let levels = |s: &Sim, e| -> Vec<u8> {
        (0..s.world.defs.work_types.len()).map(|w| level(s, e, &s.world.defs.work_types[w].id)).collect()
    };
    let (la, lb) = (levels(&s, a), levels(&s, b));
    // What a save from before roles holds: no roles, colonists in none.
    s.world.work_roles.clear();
    for e in [a, b] {
        s.world.ecs.get::<&mut Pawn>(e).unwrap().work_role = None;
    }
    let back = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
    assert_eq!(back.world.work_roles.len(), 4, "core's roles are seeded");
    assert_eq!(back.world.work_role_of(&back.world.ecs.get::<&Pawn>(a).unwrap()), Some(0), "the first, Auto");
    assert_eq!((levels(&back, a), levels(&back, b)), (la, lb), "nobody's work changes until Auto's first plan");
}

#[test]
fn roles_survive_a_load_and_replay_the_same() {
    let run = |save_at: Option<u64>| {
        let (mut s, a, b) = pair();
        let (builder, forager, haul) = (role(&s, "Builder"), role(&s, "Forager"), work(&s, "core:haul"));
        for t in 0..1200u64 {
            match t {
                100 => s.push(Command::AssignWorkRole { pawn: a, role: builder }),
                200 => s.push(Command::AssignWorkRole { pawn: b, role: forager }),
                300 => s.push(Command::SetRolePriority { role: forager, work: haul, level: Some(1) }),
                400 => s.push(Command::CreateWorkRole { label: "Crew".into(), from: RoleSource::Pawn(a) }),
                _ => {}
            }
            if Some(t) == save_at {
                s = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
            }
            s.step();
        }
        s.world.state_hash()
    };
    let h = run(None);
    assert_eq!(h, run(None), "two runs agree");
    assert_eq!(h, run(Some(600)), "a load carries on the same");
}
