//! Context menus: any node can be a subject, providers fill the menu in
//! group order, and the menu answers keys, clicks and a drag-release.

mod common;

use common::*;
use rim_ui::Input;

const PROBE: &str = r#"
local menus = require("@core/ui/menus")
local asked = 0
menus.kind("probe", {
    resolve = function(id) if not view.selected() then return nil end return { name = "Box " .. id } end,
    caption = function(ctx) return ctx.subject.name end,
    actor = function(ctx) return "tester" end,
})
menus.add("probe", { id = "probe:burn", label = "Burn", group = "damaging", run = function() ui.set_state("probe:did", "burn") end })
menus.add("probe", { id = "probe:look", label = "Look closer", group = "manage", run = function() ui.set_state("probe:did", "look") end })
menus.add("probe", {
    id = "probe:open", label = "Open",
    applies = function(ctx) asked += 1 return true end,
    run = function() ui.set_state("probe:did", "open") end,
})
menus.add("probe", { id = "probe:lock", label = "Unlock", group = "manage", disabled = function() return "no key" end, run = function() end })
ui.define("probe:panel", function(view)
    return ui.row({ id = "probe:subject", menu = { kind = "probe", id = 7 }, pad = "item", bg = "surface",
        ui.text({ "did=" .. tostring(ui.state("probe:did", "-")) .. " asked=" .. asked, id = "probe:did" }) })
end)
ui.mount("top", "probe:panel", { order = 90 })
"#;

struct T {
    ui: rim_ui::Ui,
    sim: rim_sim::Sim,
    cv: rim_ui::view::ClientView,
    t: f64,
    dir: std::path::PathBuf,
}

impl T {
    fn new(name: &str) -> T {
        let dir = scratch_mods(name, &[("probe", "", &[("ui/probe.luau", PROBE)])]);
        let sim = sim_at(&dir);
        let ui = ui_for(&sim);
        let cv = client(&sim);
        let mut t = T { ui, sim, cv, t: 0.0, dir };
        t.idle();
        assert!(t.ui.warnings().is_empty(), "{:?}", t.ui.warnings());
        t
    }
    fn input(&mut self, input: Input) -> rim_ui::Output {
        self.t += 0.05;
        self.cv.mouse = input.mouse;
        rim_ui::Ui::frame(&mut self.ui, &self.sim.world, &self.cv, &Input { time: self.t, ..input })
    }
    fn idle(&mut self) {
        let m = self.cv.mouse;
        for _ in 0..2 {
            self.input(Input { mouse: m, ..Default::default() });
        }
    }
    fn right_click(&mut self, at: (f32, f32)) {
        self.input(Input { mouse: at, ..Default::default() });
        self.input(Input { mouse: at, right_pressed: true, ..Default::default() });
        self.input(Input { mouse: at, right_released: true, ..Default::default() });
        self.idle();
    }
    fn key(&mut self, k: &str) -> rim_ui::Output {
        let m = self.cv.mouse;
        let out = self.input(Input { mouse: m, pressed: vec![k.into()], ..Default::default() });
        self.idle();
        out
    }
    fn snap(&self) -> String {
        self.ui.snapshot()
    }
    /// The menu's row labels, top to bottom.
    fn rows(&self) -> Vec<String> {
        let snap = self.snap();
        let mut out = Vec::new();
        let mut lines = snap.lines().peekable();
        while let Some(l) = lines.next() {
            if l.contains("#core:menu.row.") {
                // The row's texts follow it; the label is the first quoted one.
                for next in lines.by_ref() {
                    if let Some(q) = next.split('"').nth(1) {
                        out.push(q.to_string());
                        break;
                    }
                }
            }
        }
        out
    }
}

