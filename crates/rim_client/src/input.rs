//! Raw input: what a frame gathers from macroquad (or the autotest
//! builds by hand), and the camera's and right button's answers to it.

use super::*;

/// Seconds since the last frame, clamped: macroquad's value is raw, so the
/// first frame (load time) or a window drag would otherwise jump the camera
/// and run a burst of sim ticks.
pub(crate) fn frame_time() -> f32 {
    get_frame_time().min(0.1)
}

/// Every key a binding can name, with its name. Letters and digits are
/// themselves; the rest are spelled out.
const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::A, "a"),
    (KeyCode::B, "b"),
    (KeyCode::C, "c"),
    (KeyCode::D, "d"),
    (KeyCode::E, "e"),
    (KeyCode::F, "f"),
    (KeyCode::G, "g"),
    (KeyCode::H, "h"),
    (KeyCode::I, "i"),
    (KeyCode::J, "j"),
    (KeyCode::K, "k"),
    (KeyCode::L, "l"),
    (KeyCode::M, "m"),
    (KeyCode::N, "n"),
    (KeyCode::O, "o"),
    (KeyCode::P, "p"),
    (KeyCode::Q, "q"),
    (KeyCode::R, "r"),
    (KeyCode::S, "s"),
    (KeyCode::T, "t"),
    (KeyCode::U, "u"),
    (KeyCode::V, "v"),
    (KeyCode::W, "w"),
    (KeyCode::X, "x"),
    (KeyCode::Y, "y"),
    (KeyCode::Z, "z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::F1, "f1"),
    (KeyCode::F2, "f2"),
    (KeyCode::F3, "f3"),
    (KeyCode::F4, "f4"),
    (KeyCode::F5, "f5"),
    (KeyCode::F6, "f6"),
    (KeyCode::F7, "f7"),
    (KeyCode::F8, "f8"),
    (KeyCode::F9, "f9"),
    (KeyCode::F10, "f10"),
    (KeyCode::F11, "f11"),
    (KeyCode::F12, "f12"),
    (KeyCode::Space, "space"),
    (KeyCode::Escape, "escape"),
    (KeyCode::Tab, "tab"),
    (KeyCode::Enter, "enter"),
    (KeyCode::Backspace, "backspace"),
    (KeyCode::Delete, "delete"),
    (KeyCode::Left, "left"),
    (KeyCode::Right, "right"),
    (KeyCode::Up, "up"),
    (KeyCode::Down, "down"),
    (KeyCode::Home, "home"),
    (KeyCode::End, "end"),
    (KeyCode::PageUp, "pageup"),
    (KeyCode::PageDown, "pagedown"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::GraveAccent, "`"),
];

/// A character from the text path that a text input should keep. Control
/// characters are the keys the input already handles; the private-use
/// range is how macOS spells its arrow, home, end and function keys in
/// the same stream. macOS also sends Cmd+K as a plain 'k', so nothing is
/// typed while Cmd or Ctrl is held, unless Alt is too: Windows reports
/// AltGr, which types '@' and '€' on many layouts, as Ctrl+Alt.
fn typed_char(c: char, command: bool, alt: bool) -> Option<char> {
    let private = ('\u{e000}'..='\u{f8ff}').contains(&c);
    (!c.is_control() && !private && (!command || alt)).then_some(c)
}

/// The name a binding uses for a key, if it has one.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// One frame's raw input, in logical points. The real loop gathers it from
/// macroquad; `--autotest` builds it by hand, so both drive the same path.
/// The raw events a frame needs that macroquad's polled state loses.
#[derive(Default)]
pub(crate) struct Events {
    /// Every scroll event, as the backend reported them. `mouse_wheel()`
    /// keeps only the last one, and a trackpad sends several per frame.
    wheel: Vec<(f32, f32)>,
    /// A held key's repeats. `is_key_pressed` sees only the first press, so
    /// a held Backspace would take one character while a held letter
    /// types on.
    repeats: Vec<KeyCode>,
}

impl macroquad::miniquad::EventHandler for Events {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        self.wheel.push((x, y));
    }
    fn key_down_event(&mut self, code: KeyCode, _: macroquad::miniquad::KeyMods, repeat: bool) {
        if repeat && repeats(code) {
            self.repeats.push(code);
        }
    }
}

