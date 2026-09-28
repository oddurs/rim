//! Terms: the one shape every climate value takes.
//!
//! A value is a sum of labelled terms. A term is a scale times a product of
//! inputs, and each input can go through a piecewise-linear curve:
//!
//! ```toml
//! [field.ambient.day]
//! scale = 9.0
//! of = [{ input = "hour", curve = [[3, -1.0], [15, 1.0], [24, -0.5]] }]
//! ```
//!
//! Terms are tables keyed by label (not arrays) so a patch can change one
//! term and conflicts are reported per term. They compile at load into flat
//! programs and evaluate in integer fixed point: no floats and no
//! transcendental maths, so lockstep holds on every platform.
//!
//! Inputs are global (time of day and year, a cycle of any number of days,
//! where a sky body is, other fields' outdoor values, noise, constants), plus those read at the cell a derived or stock field
//! is worked out at: `field`, another field's value there (its outdoor value
//! where there is no cell); `terrain`, a property of the ground there;
//! `near`, how many cells it is to the nearest terrain with a tag; and
//! `input = "sky"`, 0 in an enclosed room and 1 elsewhere. A stock field's
//! rate also reads its own value there (`self`), the value its `base` terms
//! settle to (`base`), and how far it is above that (`above_base`).

use crate::rng::mix;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Fixed-point scale for term evaluation: 1.0 is `Q`.
pub const Q: i64 = 10_000;

/// Most points a curve may have.
pub const MAX_POINTS: usize = 16;

/// How far `near` looks, in cells: anything farther reads as this.
pub const NEAR_CAP: u8 = 16;

pub fn to_q(v: f64) -> i64 {
    (v * Q as f64).round() as i64
}

pub fn from_q(v: i64) -> f64 {
    v as f64 / Q as f64
}

// ---------------------------------------------------------------- data

fn d_one() -> f64 {
    1.0
}

/// One term as written in TOML.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TermDef {
    #[serde(default = "d_one")]
    pub scale: f64,
    #[serde(default)]
    pub of: Vec<InputDef>,
}

/// An input: a plain number, or a table naming a source.
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum InputDef {
    Const(f64),
    Source(Box<SourceDef>),
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct SourceDef {
    /// `"year"` (0..1), `"hour"` (0..24), `"cycle"` (0..1 over `days`),
    /// `"sky"` (0 in an enclosed room, 1 elsewhere), `"depth"` (levels
    /// below the surface, 0 on it), `"body"` (where a sky body is), or for
    /// a stock field's rate `"self"`, `"base"` or `"above_base"`.
    pub input: Option<String>,
    /// For `input = "body"`: which `[[sky_body]]`, and what of it:
    /// `"altitude"` or `"azimuth"` (degrees), `"up"` or `"phase"` (0..1).
    pub body: Option<String>,
    pub of: Option<String>,
    /// Another field's outdoor value.
    pub ambient: Option<String>,
    /// Another field's value at the cell being read, for a derived field;
    /// where there is no cell (an outdoor value), its outdoor value.
    pub field: Option<String>,
    /// Smooth noise in -1..1, keyed so different uses don't move together.
    pub noise: Option<String>,
    /// A property of the terrain at the cell being read (`[[terrain]]
    /// props`); 0 where the terrain doesn't give it, or there is no cell.
    pub terrain: Option<String>,
    /// Cells to the nearest terrain with this tag, up to `NEAR_CAP`; the
    /// cap where none is that close, or there is no cell.
    pub near: Option<String>,
    /// Noise period in game hours.
    #[serde(default)]
    pub hours: f64,
    /// A cycle's period in game days: a moon's phases, an eclipse's season.
    #[serde(default)]
    pub days: f64,
    /// How far into its cycle the game starts, in days.
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub curve: Vec<[f64; 2]>,
}

/// Terms by label. A BTreeMap so evaluation and explanation order is stable.
pub type TermsDef = BTreeMap<String, TermDef>;

// ---------------------------------------------------------------- compiled

#[derive(Clone, Debug, PartialEq)]
enum Src {
    Year,
    Hour,
    /// Ticks into a cycle at tick 0, and its period in ticks.
    Cycle {
        offset: u64,
        period: u64,
    },
    Depth,
    Body {
        body: usize,
        of: BodyOf,
    },
    Sky,
    Own,
    Base,
    AboveBase,
    Ambient(usize),
    Field(usize),
    Noise {
        key: u64,
        period: u64,
    },
    Terrain(usize),
    Near(usize),
    Const(i64),
}

#[derive(Clone, Debug)]
pub struct Curve {
    xs: Vec<i64>,
    ys: Vec<i64>,
}

impl Curve {
    pub fn compile(points: &[[f64; 2]]) -> Result<Curve, String> {
        if points.is_empty() {
            return Err("empty curve".into());
        }
        if points.len() > MAX_POINTS {
            return Err(format!("curve has {} points (at most {MAX_POINTS})", points.len()));
        }
        let xs: Vec<i64> = points.iter().map(|p| to_q(p[0])).collect();
        if xs.windows(2).any(|w| w[1] <= w[0]) {
            return Err("curve x values must increase".into());
        }
        Ok(Curve { xs, ys: points.iter().map(|p| to_q(p[1])).collect() })
    }

    /// Piecewise-linear, clamped at both ends. Integer maths only.
    pub fn eval(&self, x: i64) -> i64 {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1];
        }
        let mut i = 1;
        while self.xs[i] < x {
            i += 1;
        }
        let (x0, x1, y0, y1) = (self.xs[i - 1], self.xs[i], self.ys[i - 1], self.ys[i]);
        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    }
}

