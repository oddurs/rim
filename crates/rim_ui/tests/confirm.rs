//! kit.confirm: the safe answer is what Esc, Enter and a stray click give.

mod common;
use common::*;
use rim_ui::view::UiAction;
use rim_ui::Input;

const PROBE: &str = r#"
local kit = require("@core/ui/kit")
local function said(word)
    ui.set_state("probe:log", ui.state("probe:log", "") .. word .. " ")
end
ui.define("probe:asker", function(view)
    return kit.col({ id = "probe:asker", gap = "s" }, {
        kit.button({ id = "probe:ask", label = "Burn the stores", on_click = function()
            kit.confirm({ title = "Burn the stores?", body = "Everything in them is lost.", verb = "Burn", danger = true,
                on_confirm = function() said("yes") end, on_cancel = function() said("no") end })
        end }),
        kit.label(ui.state("probe:log", ""), { id = "probe:log" }),
    })
end)
ui.mount("windows", "probe:asker")
"#;

fn keys(names: &[&str]) -> Input {
    Input { pressed: names.iter().map(|s| s.to_string()).collect(), ..Default::default() }
}

#[test]
fn escape_enter_and_a_press_elsewhere_answer_safely() {
    let dir = scratch_mods("confirm", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let ask = |ui: &mut rim_ui::Ui, cv: &mut rim_ui::view::ClientView| {
        let at = centre(ui.find("probe:ask").unwrap());
        click(ui, &sim, cv, at);
        frame(ui, &sim, cv, Default::default());
        frame(ui, &sim, cv, Default::default());
        assert!(ui.find("core:confirm.ok").is_some(), "the question is up: {}", ui.snapshot());
    };
    let log = |ui: &rim_ui::Ui| {
        let snap = ui.snapshot();
        snap.lines().find(|l| l.contains("#probe:log")).unwrap_or_default().to_string()
    };

    // Esc: no, and the question goes.
    ask(&mut ui, &mut cv);
    assert!(ui.snapshot().contains("Burn the stores?") && ui.snapshot().contains("Everything in them is lost."));
    frame(&mut ui, &sim, &cv, keys(&["escape"]));
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.find("core:confirm.ok").is_none(), "Esc closes it");
    assert!(log(&ui).contains("\"no \""), "{}", log(&ui));

    // Enter presses what has focus: the safe answer.
    ask(&mut ui, &mut cv);
    frame(&mut ui, &sim, &cv, Input { enter: true, ..keys(&["enter"]) });
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.find("core:confirm.ok").is_none(), "Enter answers");
    assert!(log(&ui).contains("\"no no \""), "{}", log(&ui));

    // While it's up, no binding fires: Space doesn't pause.
    ask(&mut ui, &mut cv);
    let out = frame(&mut ui, &sim, &cv, keys(&["space"]));
    assert!(!out.actions.contains(&UiAction::TogglePause), "{:?}", out.actions);

    // Tab to the verb, then Enter: yes.
    frame(&mut ui, &sim, &cv, Input { tab: true, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { enter: true, ..keys(&["enter"]) });
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(log(&ui).contains("\"no no yes \""), "{}", log(&ui));

    // A press anywhere else: no.
    ask(&mut ui, &mut cv);
    click(&mut ui, &sim, &mut cv, (5.0, 5.0));
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.find("core:confirm.ok").is_none(), "a press elsewhere closes it");
    assert!(log(&ui).contains("\"no no yes no \""), "{}", log(&ui));
    let _ = std::fs::remove_dir_all(&dir);
}