impl Drop for T {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_right_click_on_a_subject_opens_its_menu_in_group_order() {
    let mut t = T::new("menus-order");
    let subject = t.ui.find("probe:subject").expect("the subject");
    t.right_click(centre(subject));
    let menu = t.ui.find("core:menu").unwrap_or_else(|| panic!("the menu opens:\n{}", t.snap()));
    assert_eq!(t.rows(), vec!["Open", "Look closer", "Unlock", "Burn"], "do, manage, then damaging:\n{}", t.snap());
    let snap = t.snap();
    assert!(snap.contains("BOX 7") && snap.contains("tester"), "the caption names the subject and the actor:\n{snap}");
    assert!(snap.contains("no key"), "a disabled row says why:\n{snap}");
    // Beside the pointer, on screen.
    let (x, y) = centre(subject);
    assert!(menu[0] >= x && menu[1] >= y, "it opens down and right of the pointer: {menu:?}");
}

#[test]
fn it_flips_to_stay_on_screen() {
    let mut t = T::new("menus-flip");
    t.ui.context(&t.sim.world, &t.cv, "probe", "7", (1590.0, 950.0));
    t.idle();
    let menu = t.ui.find("core:menu").expect("the menu");
    assert!(menu[0] + menu[2] <= 1600.0 && menu[1] + menu[3] <= 960.0, "flipped into the screen: {menu:?}");
    assert!(menu[0] + menu[2] <= 1590.0, "to the left of the point");
}

#[test]
fn keys_pick_and_close_and_bindings_wait() {
    let mut t = T::new("menus-keys");
    let at = centre(t.ui.find("probe:subject").unwrap());
    t.right_click(at);
    // "2" is the second row that can run: Look closer.
    let out = t.key("2");
    assert!(t.ui.find("core:menu").is_none(), "a pick closes it");
    assert!(t.snap().contains("did=look"), "{}", t.snap());
    assert!(
        !out.actions.iter().any(|a| matches!(a, rim_ui::view::UiAction::Speed(_))),
        "2 wasn't a speed key while the menu was up"
    );
    // Down, then Enter.
    t.right_click(at);
    t.key("down");
    t.key("enter");
    assert!(t.snap().contains("did=look"), "down from Open lands on Look closer");
    // Escape closes, and the bindings are back after.
    t.right_click(at);
    t.key("escape");
    assert!(t.ui.find("core:menu").is_none(), "Escape closes it");
    let out = t.key("2");
    assert!(
        out.actions.iter().any(|a| matches!(a, rim_ui::view::UiAction::Speed(_))),
        "2 is the speed key again: {:?}",
        out.actions
    );
}

#[test]
fn a_click_outside_closes_and_a_drag_release_picks() {
    let mut t = T::new("menus-mouse");
    let at = centre(t.ui.find("probe:subject").unwrap());
    t.right_click(at);
    t.input(Input { mouse: (800.0, 500.0), left_pressed: true, ..Default::default() });
    t.idle();
    assert!(t.ui.find("core:menu").is_none(), "a click outside closes it");
    // Press on the subject, drag onto the first row, let go.
    t.input(Input { mouse: at, right_pressed: true, ..Default::default() });
    t.idle();
    let row = t.ui.find("core:menu.row.1").expect("the first row");
    t.input(Input { mouse: centre(row), ..Default::default() });
    t.input(Input { mouse: centre(row), right_released: true, ..Default::default() });
    t.idle();
    assert!(t.snap().contains("did=open"), "the release picked Open:\n{}", t.snap());
}

#[test]
fn providers_run_once_per_opening_and_a_gone_subject_closes_it() {
    let mut t = T::new("menus-once");
    let at = centre(t.ui.find("probe:subject").unwrap());
    t.right_click(at);
    for _ in 0..20 {
        t.input(Input { mouse: (at.0 + 30.0, at.1 + 40.0), ..Default::default() });
    }
    assert!(t.snap().contains("asked=1"), "once, not per frame:\n{}", t.snap());
    // The subject goes away while the menu is up (the probe's boxes exist
    // only while something is selected).
    t.cv.selected = None;
    t.idle();
    assert!(t.ui.find("core:menu").is_none(), "a menu about nothing closes");
}
