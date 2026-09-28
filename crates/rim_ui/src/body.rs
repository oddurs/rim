//! Bodies (DESIGN.md §6h): what a creature looks like from above, as parts
//! drawn in the plan's ink. Client data, from each mod's `ui/bodies.toml`:
//! a body is the player's side (§10), so a mod that adds or redraws one
//! never touches the sim, and saving the file redraws everyone.
//!
//! A part is a look layer (a disc or a rounded box, a colour, one of the
//! plan's line weights) plus where it sits in the body's frame and, for
//! gaits, which motion moves it. A body is written facing north: forward is
//! -y, in cells, from the creature's centre.

use crate::theme::parse_color;
use rim_sim::look::Weight;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Where a part's colour comes from: a fixed colour, or a channel each
/// creature fills in (from its def today; from appearance later).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Rgba([f32; 4]),
    Skin,
    Feet,
    Torso,
    Hair,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// `r` across, `r * squash` fore and aft.
    Disc { r: f32, squash: f32 },
    /// `w` across, `h` fore and aft, corners rounded by `round` of the
    /// shorter half (1 is a capsule).
    Box { w: f32, h: f32, round: f32 },
}

/// What a gait moves a part with (the gaits item reads these).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Stride,
    Swing,
    Sway,
    Lean,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub id: String,
    pub shape: Shape,
    /// Offset from the creature's centre, in cells, facing north.
    pub at: (f32, f32),
    pub paint: Paint,
    /// An ink outline at this weight, or none.
    pub line: Option<Weight>,
    /// Drawn in the world's frame, not turned with the creature: a shadow.
    pub world: bool,
    /// Drawn only at the full level of detail: hands, feet, ears.
    pub detail: bool,
    pub motion: Option<(Motion, f32)>,
}

/// Where carried things and worn ones attach: a part's centre, or a point.
#[derive(Clone, Debug, PartialEq)]
pub enum Socket {
    Part(String),
    At(f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    /// Qualified: `core:human`.
    pub id: String,
    /// The creatures it draws, qualified.
    pub creatures: Vec<String>,
    pub parts: Vec<Part>,
    pub sockets: BTreeMap<String, Socket>,
    /// Garment layers, innermost first: the outermost one worn colours the
    /// torso.
    pub layers: Vec<String>,
}

#[derive(Default)]
pub struct Bodies {
    pub list: Vec<Body>,
    by_creature: HashMap<String, usize>,
    pub warnings: Vec<String>,
}

const PART_KEYS: &[&str] =
    &["id", "draw", "x", "y", "r", "squash", "w", "h", "round", "color", "line", "world", "detail", "move", "phase"];

impl Bodies {
    /// Every mod's `ui/bodies.toml`, in load order. A body that doesn't
    /// read is left out with a warning naming its file and part; a creature
    /// two bodies claim goes to the later one, and says so.
    pub fn load(mods: &[(String, &Path)]) -> Bodies {
        let mut out = Bodies::default();
        for (mod_id, dir) in mods {
            let path = dir.join("ui").join("bodies.toml");
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    out.warnings.push(format!("{mod_id}/ui/bodies.toml: {e}"));
                    continue;
                }
            };
            let (bodies, errors) = parse(mod_id, &text);
            // A file with nothing in it is a save caught half-written, not
            // a mod that wants its creatures drawn as discs.
            if bodies.is_empty() && errors.is_empty() {
                out.warnings.push(format!("{mod_id}/ui/bodies.toml: declares no [[body]]"));
            }
            out.warnings.extend(errors.into_iter().map(|e| format!("{mod_id}/ui/bodies.toml: {e}")));
            for b in bodies {
                let i = out.list.len();
                for c in &b.creatures {
                    if let Some(was) = out.by_creature.insert(c.clone(), i) {
                        out.warnings.push(format!(
                            "body conflict: {c} is drawn by both {} and {} ({} wins by load order)",
                            out.list[was].id, b.id, b.id
                        ));
                    }
                }
                out.list.push(b);
            }
        }
        out
    }

    /// The body a creature is drawn with, by its qualified id.
    pub fn for_creature(&self, creature: &str) -> Option<&Body> {
        self.by_creature.get(creature).map(|&i| &self.list[i])
    }
}

