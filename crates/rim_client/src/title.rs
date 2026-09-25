//! The title screen: continue, load a colony, or start a new one. It's the
//! UI's `title` layer, drawn by a mod (core's is `mods/core/ui/title.luau`)
//! over an empty world; this lists the saves and opens the one picked.

use crate::{draw, save, ui_input, upload_atlas, RawInput};
use macroquad::prelude::*;
use rim_sim::defs::DefDb;
use rim_sim::savefile::Writer;
use rim_sim::world::World;
use rim_sim::Sim;
use rim_ui::view::{ClientView, SaveView, UiAction};
use rim_ui::Ui;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Show the title screen until the player picks a game, then open it. A
/// save that fails to load says why beside it, and the screen stays up.
pub async fn run(
    ui: &mut Ui,
    atlas: &Texture2D,
    wheel_sub: usize,
    mods: &Path,
    defs: DefDb,
    seed: u64,
) -> Result<(Sim, Option<Writer>, Vec<String>), String> {
    // What the UI's world views read before there is a game.
    let world = World::new(Arc::new(defs), 1, 1, seed);
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut saves = with_failures(save::list(), &failed);
    loop {
        let dpi = screen_dpi_scale();
        ui.set_dpi(dpi);
        let raw = RawInput::gather_ui(wheel_sub);
        ui.check_reload(raw.time);
        let client = ClientView {
            screen: (screen_width() * dpi, screen_height() * dpi),
            scale: ui.theme.scale,
            mouse: (raw.mouse.0 * dpi, raw.mouse.1 * dpi),
            time: raw.time,
            warnings: ui.warnings(),
            title: true,
            saves: saves.clone(),
            ..ClientView::default()
        };
        let out = ui.frame(&world, &client, &ui_input(&raw, dpi));
        for action in out.actions {
            let (start, path) = match action {
                UiAction::Load(path) => (save::Start::Load(PathBuf::from(&path)), Some(path)),
                UiAction::NewColony => (save::Start::New, None),
                _ => continue,
            };
            match (save::open(mods, seed, start), path) {
                (Ok(game), _) => return Ok(game),
                (Err(e), Some(path)) => {
                    failed.push((path, e));
                    saves = with_failures(save::list(), &failed);
                }
                // A new colony that can't start is the mods' fault: nothing
                // on this screen would change it.
                (Err(e), None) => return Err(e),
            }
        }
        clear_background(Color::from_rgba(18, 20, 23, 255));
        upload_atlas(ui, atlas);
        draw::ui(&out.draw, atlas, ui.text.atlas.white_texel(), dpi);
        next_frame().await;
    }
}

/// The saves, each with why it last failed to load, if it did.
fn with_failures(mut saves: Vec<SaveView>, failed: &[(String, String)]) -> Vec<SaveView> {
    for s in &mut saves {
        if let Some((_, why)) = failed.iter().rev().find(|(p, _)| *p == s.path) {
            s.error = Some(why.clone());
        }
    }
    saves
}
