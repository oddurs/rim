//! A disk cache of the system font list, and a background preload.
//!
//! `fontdb::Database::load_system_fonts` opens and parses every installed
//! font: about 220 ms for ~900 faces on a stock Mac, every launch. The list of
//! faces (file, index, family names, style, weight, stretch) is all cosmic-text
//! needs up front; font data itself loads lazily when a face is first used.
//! So we store the list, and rebuild the database from it in a few ms.
//!
//! The cache is trusted only while every font file it names still has the
//! same size and modification time, and every directory holding them the same
//! modification time (so an added or removed font invalidates it). It is also
//! rebuilt after 30 days, for fonts installed in brand-new directories.
//!
//! On a first run (or a stale cache) the load happens anyway, so the client
//! starts it on a background thread as early as possible (`preload`) and joins
//! it when the UI is built.

use cosmic_text::fontdb::{Database, FaceInfo, Language, Source, Stretch, Style, Weight, ID};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = "rim-fonts 1";
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);

/// How the font list was obtained, for the profiler and tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontsFrom {
    /// Read from the cache.
    Cache,
    /// Scanned (no cache, or it was stale), and the cache was written.
    Scan,
}

/// Where the cache lives: `RIM_CACHE_DIR`, else the platform cache folder.
fn cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("RIM_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")));
    #[cfg(not(any(unix, windows)))]
    let base: Option<PathBuf> = None;
    base.map(|b| b.join("rim"))
}

fn cache_file() -> Option<PathBuf> {
    cache_dir().map(|d| d.join("fonts-v1.txt"))
}

fn stamp(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((m.len(), t.as_nanos() as u64))
}

fn style_code(s: Style) -> u8 {
    match s {
        Style::Normal => 0,
        Style::Italic => 1,
        Style::Oblique => 2,
    }
}

fn style_from(c: u8) -> Style {
    match c {
        1 => Style::Italic,
        2 => Style::Oblique,
        _ => Style::Normal,
    }
}

fn stretch_from(n: u16) -> Stretch {
    match n {
        1 => Stretch::UltraCondensed,
        2 => Stretch::ExtraCondensed,
        3 => Stretch::Condensed,
        4 => Stretch::SemiCondensed,
        6 => Stretch::SemiExpanded,
        7 => Stretch::Expanded,
        8 => Stretch::ExtraExpanded,
        9 => Stretch::UltraExpanded,
        _ => Stretch::Normal,
    }
}

/// Escape a field for our tab-separated lines.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\u{1f}', "\\u")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('u') => out.push('\u{1f}'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Write the cache for `db`. Only file-backed faces can be cached.
fn write(db: &Database, path: &Path) -> std::io::Result<()> {
    let mut files: BTreeMap<PathBuf, (u64, u64)> = BTreeMap::new();
    let mut dirs: BTreeMap<PathBuf, (u64, u64)> = BTreeMap::new();
    let mut faces = String::new();
    for f in db.faces() {
        let Source::File(p) = &f.source else { continue };
        let Some(st) = stamp(p) else { continue };
        files.insert(p.clone(), st);
        if let Some(d) = p.parent() {
            if let Some(ds) = stamp(d) {
                dirs.insert(d.to_path_buf(), ds);
            }
        }
        let families: Vec<String> = f.families.iter().map(|(n, _)| esc(n)).collect();
        faces.push_str(&format!(
            "F\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            esc(&p.to_string_lossy()),
            f.index,
            f.weight.0,
            style_code(f.style),
            f.stretch.to_number(),
            f.monospaced as u8,
            esc(&f.post_script_name),
            families.join("\u{1f}"),
        ));
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let mut out = format!("{VERSION}\t{now}\n");
    for (p, (len, t)) in &files {
        out.push_str(&format!("P\t{}\t{len}\t{t}\n", esc(&p.to_string_lossy())));
    }
    for (d, (_, t)) in &dirs {
        out.push_str(&format!("D\t{}\t{t}\n", esc(&d.to_string_lossy())));
    }
    out.push_str(&faces);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write then rename, so a crash never leaves half a cache.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(tmp, path)
}

/// Read the cache into `db` if it's still valid. Returns false (and leaves
/// `db` untouched) if it's missing, old or stale.
fn read(db: &mut Database, path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else { return false };
    let mut lines = text.lines();
    let Some(head) = lines.next() else { return false };
    let Some((version, created)) = head.split_once('\t') else { return false };
    if version != VERSION {
        return false;
    }
    let created: u64 = created.parse().unwrap_or(0);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    if now.saturating_sub(created) > MAX_AGE.as_secs() {
        return false;
    }
    let mut faces = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        match parts.first() {
            Some(&"P") if parts.len() == 4 => {
                let p = PathBuf::from(unesc(parts[1]));
                let want = (parts[2].parse().unwrap_or(0), parts[3].parse().unwrap_or(0));
                if stamp(&p) != Some(want) {
                    return false;
                }
            }
            Some(&"D") if parts.len() == 3 => {
                let d = PathBuf::from(unesc(parts[1]));
                if stamp(&d).map(|s| s.1) != parts[2].parse().ok() {
                    return false;
                }
            }
            Some(&"F") if parts.len() == 9 => {
                let families = parts[8]
                    .split('\u{1f}')
                    .filter(|s| !s.is_empty())
                    .map(|n| (unesc(n), Language::English_UnitedStates))
                    .collect();
                faces.push(FaceInfo {
                    id: ID::dummy(),
                    source: Source::File(PathBuf::from(unesc(parts[1]))),
                    index: parts[2].parse().unwrap_or(0),
                    weight: Weight(parts[3].parse().unwrap_or(400)),
                    style: style_from(parts[4].parse().unwrap_or(0)),
                    stretch: stretch_from(parts[5].parse().unwrap_or(5)),
                    monospaced: parts[6] == "1",
                    post_script_name: unesc(parts[7]),
                    families,
                });
            }
            _ => return false,
        }
    }
    if faces.is_empty() {
        return false;
    }
    for f in faces {
        db.push_face_info(f);
    }
    true
}

/// Every installed font, from the cache when it's valid, else by scanning
/// (and then writing the cache).
pub fn system_fonts() -> (Database, FontsFrom) {
    let mut db = Database::new();
    if let Some(path) = cache_file() {
        if read(&mut db, &path) {
            return (db, FontsFrom::Cache);
        }
        db = Database::new();
        db.load_system_fonts();
        if let Err(e) = write(&db, &path) {
            eprintln!("rim_ui: couldn't write the font cache {}: {e}", path.display());
        }
        return (db, FontsFrom::Scan);
    }
    db.load_system_fonts();
    (db, FontsFrom::Scan)
}

static PRELOAD: OnceLock<Mutex<Option<JoinHandle<(Database, FontsFrom)>>>> = OnceLock::new();

/// Start finding the system fonts on a background thread. Call as early as
/// possible (before loading mods and generating the map); `take` joins it.
pub fn preload() {
    let slot = PRELOAD.get_or_init(|| Mutex::new(None));
    let mut g = slot.lock().unwrap();
    if g.is_none() {
        *g = Some(std::thread::spawn(system_fonts));
    }
}

/// The preloaded fonts if `preload` was called, else load them now.
pub fn take() -> (Database, FontsFrom) {
    let handle = PRELOAD.get().and_then(|m| m.lock().unwrap().take());
    match handle.map(|h| h.join()) {
        Some(Ok(r)) => r,
        _ => system_fonts(),
    }
}
