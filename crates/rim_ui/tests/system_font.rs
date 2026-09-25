//! The UI finds this platform's own UI font (Segoe UI on Windows, SF on
//! macOS, fontconfig's sans-serif on Linux), not whatever font came first,
//! and shapes text with it. Its own test binary, because it points
//! RIM_CACHE_DIR at a scratch folder for the process.

use rim_ui::text::Text;

#[test]
fn the_system_ui_font_is_found_and_shapes_text() {
    let dir = std::env::temp_dir().join(format!("rim-systemfont-{}", std::process::id()));
    // SAFETY: this test binary has one test and sets the variable before any
    // other thread reads it.
    unsafe { std::env::set_var("RIM_CACHE_DIR", &dir) };

    let mut text = Text::new(None, &[]).expect("a font");
    let info = text.info.clone();
    println!("UI font: {} from {} ({} faces)", info.family, info.source, info.fallback_faces);
    assert_ne!(info.source, "first installed font", "a platform UI font, not a guess");
    if cfg!(windows) {
        let s = info.source.to_ascii_lowercase();
        assert!(s.ends_with("segoeui.ttf") || s.ends_with("arial.ttf"), "Segoe UI or Arial, got {}", info.source);
    }

    let one = text.shape("Colony", 14.0, 400, None).width;
    let two = text.shape("Colony Colony", 14.0, 400, None).width;
    assert!(one > 20.0, "real glyph advances, not zero-width boxes ({one})");
    assert!(two > one * 1.8, "longer text is wider ({one} vs {two})");
    let _ = std::fs::remove_dir_all(&dir);
}
