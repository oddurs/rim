//! The script API is declared once, where each function is registered
//! (`crates/rim_sim/src/script.rs`). This checks the declarations cover the
//! `rim` table exactly, and that the checked-in type definitions and
//! reference match them, and that `.luaurc` has an alias for every shipped
//! mod, so editors resolve `require` as the game does. `RIM_UPDATE_TYPES=1`
//! rewrites the files.

use rim_sim::Sim;

/// The Luau compiler parses type annotations: compiling the declarations as
/// a typed local proves every signature is valid Luau type syntax.
fn assert_valid_luau_types(defs: &str) {
    let chunk = defs.replacen("declare rim: {", "local _rim: {", 1) + " = nil\n";
    mlua::Lua::new()
        .load(&chunk)
        .set_name("rim.d.luau")
        .into_function()
        .unwrap_or_else(|e| panic!("types/rim.d.luau isn't valid Luau: {e}"));
}
use std::path::Path;

/// `.luaurc`: `@<mod>` is `mods/<mod>`, for every mod in the repo.
fn luaurc() -> String {
    let mut mods: Vec<String> = std::fs::read_dir(root().join("mods"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().join("mod.toml").is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    mods.sort();
    let aliases: Vec<String> = mods.iter().map(|m| format!("    \"{m}\": \"./mods/{m}\"")).collect();
    format!("{{\n  \"languageMode\": \"nonstrict\",\n  \"aliases\": {{\n{}\n  }}\n}}\n", aliases.join(",\n"))
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn every_rim_member_is_declared_and_the_generated_files_are_current() {
    let s = Sim::with_mods(&root().join("mods"), 1, &|m| m == "core").expect("core loads");
    let declared: Vec<String> = s.scripts.api().iter().map(|d| d.name.to_string()).collect();
    assert_eq!(declared, s.scripts.engine_names(), "declarations and the rim table disagree");
    assert_valid_luau_types(&s.scripts.luau_definitions());

    for (file, text) in [
        ("types/rim.d.luau", s.scripts.luau_definitions()),
        ("docs/modding/api-scripts.md", s.scripts.api_reference()),
        (".luaurc", luaurc()),
    ] {
        let path = root().join(file);
        if std::env::var_os("RIM_UPDATE_TYPES").is_some() {
            std::fs::write(&path, &text).unwrap();
            continue;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
        assert!(
            on_disk == text,
            "{file} is out of date with the API declared in script.rs: run \
             RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types"
        );
    }
}