/// Does holding this key act again: the UI's editing and list keys do,
/// Escape doesn't (a held Escape would close window after window), and
/// bindings never see repeats at all.
fn repeats(code: KeyCode) -> bool {
    code != KeyCode::Escape && UI_KEYS.iter().any(|&(c, _)| c == code)
}

/// One wheel notch in the backend's units. miniquad passes macOS's
/// precise (trackpad) deltas through in points and multiplies a wheel's
/// coarse line steps by 10; Windows reports 120 a notch; X11 reports 1.
const NOTCH: f32 = if cfg!(target_os = "macos") {
    10.0
} else if cfg!(target_os = "windows") {
    120.0
} else {
    1.0
};

/// Points of travel per backend unit, for precise scrolling.
const POINTS_PER_UNIT: f32 = if cfg!(target_os = "windows") { 1.0 / 3.0 } else { 1.0 };

/// How long after a precise scroll a round-numbered one still counts as
/// the trackpad's: its deltas can land on a whole notch by chance.
const PRECISE_STICKS: f64 = 0.3;

/// One frame of scrolling, sorted by what made it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scroll {
    /// Whole steps from a wheel: they zoom.
    pub notches: f32,
    /// Travel in points from a trackpad (or a smooth wheel), both axes:
    /// it pans, one to one with the fingers.
    pub travel: (f32, f32),
}

/// Sort a frame's scroll events: a wheel moves in whole notches along one
/// axis, a trackpad in fractions and often sideways, `points` points to a
/// unit. With shift held, a sideways whole notch is still the wheel (macOS
/// turns shift-wheel into a sideways scroll).
pub fn classify(events: &[(f32, f32)], notch: f32, points: f32, shift: bool) -> Scroll {
    let whole = |v: f32| v != 0.0 && ((v / notch) - (v / notch).round()).abs() < 1e-3;
    let mut s = Scroll::default();
    for &(x, y) in events {
        if x == 0.0 && whole(y) {
            s.notches += y / notch;
        } else if shift && y == 0.0 && whole(x) {
            s.notches += x / notch;
        } else {
            s.travel.0 += x * points;
            s.travel.1 += y * points;
        }
    }
    s
}