#[derive(Clone, Debug)]
struct Input {
    src: Src,
    curve: Option<Curve>,
}

#[derive(Clone, Debug)]
pub struct Term {
    pub label: String,
    scale: i64,
    inputs: Vec<Input>,
}

/// A compiled term list.
#[derive(Clone, Debug, Default)]
pub struct Terms {
    pub terms: Vec<Term>,
}

/// What of a sky body a term reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyOf {
    Altitude,
    Azimuth,
    Up,
    Phase,
}

impl BodyOf {
    pub fn read(self, b: &crate::sky::BodyState) -> i64 {
        to_q(match self {
            BodyOf::Altitude => b.altitude,
            BodyOf::Azimuth => b.azimuth,
            BodyOf::Up => b.up,
            BodyOf::Phase => b.phase,
        })
    }
}

/// What terms can read. Values in `Q` units.
pub trait Env {
    /// Fraction of the year, 0..Q.
    fn year(&self) -> i64;
    /// Hour of the day, 0..24·Q.
    fn hour(&self) -> i64;
    fn ambient(&self, field: usize) -> i64;
    /// A field's value where the terms are read; outdoors by default.
    fn field(&self, field: usize) -> i64 {
        self.ambient(field)
    }
    fn tick(&self) -> u64;
    fn seed(&self) -> u64;
    /// A terrain property where the terrain is read; nothing by default.
    fn terrain(&self, _prop: usize) -> i64 {
        0
    }
    /// Cells to the nearest terrain with a tag, times `Q`; beyond reach by
    /// default.
    fn near(&self, _tag: usize) -> i64 {
        NEAR_CAP as i64 * Q
    }
    /// 0 in an enclosed room, `Q` elsewhere; open sky by default.
    fn sky(&self) -> i64 {
        Q
    }
    /// Levels below the surface, times `Q`: 0 on it, and by default.
    fn depth(&self) -> i64 {
        0
    }
    /// Something of a sky body, times `Q`; 0 by default.
    fn body(&self, _body: usize, _of: BodyOf) -> i64 {
        0
    }
    /// A stock field's own value where it is being worked out.
    fn own(&self) -> i64 {
        0
    }
    /// What a stock field's `base` terms give where it is being worked out.
    fn base(&self) -> i64 {
        0
    }
}

