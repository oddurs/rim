//! The iron plugin (DESIGN.md §4f): bog iron to bars at a bloomery, bars to
//! nails and a saw at a forge, and nails in timber's builds.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::{Command, IVec, Sim};

fn open_near(sim: &Sim, from: IVec, w: i32) -> IVec {
    (3..40)
        .flat_map(|r| [from.offset(r, 0), from.offset(-r, 0), from.offset(0, r), from.offset(0, -r)])
        .find(|&o| {
            (-1..=w).all(|dx| {
                (-1..=1).all(|dy| {
                    let p = o.offset(dx, dy);
                    sim.world.map.passable(p)
                        && sim.world.map.item_at(p).is_none()
                        && sim.world.map.fixture_at(p).is_none()
                })
            })
        })
        .expect("open ground")
}

fn event(name: &str, pairs: Vec<(&str, Data)>) -> Command {
    Command::ModEvent {
        name: name.into(),
        data: Some(Data::Table(pairs.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect())),
    }
}

/// A bill at `site` for `recipe`, made until there are `n`.
fn bill_until(sim: &mut Sim, site: Entity, recipe: &str, n: i64) {
    let s = Data::Int(site.to_bits().get() as i64);
    sim.push(event("crafting:add_bill", vec![("site", s.clone()), ("recipe", Data::Str(recipe.into()))]));
    sim.step();
    let Some(Data::Table(state)) = sim.world.data.get("crafting:bills") else { panic!("no bills") };
    let bills = state.values().find(|st| st.get("site") == Some(&s)).and_then(|st| st.get("bills")).cloned();
    let Some(Data::Table(bills)) = bills else { panic!("no bills at the site") };
    let id = bills.values().last().and_then(|b| b.get("id")).cloned().expect("the new bill");
    let until = vec![("site", s), ("bill", id), ("mode", Data::Str("until".into())), ("target", Data::Int(n))];
    sim.push(event("crafting:set_bill", until));
    sim.step();
}

#[test]
fn with_iron_a_crate_needs_nails_and_a_shelf_its_fittings() {
    let sim = Sim::new(&common::mods(), 1).unwrap();
    let defs = &sim.world.defs;
    let cost = |id: &str| -> Vec<DefId> {
        defs.thing(defs.thing_id(id).unwrap()).build.as_ref().unwrap().cost_r.iter().map(|c| c.0).collect()
    };
    let (nails, fittings) = (defs.thing_id("iron:nails").unwrap(), defs.thing_id("iron:fittings").unwrap());
    assert_eq!(cost("timber:crate"), [nails]);
    assert_eq!(cost("timber:plank_wall"), [nails]);
    assert_eq!(cost("timber:shelf"), [fittings]);
    let bed = defs.thing(defs.thing_id("iron:bog_iron_bed").unwrap());
    assert!(bed.harvest.iter().any(|h| h.requires.iter().any(|r| r == "digging")), "dug like clay");
}

#[test]
fn bog_iron_becomes_bars_then_nails_and_a_saw() {
    let mut sim = Sim::new(&common::mods(), 4).unwrap();
    let defs = sim.world.defs.clone();
    let id = |s: &str| defs.thing_id(s).unwrap_or_else(|| panic!("no {s}"));
    let pawn = sim.world.colonists().next().unwrap();
    let home = sim.world.pawn_pos(pawn).unwrap();
    let clamp_at = open_near(&sim, home, 1);
    let clamp = sim.world.spawn_fixture_of(id("iron:charcoal_clamp"), clamp_at, false, None).unwrap();
    let bloom_at = open_near(&sim, clamp_at.offset(0, 4), 1);
    let bloomery = sim.world.spawn_fixture_of(id("iron:bloomery"), bloom_at, false, None).unwrap();
    let forge_at = open_near(&sim, bloom_at.offset(0, 4), 2);
    let forge = sim.world.spawn_fixture_of(id("iron:forge"), forge_at, false, None).unwrap();
    sim.world.place_item(id("iron:bog_iron"), home.offset(1, 0), 12);
    sim.world.place_item(id("core:wood"), home.offset(0, 1), 30);
    sim.world.place_item(id("primitive:hammerstone"), home.offset(-1, 0), 1);
    bill_until(&mut sim, clamp, "iron:charcoal", 8);
    bill_until(&mut sim, bloomery, "iron:smelt", 4);
    bill_until(&mut sim, forge, "iron:nails", 20);
    bill_until(&mut sim, forge, "iron:saw", 1);
    let (nails, saw) = (id("iron:nails"), id("iron:saw"));
    let mut made = false;
    for _ in 0..(6 * rim_sim::TICKS_PER_DAY) {
        sim.step();
        let has = |d| sim.world.ecs.query::<&rim_sim::world::Thing>().iter().any(|t| t.def == d);
        if has(nails) && has(saw) {
            made = true;
            break;
        }
    }
    assert!(made, "nails and a saw from bog iron: {:?}", sim.world.messages.iter().rev().take(3).collect::<Vec<_>>());
}