impl Events {
    /// This frame's scrolling, sorted for the camera and summed in notches
    /// for the interface's scroll areas (fractional on a trackpad), and its
    /// key repeats.
    fn gather(sub: usize) -> (Scroll, f32, Vec<KeyCode>) {
        let mut e = Events::default();
        macroquad::input::utils::repeat_all_miniquad_input(&mut e, sub);
        let shift =
            macroquad::input::is_key_down(KeyCode::LeftShift) || macroquad::input::is_key_down(KeyCode::RightShift);
        let scroll = classify(&e.wheel, NOTCH, POINTS_PER_UNIT, shift);
        let mut y: f32 = e.wheel.iter().map(|e| e.1).sum();
        if shift && y == 0.0 {
            y = e.wheel.iter().map(|e| e.0).sum();
        }
        (scroll, (y / NOTCH).clamp(-4.0, 4.0), e.repeats)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawInput {
    pub mouse: (f32, f32),
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub right_released: bool,
    /// The right button is held.
    pub right_down: bool,
    /// Wheel movement in notches (fractional on a trackpad), for the
    /// interface's scroll areas.
    pub wheel: f32,
    /// The same scrolling, sorted for the camera.
    pub scroll: Scroll,
    /// Cmd or Ctrl is held: any scroll zooms.
    pub zoom_mod: bool,
    /// A trackpad pinch this frame: how far the fingers spread (0.1 is 10%
    /// apart; negative pinches in). macOS only.
    pub pinch: f32,
    pub keys: Vec<KeyCode>,
    /// Characters typed this frame, for a focused text input.
    pub chars: Vec<char>,
    /// Every key pressed this frame by name, with modifiers ("ctrl+k").
    pub pressed: Vec<String>,
    pub shift: bool,
    pub alt: bool,
    /// Camera pan this frame, in tiles (WASD, middle-drag).
    pub pan: (f32, f32),
    pub time: f64,
    /// Advance the simulation by real frame time (the autotest steps it
    /// explicitly instead).
    pub advance: bool,
}

impl RawInput {
    /// This frame's input, with the camera's pan.
    pub(crate) fn gather(app: &mut App) -> RawInput {
        let mut raw = RawInput::gather_ui(app.wheel_sub);
        let (mx, my) = raw.mouse;
        let speed = 18.0 * frame_time() * 40.0 / app.cam.zoom;
        let (mut dx, mut dy) = (0.0, 0.0);
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            dy -= speed;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            dy += speed;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            dx -= speed;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            dx += speed;
        }
        // Middle-drag pans: the grabbed world point follows the mouse.
        if is_mouse_button_pressed(MouseButton::Middle) {
            app.pan_anchor = Some((mx, my));
        }
        if let Some((ax, ay)) = app.pan_anchor {
            if is_mouse_button_down(MouseButton::Middle) {
                dx -= (mx - ax) / app.cam.zoom;
                dy -= (my - ay) / app.cam.zoom;
                app.pan_anchor = Some((mx, my));
            } else {
                app.pan_anchor = None;
            }
        }
        raw.pan = (dx, dy);
        raw
    }

    /// The keys gathered as keys, not text: every key the UI maps
    /// (`UI_KEYS`), and Tab and Enter, which it reads as flags.
    fn gathered_keys() -> impl Iterator<Item = KeyCode> {
        UI_KEYS.iter().map(|&(code, _)| code).chain([KeyCode::Tab, KeyCode::Enter])
    }

    /// The mouse, keys and text: everything but the camera.
    pub(crate) fn gather_ui(wheel_sub: usize) -> RawInput {
        let (mx, my) = mouse_position();
        let (scroll, wheel, repeats) = Events::gather(wheel_sub);
        let mut keys: Vec<KeyCode> = Self::gathered_keys().filter(|k| is_key_pressed(*k)).collect();
        keys.extend(repeats);
        let ctrl = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);
        let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        let mut chars = Vec::new();
        while let Some(c) = get_char_pressed() {
            if let Some(c) = typed_char(c, ctrl, alt) {
                chars.push(c);
            }
        }
        let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let mut pressed = Vec::new();
        for (code, name) in KEY_NAMES {
            if is_key_pressed(*code) {
                let mut s = String::new();
                for (on, m) in [(ctrl, "ctrl+"), (alt, "alt+"), (shift_down, "shift+")] {
                    if on {
                        s.push_str(m);
                    }
                }
                s.push_str(name);
                pressed.push(s);
            }
        }

        RawInput {
            mouse: (mx, my),
            left_pressed: is_mouse_button_pressed(MouseButton::Left),
            left_released: is_mouse_button_released(MouseButton::Left),
            right_pressed: is_mouse_button_pressed(MouseButton::Right),
            right_released: is_mouse_button_released(MouseButton::Right),
            right_down: is_mouse_button_down(MouseButton::Right),
            wheel,
            scroll,
            zoom_mod: ctrl,
            pinch: pinch::take(),
            keys,
            chars,
            pressed,
            shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
            alt,
            pan: (0.0, 0.0),
            time: get_time(),
            advance: true,
        }
    }
}

/// Raw input as the UI takes it, in physical pixels.
pub(crate) fn ui_input(raw: &RawInput, dpi: f32) -> rim_ui::Input {
    let has = |k: KeyCode| raw.keys.contains(&k);
    rim_ui::Input {
        mouse: (raw.mouse.0 * dpi, raw.mouse.1 * dpi),
        left_pressed: raw.left_pressed,
        left_released: raw.left_released,
        right_pressed: raw.right_pressed,
        right_released: raw.right_released,
        wheel: raw.wheel,
        tab: has(KeyCode::Tab),
        shift: raw.shift,
        enter: has(KeyCode::Enter),
        pressed: raw.pressed.clone(),
        keys: {
            use rim_ui::Key;
            let mut keys: Vec<Key> = raw.chars.iter().map(|&c| Key::Char(c)).collect();
            for &(code, key) in &UI_KEYS {
                if has(code) {
                    keys.push(key);
                }
            }
            keys
        },
        time: raw.time,
    }
}