/// A mod's `bodies.toml`: the bodies that read, and what was wrong with
/// the rest.
pub fn parse(mod_id: &str, text: &str) -> (Vec<Body>, Vec<String>) {
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => return (Vec::new(), vec![e.message().to_string()]),
    };
    let (mut bodies, mut errors) = (Vec::new(), Vec::new());
    for (key, _) in table.iter().filter(|(k, _)| *k != "body") {
        errors.push(format!("unknown table [[{key}]] (only [[body]])"));
    }
    let Some(list) = table.get("body") else { return (bodies, errors) };
    let Some(list) = list.as_array() else {
        errors.push("'body' must be an array of tables ([[body]])".into());
        return (bodies, errors);
    };
    for b in list {
        match body(mod_id, b) {
            Ok(b) => bodies.push(b),
            Err(e) => errors.push(e),
        }
    }
    (bodies, errors)
}

fn qualify(mod_id: &str, id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("{mod_id}:{id}")
    }
}

fn body(mod_id: &str, v: &toml::Value) -> Result<Body, String> {
    let t = v.as_table().ok_or("each [[body]] must be a table")?;
    let bare = t.get("id").and_then(|v| v.as_str()).ok_or("a [[body]] has no id")?;
    if bare.split_once(':').is_some_and(|(owner, _)| owner != mod_id) {
        return Err(format!(
            "body {bare}: a mod names its own bodies (id = \"{}\")",
            bare.split_once(':').map_or(bare, |x| x.1)
        ));
    }
    let id = qualify(mod_id, bare);
    let at = |e: String| format!("body {bare}: {e}");
    if let Some(k) = t.keys().find(|k| !["id", "creatures", "parts", "sockets", "layers"].contains(&k.as_str())) {
        return Err(at(format!("unknown field `{k}` (id, creatures, parts, sockets, layers)")));
    }
    let creatures: Vec<String> = match t.get("creatures") {
        Some(toml::Value::Array(a)) => a
            .iter()
            .map(|c| c.as_str().map(|c| qualify(mod_id, c)).ok_or_else(|| at("creatures are ids".into())))
            .collect::<Result<_, _>>()?,
        _ => return Err(at("`creatures` lists the creatures it draws, like [\"core:human\"]".into())),
    };
    for (i, c) in creatures.iter().enumerate() {
        if creatures[..i].contains(c) {
            return Err(at(format!("{c} is listed twice in `creatures`")));
        }
    }
    let parts: Vec<Part> = match t.get("parts") {
        Some(toml::Value::Array(a)) if !a.is_empty() => a.iter().map(part).collect::<Result<_, _>>().map_err(at)?,
        _ => return Err(at("`parts` is a list of at least one part".into())),
    };
    for (i, p) in parts.iter().enumerate() {
        if parts[..i].iter().any(|q| q.id == p.id) {
            return Err(at(format!("two parts are called `{}`", p.id)));
        }
    }
    let layers: Vec<String> = match t.get("layers") {
        None => Vec::new(),
        Some(toml::Value::Array(a)) => a
            .iter()
            .map(|l| l.as_str().map(str::to_string).ok_or_else(|| at("`layers` are garment layer names".into())))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(at("`layers` lists garment layers, innermost first".into())),
    };
    let mut sockets = BTreeMap::new();
    if let Some(s) = t.get("sockets") {
        let s = s.as_table().ok_or_else(|| at("`sockets` is a table".into()))?;
        for (name, v) in s {
            let socket = match v {
                toml::Value::String(p) if parts.iter().any(|q| &q.id == p) => Socket::Part(p.clone()),
                toml::Value::String(p) => return Err(at(format!("socket {name}: no part `{p}`"))),
                toml::Value::Array(a) if a.len() == 2 => {
                    let n = |i: usize| a[i].as_float().or(a[i].as_integer().map(|n| n as f64));
                    match (n(0), n(1)) {
                        (Some(x), Some(y)) => Socket::At(x as f32, y as f32),
                        _ => return Err(at(format!("socket {name}: give a part id or [x, y]"))),
                    }
                }
                _ => return Err(at(format!("socket {name}: give a part id or [x, y]"))),
            };
            sockets.insert(name.clone(), socket);
        }
    }
    Ok(Body { id, creatures, parts, sockets, layers })
}

