//! The UI scripting API is declared once (`crates/rim_ui/src/api.rs`). This
//! checks the declarations cover `ui`, `act` and `view` exactly, and that the
//! checked-in type definitions and reference match them.
//! `RIM_UPDATE_TYPES=1` rewrites the files.

mod common;
use common::*;
use rim_ui::api;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// The Luau compiler parses type annotations: compiling each declared
/// global as a typed local proves every signature is valid Luau type syntax.
fn assert_valid_luau_types(defs: &str) {
    let mut chunk = String::new();
    let mut in_declare = false;
    for line in defs.lines() {
        if let Some(rest) = line.strip_prefix("declare ") {
            in_declare = true;
            chunk.push_str(&format!("local _{rest}\n"));
        } else if in_declare && line == "}" {
            in_declare = false;
            chunk.push_str("} = nil\n");
        } else {
            chunk.push_str(line);
            chunk.push('\n');
        }
    }
    mlua::Lua::new()
        .load(&chunk)
        .set_name("ui.d.luau")
        .into_function()
        .unwrap_or_else(|e| panic!("types/ui.d.luau isn't valid Luau: {e}"));
}

#[test]
fn every_ui_member_is_declared_and_the_generated_files_are_current() {
    let sim = sim_at(&mods());
    let ui = ui_for(&sim);
    let declared: Vec<String> = api::UI_API.iter().map(|d| d.name.to_string()).collect();
    let mut sorted = declared.clone();
    sorted.sort();
    assert_eq!(declared, sorted, "keep UI_API sorted by name");
    let registered = ui.vm.api_names();
    let missing: Vec<&String> = registered.iter().filter(|n| !declared.contains(n)).collect();
    let stale: Vec<&String> = declared.iter().filter(|n| !registered.contains(n)).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "declare these in crates/rim_ui/src/api.rs: {missing:?}; these are declared but not registered: {stale:?}"
    );
    assert_valid_luau_types(&api::luau_definitions());

    for (file, text) in [("types/ui.d.luau", api::luau_definitions()), ("docs/modding/api-ui.md", api::api_reference())]
    {
        let path = root().join(file);
        if std::env::var_os("RIM_UPDATE_TYPES").is_some() {
            std::fs::write(&path, &text).unwrap();
            continue;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
        assert!(
            on_disk == text,
            "{file} is out of date with crates/rim_ui/src/api.rs: run \
             RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types"
        );
    }
}