/// What the names in terms resolve to, as indices.
pub trait Names {
    fn field(&self, id: &str) -> Option<usize>;
    /// A terrain property, by name.
    fn prop(&self, _name: &str) -> Option<usize> {
        None
    }
    /// A terrain tag, by name.
    fn tag(&self, _name: &str) -> Option<usize> {
        None
    }
    /// A sky body, by id.
    fn body(&self, _id: &str) -> Option<usize> {
        None
    }
}

/// Where only fields have names.
impl<F: Fn(&str) -> Option<usize>> Names for F {
    fn field(&self, id: &str) -> Option<usize> {
        self(id)
    }
}

impl Terms {
    /// `names` resolves what the terms name. `ctx` names the def in errors
    /// and warnings. A terrain property or tag no terrain has is a warning,
    /// not an error: a mod may read one another mod adds.
    pub fn compile(def: &TermsDef, ctx: &str, names: &dyn Names, warnings: &mut Vec<String>) -> Result<Terms, String> {
        let mut terms = Vec::new();
        for (label, t) in def {
            let here = format!("{ctx}, term '{label}'");
            let mut inputs = Vec::new();
            for i in &t.of {
                inputs.push(match i {
                    InputDef::Const(v) => Input { src: Src::Const(to_q(*v)), curve: None },
                    InputDef::Source(s) => compile_source(s, &here, names, warnings)?,
                });
            }
            terms.push(Term { label: label.clone(), scale: to_q(t.scale), inputs });
        }
        Ok(Terms { terms })
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Fields this list reads, for dependency ordering.
    pub fn reads(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .terms
            .iter()
            .flat_map(|t| t.inputs.iter())
            .filter_map(|i| match i.src {
                Src::Ambient(f) | Src::Field(f) => Some(f),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Whether any term reads a stock field's own state: `self`, `base` or
    /// `above_base`.
    pub fn reads_own(&self) -> bool {
        self.terms.iter().flat_map(|t| t.inputs.iter()).any(|i| matches!(i.src, Src::Own | Src::Base | Src::AboveBase))
    }

    /// Terrain tags this list reads with `near`.
    pub fn nears(&self) -> impl Iterator<Item = usize> + '_ {
        self.terms.iter().flat_map(|t| t.inputs.iter()).filter_map(|i| match i.src {
            Src::Near(t) => Some(t),
            _ => None,
        })
    }

    fn term(t: &Term, env: &dyn Env) -> i64 {
        let mut acc = t.scale;
        for i in &t.inputs {
            let mut v = match i.src {
                Src::Year => env.year(),
                Src::Hour => env.hour(),
                Src::Cycle { offset, period } => cycle(env.tick(), offset, period),
                Src::Depth => env.depth(),
                Src::Body { body, of } => env.body(body, of),
                Src::Sky => env.sky(),
                Src::Own => env.own(),
                Src::Base => env.base(),
                Src::AboveBase => env.own() - env.base(),
                Src::Ambient(f) => env.ambient(f),
                Src::Field(f) => env.field(f),
                Src::Noise { key, period } => noise(env.seed() ^ key, env.tick(), period),
                Src::Terrain(p) => env.terrain(p),
                Src::Near(t) => env.near(t),
                Src::Const(c) => c,
            };
            if let Some(c) = &i.curve {
                v = c.eval(v);
            }
            acc = acc * v / Q;
        }
        acc
    }

    pub fn eval(&self, env: &dyn Env) -> i64 {
        self.terms.iter().map(|t| Self::term(t, env)).sum()
    }

    /// Each term's contribution, by label.
    pub fn explain(&self, env: &dyn Env) -> Vec<(String, i64)> {
        self.terms.iter().map(|t| (t.label.clone(), Self::term(t, env))).collect()
    }
}

fn compile_source(s: &SourceDef, ctx: &str, names: &dyn Names, warnings: &mut Vec<String>) -> Result<Input, String> {
    let named = [s.input.is_some(), s.ambient.is_some(), s.field.is_some(), s.noise.is_some()]
        .iter()
        .chain(&[s.terrain.is_some(), s.near.is_some()])
        .filter(|b| **b)
        .count();
    if named != 1 {
        return Err(format!(
            "{ctx}: an input needs exactly one of `input`, `ambient`, `field`, `noise`, `terrain` or `near`"
        ));
    }
    if (s.days != 0.0 || s.offset != 0.0) && s.input.as_deref() != Some("cycle") {
        return Err(format!("{ctx}: `days` and `offset` are for `input = \"cycle\"`"));
    }
    if (s.body.is_some() || s.of.is_some()) && s.input.as_deref() != Some("body") {
        return Err(format!("{ctx}: `body` and `of` are for `input = \"body\"`"));
    }
    let src = if let Some(i) = &s.input {
        match i.as_str() {
            "year" => Src::Year,
            "hour" => Src::Hour,
            "cycle" => {
                // An hour at least, and short of a million years: a moon
                // whose cycle rounds to nothing would never wax.
                if !((1.0 / 24.0..=3.65e8).contains(&s.days) && s.offset.is_finite()) {
                    return Err(format!("{ctx}: a cycle needs `days` of an hour (1/24) or more"));
                }
                let period = (s.days * crate::TICKS_PER_DAY as f64).round() as i64;
                let offset = ((s.offset * crate::TICKS_PER_DAY as f64).round() as i64).rem_euclid(period);
                Src::Cycle { offset: offset as u64, period: period as u64 }
            }
            "depth" => Src::Depth,
            "body" => {
                let id = s.body.as_deref().ok_or_else(|| format!("{ctx}: `input = \"body\"` needs a `body`"))?;
                let body = names.body(id).ok_or_else(|| format!("{ctx}: unknown sky body '{id}'"))?;
                let of = match s.of.as_deref() {
                    Some("altitude") => BodyOf::Altitude,
                    Some("azimuth") => BodyOf::Azimuth,
                    Some("up") => BodyOf::Up,
                    Some("phase") => BodyOf::Phase,
                    other => {
                        return Err(format!(
                            "{ctx}: a body input needs `of` = \"altitude\", \"azimuth\", \"up\" or \"phase\", not {other:?}"
                        ))
                    }
                };
                Src::Body { body, of }
            }
            "sky" => Src::Sky,
            "self" => Src::Own,
            "base" => Src::Base,
            "above_base" => Src::AboveBase,
            other => {
                return Err(format!(
                    "{ctx}: unknown input '{other}' (have: year, hour, cycle, depth, body, sky, self, base, above_base)"
                ))
            }
        }
    } else if let Some(a) = &s.ambient {
        Src::Ambient(names.field(a).ok_or_else(|| format!("{ctx}: unknown field '{a}'"))?)
    } else if let Some(f) = &s.field {
        Src::Field(names.field(f).ok_or_else(|| format!("{ctx}: unknown field '{f}'"))?)
    } else if let Some(p) = &s.terrain {
        names.prop(p).map(Src::Terrain).unwrap_or_else(|| {
            warnings.push(format!("{ctx}: no terrain has the property '{p}', so it reads 0"));
            Src::Const(0)
        })
    } else if let Some(t) = &s.near {
        names.tag(t).map(Src::Near).unwrap_or_else(|| {
            warnings.push(format!("{ctx}: no terrain has the tag '{t}', so nothing is near"));
            Src::Const(NEAR_CAP as i64 * Q)
        })
    } else {
        let key = s.noise.as_deref().unwrap_or_default();
        if s.hours <= 0.0 {
            return Err(format!("{ctx}: noise '{key}' needs `hours` > 0"));
        }
        let period = (s.hours * crate::TICKS_PER_DAY as f64 / 24.0).round().max(1.0) as u64;
        Src::Noise { key: mix(key.bytes().fold(0xC11A_7E00u64, |h, b| mix(h ^ b as u64))), period }
    };
    let curve =
        if s.curve.is_empty() { None } else { Some(Curve::compile(&s.curve).map_err(|e| format!("{ctx}: {e}"))?) };
    Ok(Input { src, curve })
}

/// Smooth value noise over ticks, in -Q..Q. Stateless: the same seed and
/// tick always give the same value, and nothing is drawn from the world RNG.
pub fn noise(seed: u64, tick: u64, period: u64) -> i64 {
    let k = tick / period;
    let f = ((tick % period) as i64 * Q) / period as i64;
    let at = |k: u64| (mix(seed ^ mix(k)) % (2 * Q as u64 + 1)) as i64 - Q;
    let (a, b) = (at(k), at(k + 1));
    // Smoothstep: f²(3 - 2f).
    let s = f * f / Q * (3 * Q - 2 * f) / Q;
    a + (b - a) * s / Q
}

/// Order fields so each is evaluated after the fields its terms read.
/// `reads[i]` lists what field `i` reads. Errors name a cycle.
pub fn order(reads: &[Vec<usize>], names: &[&str]) -> Result<Vec<usize>, String> {
    let n = reads.len();
    // 0 = unvisited, 1 = in progress, 2 = done.
    let mut state = vec![0u8; n];
    let mut out = Vec::with_capacity(n);
    fn visit(
        i: usize,
        reads: &[Vec<usize>],
        state: &mut [u8],
        out: &mut Vec<usize>,
        path: &mut Vec<usize>,
        names: &[&str],
    ) -> Result<(), String> {
        match state[i] {
            2 => return Ok(()),
            1 => {
                let start = path.iter().position(|&p| p == i).unwrap_or(0);
                let cycle: Vec<&str> = path[start..].iter().chain([&i]).map(|&p| names[p]).collect();
                return Err(format!("field terms form a cycle: {}", cycle.join(" -> ")));
            }
            _ => {}
        }
        state[i] = 1;
        path.push(i);
        for &d in &reads[i] {
            visit(d, reads, state, out, path, names)?;
        }
        path.pop();
        state[i] = 2;
        out.push(i);
        Ok(())
    }
    for i in 0..n {
        visit(i, reads, &mut state, &mut out, &mut Vec::new(), names)?;
    }
    Ok(out)
}

/// How far through its cycle `tick` is, 0..`Q`: integer maths, so every
/// machine agrees on a moon's phase. A period is under a million years of
/// ticks, so `into * Q` fits.
fn cycle(tick: u64, offset: u64, period: u64) -> i64 {
    let into = (tick % period + offset) % period;
    (into * Q as u64 / period) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    struct E {
        year: i64,
        hour: i64,
        amb: Vec<i64>,
        tick: u64,
    }
    impl Env for E {
        fn year(&self) -> i64 {
            self.year
        }
        fn hour(&self) -> i64 {
            self.hour
        }
        fn ambient(&self, f: usize) -> i64 {
            self.amb[f]
        }
        fn tick(&self) -> u64 {
            self.tick
        }
        fn seed(&self) -> u64 {
            1
        }
    }

    fn parse(src: &str) -> TermsDef {
        toml::from_str(src).unwrap()
    }

    fn resolve(s: &str) -> Option<usize> {
        ["cloud", "temperature"].iter().position(|x| *x == s)
    }

    #[test]
    fn curves_interpolate_and_clamp() {
        let c = Curve::compile(&[[0.0, 1.0], [10.0, 3.0], [20.0, -1.0]]).unwrap();
        assert_eq!(c.eval(to_q(-5.0)), to_q(1.0));
        assert_eq!(c.eval(to_q(5.0)), to_q(2.0));
        assert_eq!(c.eval(to_q(15.0)), to_q(1.0));
        assert_eq!(c.eval(to_q(99.0)), to_q(-1.0));
        assert!(Curve::compile(&[[1.0, 0.0], [1.0, 1.0]]).is_err());
    }

    #[test]
    fn terms_sum_products_and_explain() {
        let def = parse(
            r#"
            mean = { of = [10] }
            day = { scale = 9.0, of = [{ input = "hour", curve = [[3, -1.0], [15, 1.0], [27, -1.0]] }, { ambient = "cloud", curve = [[0, 1.0], [100, 0.5]] }] }
            "#,
        );
        let t = Terms::compile(&def, "field/temperature", &resolve, &mut vec![]).unwrap();
        let e = E { year: 0, hour: to_q(15.0), amb: vec![to_q(50.0), 0], tick: 0 };
        // 10 + 9 × 1 × 0.75
        assert_eq!(t.eval(&e), to_q(16.75));
        let ex = t.explain(&e);
        assert_eq!(ex.iter().map(|x| x.1).sum::<i64>(), t.eval(&e));
        assert_eq!(ex[0].0, "day");
        assert_eq!(t.reads(), vec![0]);
    }

    #[test]
    fn errors_name_the_term() {
        let bad = parse(r#"day = { of = [{ input = "moon" }] }"#);
        let e = Terms::compile(&bad, "field/temperature", &resolve, &mut vec![]).unwrap_err();
        assert!(e.contains("field/temperature, term 'day'") && e.contains("moon"), "{e}");
        let bad = parse(r#"x = { of = [{ ambient = "nope" }] }"#);
        assert!(Terms::compile(&bad, "f", &resolve, &mut vec![]).unwrap_err().contains("unknown field 'nope'"));
        let bad = parse(r#"x = { of = [{ ambient = "cloud", input = "hour" }] }"#);
        assert!(Terms::compile(&bad, "f", &resolve, &mut vec![]).unwrap_err().contains("exactly one"));
    }

    struct Ground;
    impl Names for Ground {
        fn field(&self, _: &str) -> Option<usize> {
            None
        }
        fn prop(&self, name: &str) -> Option<usize> {
            (name == "fertility").then_some(0)
        }
        fn tag(&self, name: &str) -> Option<usize> {
            (name == "water").then_some(0)
        }
    }

    /// Reads fertility 0.8 and water three cells away.
    struct Cell;
    impl Env for Cell {
        fn year(&self) -> i64 {
            0
        }
        fn hour(&self) -> i64 {
            0
        }
        fn ambient(&self, _: usize) -> i64 {
            0
        }
        fn tick(&self) -> u64 {
            0
        }
        fn seed(&self) -> u64 {
            0
        }
        fn terrain(&self, _: usize) -> i64 {
            to_q(0.8)
        }
        fn near(&self, _: usize) -> i64 {
            3 * Q
        }
    }

    #[test]
    fn terrain_and_near_read_the_cell_and_unknown_names_warn() {
        let def = parse(
            r#"
            soil = { scale = 2.0, of = [{ terrain = "fertility" }] }
            damp = { of = [{ near = "water", curve = [[0, 1.0], [4, 0.0]] }] }
            "#,
        );
        let mut warnings = vec![];
        let t = Terms::compile(&def, "field/growth", &Ground, &mut warnings).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        // 2 × 0.8 + (1 - 3/4)
        assert_eq!(t.eval(&Cell), to_q(1.85));
        assert_eq!(t.nears().collect::<Vec<_>>(), vec![0]);
        // Outdoors there is no cell: no terrain, and nothing near.
        let e = E { year: 0, hour: 0, amb: vec![], tick: 0 };
        assert_eq!(t.eval(&e), 0);

        let def = parse(
            r#"x = { of = [{ terrain = "salinity" }] }
            y = { of = [{ near = "lava" }] }"#,
        );
        let t = Terms::compile(&def, "field/growth", &Ground, &mut warnings).unwrap();
        assert_eq!(t.eval(&Cell), NEAR_CAP as i64 * Q);
        assert!(
            warnings.len() == 2 && warnings[0].contains("'salinity'") && warnings[1].contains("'lava'"),
            "{warnings:?}"
        );
    }

    /// Integer evaluation tracks an f64 reference closely over random curves.
    #[test]
    fn fixed_point_matches_float_reference() {
        let mut rng = crate::rng::Rng::new(5);
        let mut worst: f64 = 0.0;
        for _ in 0..2000 {
            let n = 2 + rng.below(6) as usize;
            let mut x = -50.0 + rng.float() * 10.0;
            let pts: Vec<[f64; 2]> = (0..n)
                .map(|_| {
                    x += 0.5 + rng.float() * 20.0;
                    [(x * 100.0).round() / 100.0, ((rng.float() * 60.0 - 30.0) * 100.0).round() / 100.0]
                })
                .collect();
            let c = Curve::compile(&pts).unwrap();
            let xv = pts[0][0] - 5.0 + rng.float() * (pts[n - 1][0] - pts[0][0] + 10.0);
            let xv = (xv * 100.0).round() / 100.0;
            let reference = {
                if xv <= pts[0][0] {
                    pts[0][1]
                } else if xv >= pts[n - 1][0] {
                    pts[n - 1][1]
                } else {
                    let i = pts.iter().position(|p| p[0] >= xv).unwrap();
                    let (a, b) = (pts[i - 1], pts[i]);
                    a[1] + (b[1] - a[1]) * (xv - a[0]) / (b[0] - a[0])
                }
            };
            let scale = (rng.float() * 4.0 * 100.0).round() / 100.0;
            let got = from_q(to_q(scale) * c.eval(to_q(xv)) / Q);
            worst = worst.max((got - scale * reference).abs());
        }
        assert!(worst < 0.01, "worst error {worst}");
    }

    #[test]
    fn noise_is_smooth_and_bounded() {
        let mut last = noise(9, 0, 1000);
        for t in 1..20_000 {
            let v = noise(9, t, 1000);
            assert!((-Q..=Q).contains(&v));
            assert!((v - last).abs() <= 2 * Q * 3 / 1000 + 2, "jump at {t}");
            last = v;
        }
    }

    #[test]
    fn order_respects_reads_and_finds_cycles() {
        let o = order(&[vec![1], vec![], vec![0]], &["a", "b", "c"]).unwrap();
        let pos = |x| o.iter().position(|&i| i == x).unwrap();
        assert!(pos(1) < pos(0) && pos(0) < pos(2));
        let e = order(&[vec![1], vec![0]], &["a", "b"]).unwrap_err();
        assert!(e.contains("a -> b -> a"), "{e}");
    }

    #[test]
    fn a_cycle_runs_zero_to_one_over_its_days_and_matches_floats() {
        let def = parse(
            r#"
            [moon]
            of = [{ input = "cycle", days = 29.5, offset = 3.25 }]
            [eclipse]
            of = [{ input = "cycle", days = 20, curve = [[0, 1], [0.02, 0], [0.98, 0], [1, 1]] },
                  { input = "cycle", days = 1.5, offset = -0.5 }]
            "#,
        );
        let t = Terms::compile(&def, "t", &resolve, &mut Vec::new()).unwrap();
        let day = crate::TICKS_PER_DAY as f64;
        let reference = |tick: u64, days: f64, offset: f64| (tick as f64 / day + offset).rem_euclid(days) / days;
        let mut worst: f64 = 0.0;
        for k in 0..20_000u64 {
            // Across years, and far into a long game.
            let tick = k * 7919 + if k % 2 == 0 { 0 } else { 1 << 40 };
            let e = E { year: 0, hour: 0, amb: vec![0, 0], tick };
            let got = t.explain(&e);
            let moon = from_q(got.iter().find(|(l, _)| l == "moon").unwrap().1);
            worst = worst.max((moon - reference(tick, 29.5, 3.25)).abs());
            assert!((0.0..1.0).contains(&moon), "a phase is 0 up to 1: {moon}");
        }
        assert!(worst < 0.01, "within 0.01 of the float reference: {worst}");
        // An eclipse: the product of two cycles, one through a curve.
        let at = |tick| from_q(t.explain(&E { year: 0, hour: 0, amb: vec![0, 0], tick })[0].1);
        assert!(at(0) > 0.0 && at((10.0 * day) as u64) == 0.0, "only near the long cycle's turn");
        for bad in [
            r#"of = [{ input = "cycle" }]"#,
            r#"of = [{ input = "cycle", days = 0.001 }]"#,
            r#"of = [{ input = "hour", days = 8 }]"#,
            r#"of = [{ ambient = "cloud", offset = 2 }]"#,
        ] {
            let err = Terms::compile(&parse(&format!("[x]\n{bad}")), "t", &resolve, &mut Vec::new()).unwrap_err();
            assert!(err.contains("days"), "{bad}: {err}");
        }
    }
}
