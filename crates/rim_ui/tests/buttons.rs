//! kit.button's roles and sizes, and the keyboard's focus ring.

mod common;
use common::*;
use rim_ui::paint::Draw;
use rim_ui::Input;

const PROBE: &str = r#"
local kit = require("@core/ui/kit")
ui.define("probe:buttons", function(view)
    return kit.col({ id = "probe:buttons", gap = "l", pad = "xl" }, {
        kit.button({ id = "probe:primary", label = "Start", kind = "primary", on_click = function() end }),
        kit.button({ id = "probe:secondary", label = "Options", on_click = function() end }),
        kit.button({ id = "probe:quiet", label = "Later", kind = "quiet", on_click = function() end }),
        kit.button({ id = "probe:danger", label = "Delete", kind = "danger", on_click = function() end }),
        kit.button({ id = "probe:small", label = "Options", size = "s" }),
        kit.button({ id = "probe:large", label = "Options", size = "l" }),
    })
end)
ui.mount("windows", "probe:buttons")
"#;

fn same(a: [f32; 4], b: [f32; 4]) -> bool {
    (0..4).all(|i| (a[i] - b[i]).abs() < 0.01)
}

/// The fill and the edge drawn exactly on `r`, if any.
fn look(draw: &[Draw], r: [f32; 4]) -> (Option<[f32; 4]>, Option<[f32; 4]>) {
    let fill = draw.iter().find_map(|d| match d {
        Draw::Rect { rect, color, .. } if same(*rect, r) => Some(*color),
        _ => None,
    });
    let edge = draw.iter().find_map(|d| match d {
        Draw::Outline { rect, color, .. } if same(*rect, r) => Some(*color),
        _ => None,
    });
    (fill, edge)
}

/// Outlines just outside `r` on every side, hugging it: the focus ring.
fn rings(draw: &[Draw], r: [f32; 4]) -> Vec<[f32; 4]> {
    let hugs = |o: [f32; 4]| {
        let e = r[0] - o[0];
        e > 0.0 && e < 8.0 && same([o[0] + e, o[1] + e, o[2] - 2.0 * e, o[3] - 2.0 * e], r)
    };
    draw.iter()
        .filter_map(|d| match d {
            Draw::Outline { rect, color, .. } if hugs(*rect) => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn each_role_draws_its_own_tokens_and_sizes_step_up() {
    let dir = scratch_mods("buttons", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let colors = ui.theme.color.clone();
    let c = |name: &str| colors[name];
    let at = |ui: &rim_ui::Ui, id: &str| ui.find(id).unwrap_or_else(|| panic!("{id} is drawn"));

    assert_eq!(look(&out.draw, at(&ui, "probe:primary")), (Some(c("accent")), Some(c("accent"))));
    assert_eq!(look(&out.draw, at(&ui, "probe:secondary")), (Some(c("surface_raised")), Some(c("line"))));
    assert_eq!(look(&out.draw, at(&ui, "probe:quiet")), (None, None), "quiet has no fill or edge at rest");
    assert_eq!(look(&out.draw, at(&ui, "probe:danger")), (Some(c("surface_raised")), Some(c("threat"))));

    // Under the pointer each takes its role's hover.
    for (id, hover) in [
        ("probe:primary", "accent_hover"),
        ("probe:secondary", "surface_hover"),
        ("probe:quiet", "surface_hover"),
        ("probe:danger", "threat_soft"),
    ] {
        let mouse = centre(at(&ui, id));
        let out = frame(&mut ui, &sim, &cv, Input { mouse, ..Default::default() });
        assert_eq!(look(&out.draw, at(&ui, id)).0, Some(c(hover)), "{id} under the pointer");
    }

    let h = |id: &str| at(&ui, id)[3];
    assert!(h("probe:small") < h("probe:secondary") && h("probe:secondary") < h("probe:large"), "s < m < l");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_kind_is_an_error_naming_the_kinds() {
    let bad = PROBE.replace("kind = \"danger\"", "kind = \"dangerous\"");
    let dir = scratch_mods("buttons-bad", &[("probe", "", &[("ui/probe.luau", &bad)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let w = ui.warnings().join("\n");
    assert!(w.contains("unknown kind 'dangerous'") && w.contains("primary, secondary, quiet or danger"), "{w}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_focus_ring_follows_the_keyboard_not_the_pointer() {
    let dir = scratch_mods("buttons-ring", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let primary = ui.find("probe:primary").unwrap();
    let secondary = ui.find("probe:secondary").unwrap();
    let focus = ui.theme.color["focus"];

    // A click focuses the button, but the pointer already says where it is.
    click(&mut ui, &sim, &mut cv, centre(primary));
    let away = (primary[0] + primary[2] + 200.0, primary[1]);
    let out = frame(&mut ui, &sim, &cv, Input { mouse: away, ..Default::default() });
    assert!(rings(&out.draw, primary).is_empty(), "no ring after a click");

    // Tab moves focus on, and now it's drawn: outside the button, in the
    // focus colour, and only around the one focused.
    let out = frame(&mut ui, &sim, &cv, Input { mouse: away, tab: true, ..Default::default() });
    let out = if rings(&out.draw, secondary).is_empty() {
        frame(&mut ui, &sim, &cv, Input { mouse: away, ..Default::default() })
    } else {
        out
    };
    assert_eq!(rings(&out.draw, secondary), [focus], "the tabbed-to button has a ring");
    assert!(rings(&out.draw, primary).is_empty(), "the one it left doesn't");

    // A press anywhere puts the ring away.
    frame(&mut ui, &sim, &cv, Input { mouse: away, left_pressed: true, ..Default::default() });
    let out = frame(&mut ui, &sim, &cv, Input { mouse: away, left_released: true, ..Default::default() });
    assert!(rings(&out.draw, secondary).is_empty(), "a press ends the ring");
    let _ = std::fs::remove_dir_all(&dir);
}