/// The keys the UI takes as keys: editing a focused input, and moving
/// through a list or a menu. `RawInput::gather_ui` collects these and
/// `ui_input` maps them, from this one list, so an arrow can't be mapped
/// and never collected again.
const UI_KEYS: [(KeyCode, rim_ui::Key); 9] = [
    (KeyCode::Backspace, rim_ui::Key::Backspace),
    (KeyCode::Delete, rim_ui::Key::Delete),
    (KeyCode::Left, rim_ui::Key::Left),
    (KeyCode::Right, rim_ui::Key::Right),
    (KeyCode::Up, rim_ui::Key::Up),
    (KeyCode::Down, rim_ui::Key::Down),
    (KeyCode::Home, rim_ui::Key::Home),
    (KeyCode::End, rim_ui::Key::End),
    (KeyCode::Escape, rim_ui::Key::Escape),
];

/// The camera's answer to a frame of scrolling. A trackpad's travel pans
/// one to one with the fingers (momentum included: macOS keeps sending it
/// after they lift); a wheel's notches zoom at the pointer, a notch a 12%
/// step; Cmd or Ctrl with either zooms. The scroll setting can pin one.
pub(crate) fn scroll_camera(app: &mut App, raw: &RawInput) {
    let (mx, my) = raw.mouse;
    let mut s = raw.scroll;
    if s.travel != (0.0, 0.0) {
        app.last_precise = raw.time;
    } else if s.notches != 0.0 && raw.time - app.last_precise < PRECISE_STICKS {
        s = Scroll { notches: 0.0, travel: (0.0, s.notches * NOTCH * POINTS_PER_UNIT) };
    }
    if s == Scroll::default() {
        return;
    }
    let zoom_in_steps = s.notches + s.travel.1 / 40.0;
    match (raw.zoom_mod, app.scroll_mode) {
        (true, _) | (false, ScrollMode::Zoom) => apply(app, Action::Zoom(1.12f32.powf(zoom_in_steps), mx, my)),
        (false, ScrollMode::Pan) => {
            let (dx, dy) = (s.travel.0, s.travel.1 + s.notches * 40.0);
            apply(app, Action::Pan(-dx / app.cam.zoom, -dy / app.cam.zoom));
        }
        (false, ScrollMode::Auto) => {
            if s.notches != 0.0 {
                apply(app, Action::Zoom(1.12f32.powf(s.notches), mx, my));
            }
            if s.travel != (0.0, 0.0) {
                apply(app, Action::Pan(-s.travel.0 / app.cam.zoom, -s.travel.1 / app.cam.zoom));
            }
        }
    }
}

/// How long a right press is held before it opens the orders menu.
pub(crate) const HOLD_SECS: f64 = 0.35;

/// How far a press may wander, in points, and still be a click.
const CLICK_SLOP: f32 = 6.0;

/// A right press on the map, until it's a click, a hold or a drag.
pub(crate) struct RightPress {
    at: (f32, f32),
    /// Where the pointer was last frame, for a drag's pan.
    last: (f32, f32),
    t: f64,
    moved: bool,
    opened: bool,
}

