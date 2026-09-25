//! What `rim check` asks of a mod's UI scripts before any of them runs: the
//! surface version it targets is one this engine provides, and every
//! `ui.`, `act.` and `view.` member it names exists. A mod that names a
//! missing view call is told at check time, by name, not at first hover.

use crate::api::{UI_API, UI_API_VERSION};
use std::path::Path;

/// Problems with a mod's UI, each naming the file and line. Empty is clean.
pub fn check_mod_ui(mod_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let manifest = match rim_sim::modloader::read_manifest(mod_dir) {
        Ok(m) => m,
        Err(e) => return vec![e],
    };
    if let Some(v) = &manifest.ui_api {
        if let Err(e) = check_version(&manifest.id, v) {
            out.push(e);
            return out;
        }
    }
    let files = crate::vm::ui_files(mod_dir);
    for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "luau")) {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let shown = format!("{}/ui/{}", manifest.id, f.file_name().unwrap_or_default().to_string_lossy());
        for (line, name) in member_refs(&src) {
            if !UI_API.iter().any(|d| d.name == name) {
                let hint = closest(&name).map(|c| format!(" (did you mean {c}?)")).unwrap_or_default();
                out.push(format!(
                    "{shown}:{line}: {name} is not in the UI API {}.{}{hint}",
                    UI_API_VERSION.0, UI_API_VERSION.1
                ));
            }
        }
    }
    out
}

fn check_version(id: &str, v: &str) -> Result<(), String> {
    let parse = |s: &str| -> Option<(u32, u32)> {
        let (a, b) = s.split_once('.')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    };
    let (maj, min) = parse(v).ok_or_else(|| format!("mod '{id}': bad ui_api version '{v}'"))?;
    let ok = maj == UI_API_VERSION.0 && if maj == 0 { min == UI_API_VERSION.1 } else { min <= UI_API_VERSION.1 };
    if ok {
        Ok(())
    } else {
        Err(format!("mod '{id}' targets ui_api {v} but the engine provides {}.{}", UI_API_VERSION.0, UI_API_VERSION.1))
    }
}

/// The member of the API nearest to a wrong name, by a crude distance.
fn closest(name: &str) -> Option<&'static str> {
    let (ns, member) = name.split_once('.')?;
    UI_API
        .iter()
        .filter(|d| d.name.starts_with(ns) && d.name.as_bytes().get(ns.len()) == Some(&b'.'))
        .map(|d| (distance(member, &d.name[ns.len() + 1..]), d.name))
        .filter(|(dist, _)| *dist <= 3)
        .min_by_key(|(dist, _)| *dist)
        .map(|(_, n)| n)
}

