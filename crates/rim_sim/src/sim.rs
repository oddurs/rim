//! `Sim` ties it together: load mods, generate the world, run ticks.

use crate::command::{self, Command};
use crate::modloader::{self, ModManifest};
use crate::profile::Profile;
use crate::script::ScriptHost;
use crate::world::*;
use crate::{ai, mapgen, systems, TICKS_PER_DAY};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

pub const MAP_SIZE: i32 = 200;

pub struct Sim {
    pub world: World,
    pub scripts: ScriptHost,
    pub mods: Vec<ModManifest>,
    pub warnings: Vec<String>,
    pub profile: Profile,
    queue: Vec<Command>,
}

impl Sim {
    pub fn new(mods_dir: &Path, seed: u64) -> Result<Sim, String> {
        Self::with_mods(mods_dir, seed, &|_| true)
    }

    /// Load only the mods `enabled` accepts, by id.
    pub fn with_mods(mods_dir: &Path, seed: u64, enabled: &dyn Fn(&str) -> bool) -> Result<Sim, String> {
        let loaded = modloader::load_only(mods_dir, enabled)?;
        let scripts = ScriptHost::load(&loaded.scripts, &loaded.defs)?;
        let defs = Arc::new(loaded.defs);
        let mut world = World::new(defs.clone(), MAP_SIZE, MAP_SIZE, seed);
        let start = mapgen::generate(&mut world);

        let s = defs.start.as_ref().unwrap();
        for i in 0..s.count {
            let p = start.offset(i as i32, 0);
            let e =
                world.spawn_pawn(s.creature_r, Faction::Player, if world.map.passable(p) { p } else { start }, None);
            if i == 0 {
                if let Ok(mut pawn) = world.ecs.get::<&mut Pawn>(e) {
                    pawn.founder = true;
                }
            }
        }
        for item in &s.items {
            if let Some(d) = defs.thing_id(&item.thing) {
                world.place_item(d, start, item.count);
            }
        }
        let founder = world.colonists().next().and_then(|e| world.ecs.get::<&Pawn>(e).ok().map(|p| p.name.clone()));
        if let Some(name) = founder {
            let title = if s.title.is_empty() { String::new() } else { format!(", {}", s.title) };
            world.message(
                format!("{name}{title}, wakes with nothing. Build a shelter before nightfall."),
                MsgKind::Info,
            );
        }
        systems::wealth(&mut world);
        let clock = world.clock();
        world.fields.update_ambient(&defs, clock);

        Ok(Sim {
            world,
            scripts,
            mods: loaded.mods,
            warnings: loaded.warnings,
            profile: Profile::default(),
            queue: Vec::new(),
        })
    }

    /// Queue a player command; it applies at the start of the next tick.
    pub fn push(&mut self, c: Command) {
        self.queue.push(c);
    }

    pub fn step(&mut self) {
        let t0 = Instant::now();
        let w = &mut self.world;
        let prof = &mut self.profile;

        for c in self.queue.drain(..) {
            command::apply(w, c);
        }
        prof.time("regions", || w.map.ensure_regions());
        prof.time("rooms", || w.map.ensure_rooms());
        prof.time("boundary", || w.refresh_boundaries());
        let defs = w.defs.clone();
        let clock = w.clock();
        prof.time("fields", || w.fields.update(&defs, &mut w.map, clock));
        prof.time("pawns", || ai::tick_pawns(w));
        prof.time("deaths", || systems::deaths(w));
        if w.tick.is_multiple_of(systems::NEEDS_INTERVAL) {
            prof.time("needs", || systems::needs(w));
        }
        if w.tick.is_multiple_of(250) {
            prof.time("regrow", || systems::regrow(w));
            prof.time("wealth", || systems::wealth(w));
        }
        if w.tick % 500 == 250 {
            prof.time("plants", || systems::spread_plants(w));
        }
        if w.tick > 0 && w.tick.is_multiple_of(TICKS_PER_DAY) {
            w.events.push(GameEvent::NewDay { day: w.day() });
            let yesterday = w.season_index_at(w.tick - 1);
            if w.season_index() != yesterday {
                let season = w.season().to_string();
                w.events.push(GameEvent::SeasonChanged { season, index: w.season_index(), year: w.year() });
            }
        }
        self.scripts.run_hooks(w, prof);
        self.scripts.dispatch_events(w, prof);
        w.tick += 1;
        prof.add("tick", t0.elapsed().as_secs_f64() * 1e6);
    }
}