fn part(v: &toml::Value) -> Result<Part, String> {
    let t = v.as_table().ok_or("each part is a table")?;
    let id = t.get("id").and_then(|v| v.as_str()).ok_or("a part has no id")?.to_string();
    let at = |e: String| format!("part {id}: {e}");
    if let Some(k) = t.keys().find(|k| !PART_KEYS.contains(&k.as_str())) {
        return Err(at(format!("unknown field `{k}` ({})", PART_KEYS.join(", "))));
    }
    let num = |k: &str, default: f32| -> Result<f32, String> {
        let Some(v) = t.get(k) else { return Ok(default) };
        match v.as_float().or(v.as_integer().map(|n| n as f64)) {
            Some(f) if f.is_finite() => Ok(f as f32),
            _ => Err(at(format!("`{k}` is a number"))),
        }
    };
    let text = |k: &str, default: &str| -> Result<String, String> {
        match t.get(k) {
            None => Ok(default.to_string()),
            Some(v) => v.as_str().map(str::to_string).ok_or_else(|| at(format!("`{k}` is text"))),
        }
    };
    // Sizes are in cells: a part bigger than its cell or of no size is a typo.
    let size = |k: &str, default: f32| -> Result<f32, String> {
        let n = num(k, default)?;
        if n > 0.0 && n <= 1.0 {
            Ok(n)
        } else {
            Err(at(format!("`{k}` = {n}: sizes are cells, above 0 and at most 1")))
        }
    };
    let draw = text("draw", "disc")?;
    let only = |allowed: &[&str]| -> Result<(), String> {
        let shape_keys = ["r", "squash", "w", "h", "round"];
        match shape_keys.iter().find(|k| t.contains_key(**k) && !allowed.contains(k)) {
            Some(k) => Err(at(format!("`{k}` isn't a field of draw = \"{draw}\""))),
            None => Ok(()),
        }
    };
    let shape = match draw.as_str() {
        "disc" => {
            only(&["r", "squash"])?;
            let squash = num("squash", 1.0)?;
            if !(0.2..=5.0).contains(&squash) {
                return Err(at(format!("`squash` = {squash}: between 0.2 and 5")));
            }
            Shape::Disc { r: size("r", 0.1)?, squash }
        }
        "box" => {
            only(&["w", "h", "round"])?;
            let round = num("round", 0.0)?;
            if !(0.0..=1.0).contains(&round) {
                return Err(at(format!("`round` = {round}: a share of the shorter half, 0 to 1")));
            }
            Shape::Box { w: size("w", 0.2)?, h: size("h", 0.2)?, round }
        }
        o => return Err(at(format!("draw = \"{o}\": bodies draw \"disc\" or \"box\""))),
    };
    let color = text("color", "@torso")?;
    let paint = match color.as_str() {
        "@skin" => Paint::Skin,
        "@feet" => Paint::Feet,
        "@torso" => Paint::Torso,
        "@hair" => Paint::Hair,
        c if c.starts_with('@') => {
            return Err(at(format!("color = \"{c}\": the channels are @skin, @feet, @torso and @hair")))
        }
        c => Paint::Rgba(parse_color(c).ok_or_else(|| at(format!("color = \"{c}\": \"#rrggbb\" or \"#rrggbbaa\"")))?),
    };
    let line = match t.get("line") {
        None => None,
        Some(_) => Some(Weight::parse(&text("line", "")?).map_err(at)?),
    };
    let flag =
        |k: &str| t.get(k).map_or(Ok(false), |v| v.as_bool().ok_or_else(|| at(format!("`{k}` is true or false"))));
    if t.contains_key("phase") && !t.contains_key("move") {
        return Err(at("`phase` is where in a `move` the part is, and it has no `move`".into()));
    }
    let motion = match t.get("move") {
        None => None,
        Some(_) => {
            let m = match text("move", "")?.as_str() {
                "stride" => Motion::Stride,
                "swing" => Motion::Swing,
                "sway" => Motion::Sway,
                "lean" => Motion::Lean,
                o => return Err(at(format!("move = \"{o}\": stride, swing, sway or lean"))),
            };
            Some((m, num("phase", 0.0)?))
        }
    };
    let (x, y) = (num("x", 0.0)?, num("y", 0.0)?);
    // A part stays within a cell of the creature's centre, so a figure is
    // never culled while a part of it is on screen.
    if x.abs() > 1.0 || y.abs() > 1.0 {
        return Err(at(format!("`x`, `y` = {x}, {y}: a part sits within a cell of the centre")));
    }
    Ok(Part { id: id.clone(), shape, at: (x, y), paint, line, world: flag("world")?, detail: flag("detail")?, motion })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE: &str = include_str!("../../../mods/core/ui/bodies.toml");

    #[test]
    fn cores_bodies_read_cleanly() {
        let (bodies, errors) = parse("core", CORE);
        assert!(errors.is_empty(), "{errors:?}");
        for c in ["core:human", "core:deer", "core:wolf", "core:hare"] {
            assert!(bodies.iter().any(|b| b.creatures.iter().any(|x| x == c)), "a body draws {c}");
        }
    }

    /// Every TOML sample in docs/modding/bodies.md reads cleanly.
    #[test]
    fn the_guides_samples_read() {
        let guide = include_str!("../../../docs/modding/bodies.md").replace("\r\n", "\n");
        let samples: Vec<&str> =
            guide.split("```toml\n").skip(1).map(|s| s.split("```").next().unwrap_or("")).collect();
        assert!(!samples.is_empty(), "the guide has samples");
        for s in samples {
            let (bodies, errors) = parse("my_mod", s);
            assert!(errors.is_empty() && !bodies.is_empty(), "{errors:?} in\n{s}");
        }
    }

    #[test]
    fn a_bad_part_names_its_body_and_part() {
        let err = |text: &str| parse("m", text).1.join(" | ");
        let base = "[[body]]\nid = \"crab\"\ncreatures = [\"crab\"]\nparts = [";
        assert!(err(&format!("{base}{{ id = \"shell\", draw = \"disc\", r = 0.3, sqash = 2 }}]"))
            .contains("body crab: part shell: unknown field `sqash`"));
        assert!(err(&format!("{base}{{ id = \"shell\", r = 2.0 }}]")).contains("part shell: `r` = 2: sizes are cells"));
        assert!(err(&format!("{base}{{ id = \"shell\", draw = \"star\" }}]")).contains("part shell: draw = \"star\""));
        assert!(err(&format!("{base}{{ id = \"shell\", color = \"@fur\" }}]")).contains("part shell: color = \"@fur\""));
        assert!(err(&format!("{base}{{ id = \"shell\", line = \"bold\" }}]")).contains("part shell:"));
        assert!(err(&format!("{base}{{ id = \"shell\", draw = \"box\", r = 0.2 }}]"))
            .contains("`r` isn't a field of draw = \"box\""));
        assert!(err("[[body]]\nid = \"crab\"\nparts = [{ id = \"a\" }]").contains("body crab: `creatures`"));
        let (ok, _) = parse("m", &format!("{base}{{ id = \"shell\" }}]\nsockets = {{ hold = \"claw\" }}"));
        assert!(ok.is_empty(), "a socket on a part that isn't there leaves the body out");
        assert!(err(&format!("{base}{{ id = \"shell\", color = 5 }}]")).contains("part shell: `color` is text"));
        assert!(err(&format!("{base}{{ id = \"shell\", x = nan }}]")).contains("part shell: `x` is a number"));
        assert!(err(&format!("{base}{{ id = \"shell\", y = 3.0 }}]")).contains("within a cell of the centre"));
        assert!(err(&format!("{base}{{ id = \"shell\", phase = 0.5 }}]")).contains("it has no `move`"));
        assert!(err(&format!("{base}{{ id = \"a\" }}, {{ id = \"a\" }}]")).contains("two parts are called `a`"));
        let foreign = "[[body]]\nid = \"core:human\"\ncreatures = [\"x\"]\nparts = [{ id = \"a\" }]";
        assert!(err(foreign).contains("a mod names its own bodies"), "{}", err(foreign));
        // "crab" and "m:crab" are one creature: listed twice, not drawn twice.
        let twice = "[[body]]\nid = \"crab\"\ncreatures = [\"crab\", \"m:crab\"]\nparts = [{ id = \"shell\" }]";
        assert!(err(twice).contains("body crab: m:crab is listed twice"), "{}", err(twice));
    }

    #[test]
    fn ids_qualify_and_the_later_body_wins_a_creature() {
        let dir = std::env::temp_dir().join(format!("rim-bodies-{}", std::process::id()));
        let (a, b) = (dir.join("a"), dir.join("b"));
        for (d, text) in [
            (&a, "[[body]]\nid = \"crab\"\ncreatures = [\"crab\"]\nparts = [{ id = \"shell\" }]"),
            (&b, "[[body]]\nid = \"crab\"\ncreatures = [\"a:crab\"]\nparts = [{ id = \"shell\", r = 0.3 }]"),
        ] {
            std::fs::create_dir_all(d.join("ui")).unwrap();
            std::fs::write(d.join("ui/bodies.toml"), text).unwrap();
        }
        let all = Bodies::load(&[("a".into(), a.as_path()), ("b".into(), b.as_path())]);
        assert_eq!(all.for_creature("a:crab").map(|x| x.id.as_str()), Some("b:crab"));
        assert!(
            all.warnings.iter().any(|w| w.contains("a:crab is drawn by both a:crab and b:crab")),
            "{:?}",
            all.warnings
        );
        // Saved empty, or caught half-written: said, not taken for no bodies.
        std::fs::write(b.join("ui/bodies.toml"), "").unwrap();
        let empty = Bodies::load(&[("b".into(), b.as_path())]);
        assert!(empty.warnings.iter().any(|w| w == "b/ui/bodies.toml: declares no [[body]]"), "{:?}", empty.warnings);
        std::fs::remove_dir_all(&dir).ok();
    }
}