/// Levenshtein distance, small strings only.
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Every `ui.x`, `act.x` and `view.x` in code (not in comments or
/// strings), with its line. A small scanner rather than a parser: Luau
/// strings and comments are the only places a member-looking token can
/// hide.
pub fn member_refs(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let (mut i, mut line) = (0, 1);
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
        } else if c == b'-' && b.get(i + 1) == Some(&b'-') {
            // A comment: to the end of the line, or a long bracket.
            i += 2;
            if let Some(n) = long_bracket(b, i) {
                let (skipped, lines) = skip_long(b, i, n);
                i = skipped;
                line += lines;
            } else {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
        } else if c == b'"' || c == b'\'' {
            i += 1;
            while i < b.len() && b[i] != c {
                if b[i] == b'\\' {
                    i += 1;
                }
                if i < b.len() && b[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 1;
        } else if c == b'`' {
            // An interpolated string: `{expr}` parts are code, the rest is text.
            i += 1;
            while i < b.len() && b[i] != b'`' {
                if b[i] == b'{' {
                    let start = i + 1;
                    let mut depth = 1;
                    i += 1;
                    while i < b.len() && depth > 0 {
                        depth += usize::from(b[i] == b'{');
                        depth -= usize::from(b[i] == b'}');
                        i += 1;
                    }
                    for (l, n) in member_refs(&src[start..i.saturating_sub(1)]) {
                        out.push((line + l - 1, n));
                    }
                } else {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            i += 1;
        } else if let Some(n) = long_bracket(b, i) {
            let (skipped, lines) = skip_long(b, i, n);
            i = skipped;
            line += lines;
        } else if is_ident(c) && (i == 0 || !is_ident(b[i - 1]) && b[i - 1] != b'.' && b[i - 1] != b':') {
            let start = i;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
            let ns = &src[start..i];
            if matches!(ns, "ui" | "act" | "view") && b.get(i) == Some(&b'.') {
                let m0 = i + 1;
                let mut m = m0;
                while m < b.len() && is_ident(b[m]) {
                    m += 1;
                }
                if m > m0 && !b[m0].is_ascii_digit() {
                    out.push((line, format!("{ns}.{}", &src[m0..m])));
                }
                i = m;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `[[` or `[=*[` at `i`: the level, if it opens a long bracket.
fn long_bracket(b: &[u8], i: usize) -> Option<usize> {
    if b.get(i) != Some(&b'[') {
        return None;
    }
    let mut n = 0;
    while b.get(i + 1 + n) == Some(&b'=') {
        n += 1;
    }
    (b.get(i + 1 + n) == Some(&b'[')).then_some(n)
}

/// Skip a long bracket of level `n` starting at `i`: (index after it, lines crossed).
fn skip_long(b: &[u8], i: usize, n: usize) -> (usize, usize) {
    let close: Vec<u8> =
        std::iter::once(b']').chain(std::iter::repeat_n(b'=', n)).chain(std::iter::once(b']')).collect();
    let mut j = i + n + 2;
    let mut lines = 0;
    while j < b.len() {
        if b[j..].starts_with(&close) {
            return (j + close.len(), lines);
        }
        if b[j] == b'\n' {
            lines += 1;
        }
        j += 1;
    }
    (b.len(), lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_are_found_in_code_only() {
        let src = "local a = view.tick() -- view.nope in a comment\n\
                   local s = \"act.nope in a string\" .. 'ui.nope'\n\
                   --[[ view.nope\n in a block ]] act.select(1)\n\
                   local t = `now {view.hour()} ui.nope`\n\
                   x.view.tick() ui.t(\"k\", \"ui.nope\")\n";
        let got = member_refs(src);
        assert_eq!(
            got,
            vec![
                (1, "view.tick".to_string()),
                (4, "act.select".to_string()),
                (5, "view.hour".to_string()),
                (6, "ui.t".to_string()),
            ]
        );
    }

    #[test]
    fn a_missing_call_is_named_with_its_line_and_a_hint() {
        let dir = std::env::temp_dir().join(format!("rim-uicheck-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(dir.join("mod.toml"), "id = \"probe\"\nname = \"p\"\nversion = \"0\"\napi = \"0.4\"\n").unwrap();
        std::fs::write(dir.join("ui/a.luau"), "local n = view.tick()\nlocal m = view.tikc()\nact.selct(1)\n").unwrap();
        let problems = check_mod_ui(&dir);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("probe/ui/a.luau:2: view.tikc is not in the UI API"), "{}", problems[0]);
        assert!(problems[0].ends_with("(did you mean view.tick?)"), "{}", problems[0]);
        assert!(problems[1].starts_with("probe/ui/a.luau:3: act.selct"), "{}", problems[1]);
        // The version the mod targets must be one this engine provides.
        std::fs::write(
            dir.join("mod.toml"),
            "id = \"probe\"\nname = \"p\"\nversion = \"0\"\napi = \"0.4\"\nui_api = \"0.9\"\n",
        )
        .unwrap();
        let problems = check_mod_ui(&dir);
        assert_eq!(problems, vec!["mod 'probe' targets ui_api 0.9 but the engine provides 0.1".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
