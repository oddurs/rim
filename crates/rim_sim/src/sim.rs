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
    /// Mods already warned about for going over their time budget.
    over_budget: Vec<String>,
}

impl Sim {
    pub fn new(mods_dir: &Path, seed: u64) -> Result<Sim, String> {
        Self::with_mods(mods_dir, seed, &|_| true)
    }

    /// Load only the mods `enabled` accepts, by id.
    pub fn with_mods(mods_dir: &Path, seed: u64, enabled: &dyn Fn(&str) -> bool) -> Result<Sim, String> {
        Self::build(mods_dir, seed, enabled, MAP_SIZE)
    }

    /// Everything configurable: which mods, and the map's size (square).
    pub fn build(mods_dir: &Path, seed: u64, enabled: &dyn Fn(&str) -> bool, size: i32) -> Result<Sim, String> {
        let loaded = modloader::load_only(mods_dir, enabled)?;
        let scripts = ScriptHost::load(&loaded.mods, &loaded.scripts, &loaded.defs)?;
        let warnings: Vec<String> = loaded.warnings.iter().chain(&scripts.warnings).cloned().collect();
        let defs = Arc::new(loaded.defs);
        let mut world = World::new(defs.clone(), size, size, seed);
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
            let d = defs
                .resolve("thing", &item.thing, crate::defs::home_of(&s.id))
                .map_err(|e| format!("start/{}: {e}", s.id))?;
            world.place_item(d, start, item.count);
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
            warnings,
            profile: Profile::default(),
            queue: Vec::new(),
            over_budget: Vec::new(),
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
        if w.tick.is_multiple_of(600) {
            self.check_mod_budgets();
        }
    }

    /// Warn (once per mod, in the load warnings the profiler shows) when a
    /// mod's hooks take longer than `MOD_BUDGET_US` per call on average.
    /// Wall-clock, so it only ever warns: it must never change the game. The
    /// hard, deterministic limit is the step budget.
    fn check_mod_budgets(&mut self) {
        for (name, us) in &self.profile.entries {
            let Some(m) = name.strip_prefix("mod:") else { continue };
            if *us > crate::script::MOD_BUDGET_US && !self.over_budget.iter().any(|w| w == m) {
                self.over_budget.push(m.to_string());
                self.warnings.push(format!(
                    "mod '{m}' is slow: its script calls take {:.2} ms on average (budget {:.2} ms)",
                    us / 1000.0,
                    crate::script::MOD_BUDGET_US / 1000.0
                ));
            }
        }
    }
}
