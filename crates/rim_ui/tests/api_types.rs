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

/// The fields a Luau type declares: `name:` at the start of each entry of
/// `type <name> = { ... }` in the UI types.
fn type_fields(name: &str) -> Vec<String> {
    let start = api::UI_TYPES.find(&format!("type {name} =")).unwrap_or_else(|| panic!("no type {name}"));
    let body = &api::UI_TYPES[start..];
    let open = body.find('{').unwrap();
    // The declaration's own braces, not the nested ones in its fields.
    let (mut depth, mut end) = (0, 0);
    for (i, c) in body[open..].char_indices() {
        depth += i32::from(c == '{') - i32::from(c == '}');
        if depth == 0 {
            end = open + i;
            break;
        }
    }
    let mut out = Vec::new();
    let mut depth = 0;
    let mut word = String::new();
    for c in body[open + 1..end].chars() {
        match c {
            '{' | '(' => depth += 1,
            '}' | ')' => depth -= 1,
            ':' if depth == 0 && !word.is_empty() => out.push(std::mem::take(&mut word)),
            c if depth == 0 && (c.is_ascii_alphanumeric() || c == '_') => word.push(c),
            _ if depth == 0 => word.clear(),
            _ => {}
        }
    }
    out
}

/// Every property a node table can carry, as `node_from_table` matches
/// them (crates/rim_ui/src/node.rs), is declared on Node, or on GridProps
/// or ListProps for the kinds only they take: an editor's strict check of
/// core's own scripts mustn't flag what the engine accepts.
#[test]
fn every_node_property_the_engine_takes_is_typed() {
    let src = include_str!("../src/node.rs");
    let body = src.split("pub fn node_from_table").nth(1).expect("node_from_table");
    let body = body.split("\n}\n").next().unwrap();
    let mut props: Vec<String> = Vec::new();
    for line in body.lines() {
        // The match on the table's keys, one level in from the loop.
        let Some(arm) = line.strip_prefix("            \"") else { continue };
        let Some((pat, _)) = arm.split_once("=>") else { continue };
        let pat = pat.split(" if ").next().unwrap();
        props.extend(pat.split('|').map(|p| p.trim().trim_matches('"').to_string()));
    }
    assert!(props.len() > 50, "the property match wasn't found: {props:?}");
    let mut typed = type_fields("Node");
    typed.extend(type_fields("GridProps"));
    typed.extend(type_fields("ListProps"));
    let missing: Vec<&String> = props.iter().filter(|p| !typed.contains(p)).collect();
    assert!(missing.is_empty(), "declare these in UI_TYPES (crates/rim_ui/src/api.rs): {missing:?}");

    // A grid cell's fields, as the grid reads them.
    let cells: Vec<String> =
        src.split("ct.get::<Value>(\"").skip(1).filter_map(|s| s.split('"').next()).map(str::to_string).collect();
    assert!(cells.len() > 5, "the cell fields weren't found");
    let cell = type_fields("Cell");
    let missing: Vec<&String> = cells.iter().filter(|c| !cell.contains(c)).collect();
    assert!(missing.is_empty(), "declare these on Cell: {missing:?}");
}