/// The right button on the map, decided on release rather than press. A
/// tool in hand drops at once; otherwise a press held still opens the
/// orders menu, one that wanders is a drag and orders nothing, and a
/// click gives the selected colonists the first safe order there, or opens
/// the menu when there is none. Nothing damaging ever happens here: that
/// takes a pick from the menu.
pub(crate) fn right_button(app: &mut App, raw: &RawInput, cv: &rim_ui::view::ClientView, captured: bool) {
    let (mx, my) = raw.mouse;
    if raw.right_pressed && !captured {
        if app.tool != Tool::Select {
            app.tool = Tool::Select;
            app.drag_start = None;
            app.refused = None;
            return;
        }
        app.right = Some(RightPress { at: (mx, my), last: (mx, my), t: raw.time, moved: false, opened: false });
    }
    let Some(r) = app.right.as_mut() else { return };
    if !r.moved && !r.opened && ((mx - r.at.0).powi(2) + (my - r.at.1).powi(2)).sqrt() > CLICK_SLOP {
        r.moved = true;
    }
    let (last, moved, held_open) = (r.last, r.moved, r.opened);
    r.last = (mx, my);
    let released = raw.right_released || !raw.right_down;
    // A right-drag pans, the grabbed ground following the pointer: the pan
    // a mouse without a middle button has.
    if moved {
        let z = app.cam.zoom;
        apply(app, Action::Pan(-(mx - last.0) / z, -(my - last.1) / z));
        if released {
            app.right = None;
        }
        return;
    }
    if !held_open && raw.time - r.t >= HOLD_SECS {
        r.opened = true;
        let at = r.at;
        open_orders(app, cv, at);
    }
    if !released {
        return;
    }
    let Some(r) = app.right.take() else { return };
    if r.opened {
        return;
    }
    let (x, y) = r.at;
    let cell = app.cam.tile_at(x, y);
    let on = pawn_under(app, x, y);
    let safe = selection(app).into_iter().any(|e| order::resolve(&app.sim.world, e, cell, on).is_some());
    if safe {
        apply(app, Action::RightClick(x, y));
    } else if !selection(app).is_empty() {
        open_orders(app, cv, (x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wheel_steps_and_a_trackpad_travels() {
        // macOS units: a wheel notch is 10, a trackpad reports points.
        assert_eq!(
            classify(&[(0.0, 10.0), (0.0, 20.0), (0.0, -10.0)], 10.0, 1.0, false),
            Scroll { notches: 2.0, travel: (0.0, 0.0) }
        );
        let pad = classify(&[(0.0, 2.5), (1.25, -3.75)], 10.0, 1.0, false);
        assert_eq!(pad.notches, 0.0, "fractions are a trackpad");
        assert_eq!(pad.travel, (1.25, -1.25), "and travel in points, both axes");
        let sideways = classify(&[(3.0, 0.0)], 10.0, 1.0, false);
        assert_eq!(sideways.travel, (3.0, 0.0), "sideways is travel");
        assert_eq!(classify(&[(10.0, 0.0)], 10.0, 1.0, true).notches, 1.0, "shift-wheel is still the wheel");
        assert_eq!(classify(&[(0.0, 10.0), (0.0, 0.5)], 10.0, 1.0, false), Scroll { notches: 1.0, travel: (0.0, 0.5) });
        // Windows: 120 a notch, a precision touchpad's units a third of a point.
        assert_eq!(
            classify(&[(0.0, 240.0), (0.0, 30.0)], 120.0, 1.0 / 3.0, false),
            Scroll { notches: 2.0, travel: (0.0, 10.0) }
        );
    }

    /// Every key the UI maps is one the frame collects: the arrows once
    /// weren't, and the command palette never heard them.
    #[test]
    fn the_ui_hears_every_key_it_maps() {
        let gathered: Vec<_> = super::RawInput::gathered_keys().collect();
        for (code, key) in super::UI_KEYS {
            assert!(gathered.contains(&code), "{key:?} is mapped but {code:?} is never collected");
        }
        let raw = super::RawInput {
            keys: vec![macroquad::prelude::KeyCode::Up, macroquad::prelude::KeyCode::Down],
            ..Default::default()
        };
        let keys = super::ui_input(&raw, 1.0).keys;
        assert!(keys.contains(&rim_ui::Key::Up) && keys.contains(&rim_ui::Key::Down), "{keys:?}");
    }

    #[test]
    fn function_keys_are_not_text() {
        let plain = |c| super::typed_char(c, false, false);
        assert_eq!(plain('a'), Some('a'));
        assert_eq!(plain('é'), Some('é'));
        assert_eq!(plain(' '), Some(' '));
        assert_eq!(plain('\u{f701}'), None, "macOS down arrow");
        assert_eq!(plain('\u{8}'), None, "backspace");
        assert_eq!(plain('\u{1b}'), None, "escape");
    }

    /// A held Backspace or arrow acts again, as a held letter types again;
    /// Escape and keys the UI doesn't map don't.
    #[test]
    fn a_held_editing_key_repeats() {
        use macroquad::miniquad::{EventHandler, KeyMods};
        use macroquad::prelude::KeyCode::*;
        let mut e = super::Events::default();
        for (code, repeat) in [(Backspace, true), (Left, true), (Down, false), (Escape, true), (A, true), (Tab, true)] {
            e.key_down_event(code, KeyMods::default(), repeat);
        }
        assert_eq!(e.repeats, [Backspace, Left]);
    }

    #[test]
    fn a_command_key_types_nothing() {
        assert_eq!(super::typed_char('k', true, false), None, "Cmd+K on macOS arrives as 'k'");
        assert_eq!(super::typed_char('@', true, true), Some('@'), "AltGr is Ctrl+Alt on Windows");
        assert_eq!(super::typed_char('ø', false, true), Some('ø'), "Option types on macOS");
    }
}
