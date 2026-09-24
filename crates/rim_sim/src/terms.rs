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
//! v1 inputs are global (time of day and year, other fields' outdoor values,
//! noise, constants). Per-cell inputs arrive with stock fields.

use crate::rng::mix;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Fixed-point scale for term evaluation: 1.0 is `Q`.
pub const Q: i64 = 10_000;

/// Most points a curve may have.
pub const MAX_POINTS: usize = 16;

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
    Source(SourceDef),
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct SourceDef {
    /// `"year"` (0..1) or `"hour"` (0..24).
    pub input: Option<String>,
    /// Another field's outdoor value.
    pub ambient: Option<String>,
    /// Smooth noise in -1..1, keyed so different uses don't move together.
    pub noise: Option<String>,
    /// Noise period in game hours.
    #[serde(default)]
    pub hours: f64,
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
    Ambient(usize),
    Noise { key: u64, period: u64 },
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

/// What terms can read. Values in `Q` units.
pub trait Env {
    /// Fraction of the year, 0..Q.
    fn year(&self) -> i64;
    /// Hour of the day, 0..24·Q.
    fn hour(&self) -> i64;
    fn ambient(&self, field: usize) -> i64;
    fn tick(&self) -> u64;
    fn seed(&self) -> u64;
}

impl Terms {
    /// `resolve` maps a field id to its index. `ctx` names the def in errors.
    pub fn compile(def: &TermsDef, ctx: &str, resolve: &dyn Fn(&str) -> Option<usize>) -> Result<Terms, String> {
        let mut terms = Vec::new();
        for (label, t) in def {
            let here = format!("{ctx}, term '{label}'");
            let mut inputs = Vec::new();
            for i in &t.of {
                inputs.push(match i {
                    InputDef::Const(v) => Input { src: Src::Const(to_q(*v)), curve: None },
                    InputDef::Source(s) => compile_source(s, &here, resolve)?,
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
            .filter_map(|i| if let Src::Ambient(f) = i.src { Some(f) } else { None })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn term(t: &Term, env: &dyn Env) -> i64 {
        let mut acc = t.scale;
        for i in &t.inputs {
            let mut v = match i.src {
                Src::Year => env.year(),
                Src::Hour => env.hour(),
                Src::Ambient(f) => env.ambient(f),
                Src::Noise { key, period } => noise(env.seed() ^ key, env.tick(), period),
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

fn compile_source(s: &SourceDef, ctx: &str, resolve: &dyn Fn(&str) -> Option<usize>) -> Result<Input, String> {
    let named = [s.input.is_some(), s.ambient.is_some(), s.noise.is_some()].iter().filter(|b| **b).count();
    if named != 1 {
        return Err(format!("{ctx}: an input needs exactly one of `input`, `ambient` or `noise`"));
    }
    let src = if let Some(i) = &s.input {
        match i.as_str() {
            "year" => Src::Year,
            "hour" => Src::Hour,
            other => return Err(format!("{ctx}: unknown input '{other}' (have: year, hour)")),
        }
    } else if let Some(a) = &s.ambient {
        Src::Ambient(resolve(a).ok_or_else(|| format!("{ctx}: unknown field '{a}'"))?)
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
                return Err(format!("field ambient terms form a cycle: {}", cycle.join(" -> ")));
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
        let t = Terms::compile(&def, "field/temperature", &resolve).unwrap();
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
        let e = Terms::compile(&bad, "field/temperature", &resolve).unwrap_err();
        assert!(e.contains("field/temperature, term 'day'") && e.contains("moon"), "{e}");
        let bad = parse(r#"x = { of = [{ ambient = "nope" }] }"#);
        assert!(Terms::compile(&bad, "f", &resolve).unwrap_err().contains("unknown field 'nope'"));
        let bad = parse(r#"x = { of = [{ ambient = "cloud", input = "hour" }] }"#);
        assert!(Terms::compile(&bad, "f", &resolve).unwrap_err().contains("exactly one"));
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
}
