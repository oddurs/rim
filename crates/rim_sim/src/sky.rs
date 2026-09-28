//! Where the sky's bodies are: a pure function of the tick, the calendar
//! and the `[[sky_body]]` defs (DESIGN.md §6e). Nothing here is saved.
//!
//! Each body turns about the world's axis once a `day_period`, highest at
//! `transit`; its declination swings by `tilt` over the year, furthest north
//! at the calendar's `midsummer`; and a body with `phase_days` waxes and
//! wanes. From those and the world's latitude come its altitude, azimuth,
//! how far it is up (with a short twilight), and how much of it is lit.
//! `Fields` works them out with the outdoor values, every
//! `AMBIENT_INTERVAL` ticks (the sun moves 0.36° in that time), and keeps
//! them for terms and the renderer.
//!
//! The maths is in-crate: the platform's libm differs from machine to
//! machine in the last bits, and lockstep can't. `sin`, `cos` and `atan2`
//! here use only + − × ÷, which IEEE 754 rounds the same everywhere, and
//! `f64::sqrt`, which it requires to be correctly rounded (a
//! `tests/no_trig.rs` guard keeps std's trig out of the crate).

use crate::defs::{CalendarDef, DefDb, SkyBodyDef};
use crate::TICKS_PER_DAY;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// Degrees below and above the horizon over which a body comes up: `up`
/// is 0 at −`TWILIGHT` and 1 at +`TWILIGHT`.
pub const TWILIGHT: f64 = 6.0;

/// Where a body is, and how much of it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyState {
    /// Degrees above the horizon, −90..90.
    pub altitude: f64,
    /// Degrees round the horizon in map terms, 0..360: 0 east, 90 south
    /// (the map's y grows south), 180 west, 270 north.
    pub azimuth: f64,
    /// How far it is up, 0..1: 0 below the twilight band, 1 above it.
    pub up: f64,
    /// The lit fraction, 0..1: 1 for a body without phases.
    pub phase: f64,
}

/// Every body's state at `tick`, in `defs.sky_bodies` order.
pub fn states(defs: &DefDb, tick: u64) -> Vec<BodyState> {
    defs.sky_bodies.iter().map(|b| state(b, &defs.calendar, tick)).collect()
}

/// One body's state at `tick`.
pub fn state(b: &SkyBodyDef, cal: &CalendarDef, tick: u64) -> BodyState {
    // Tick 0 is 06:00, so the day's turn counts from `transit` hours after
    // the first midnight. Whole ticks and a remainder: exact in a long game.
    let since = tick as i64 + (TICKS_PER_DAY / 4) as i64 - b.transit_ticks;
    let turn = since.rem_euclid(b.period_ticks) as f64 / b.period_ticks as f64;
    let year_len = cal.year_days as u64 * TICKS_PER_DAY;
    let year = ((cal.start_day as u64 * TICKS_PER_DAY + tick) % year_len) as f64 / year_len as f64;

    let h = TAU * turn;
    let dec = radians(b.tilt) * cos(TAU * (year - cal.midsummer));
    let lat = radians(cal.latitude);
    let (sh, ch, sd, cd, sl, cl) = (sin(h), cos(h), sin(dec), cos(dec), sin(lat), cos(lat));
    // East, north and up, with the hour angle growing westward.
    let east = -cd * sh;
    let north = sd * cl - cd * ch * sl;
    let upward = sd * sl + cd * ch * cl;
    let altitude = degrees(atan2(upward, (east * east + north * north).sqrt()));
    let mut azimuth = degrees(atan2(-north, east));
    if azimuth < 0.0 {
        azimuth += 360.0;
    }
    let x = ((altitude + TWILIGHT) / (2.0 * TWILIGHT)).clamp(0.0, 1.0);
    let phase = match b.phase_ticks {
        Some(period) => {
            let into = (tick % period + b.phase_offset_ticks) % period;
            (1.0 - cos(TAU * into as f64 / period as f64)) / 2.0
        }
        None => 1.0,
    };
    BodyState { altitude, azimuth, up: x * x * (3.0 - 2.0 * x), phase }
}

fn radians(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

fn degrees(rad: f64) -> f64 {
    rad * (180.0 / PI)
}

/// Sine, from its Taylor series on −π/2..π/2: within 1e-15 of std there.
/// Arguments are angles of a turn or two, so the reduction is by subtraction.
pub fn sin(x: f64) -> f64 {
    let mut x = x;
    while x > PI {
        x -= TAU;
    }
    while x < -PI {
        x += TAU;
    }
    if x > FRAC_PI_2 {
        x = PI - x;
    } else if x < -FRAC_PI_2 {
        x = -PI - x;
    }
    // x(1 − x²/(2·3)(1 − x²/(4·5)(1 − …))), through x¹⁹/19!.
    let x2 = x * x;
    let mut acc = 1.0;
    for d in [342.0, 272.0, 210.0, 156.0, 110.0, 72.0, 42.0, 20.0, 6.0] {
        acc = 1.0 - x2 / d * acc;
    }
    x * acc
}

pub fn cos(x: f64) -> f64 {
    sin(FRAC_PI_2 - x)
}

/// The angle of (x, y), −π..π, as std's `atan2`.
pub fn atan2(y: f64, x: f64) -> f64 {
    if x > 0.0 {
        atan(y / x)
    } else if x < 0.0 {
        if y >= 0.0 {
            atan(y / x) + PI
        } else {
            atan(y / x) - PI
        }
    } else if y > 0.0 {
        FRAC_PI_2
    } else if y < 0.0 {
        -FRAC_PI_2
    } else {
        0.0
    }
}

/// Arctangent: folded to 0..tan(π/8), where 21 terms of its series are
/// within 1e-16.
pub fn atan(x: f64) -> f64 {
    let a = x.abs();
    let (base, y) = if a > 1.0 { (FRAC_PI_2, -1.0 / a) } else { (0.0, a) };
    // tan(π/8): past it, atan(y) = π/4 + atan((y − 1)/(y + 1)).
    let (base, y) = if y.abs() > 0.414_213_562_373_095_1 {
        let s = y.signum();
        (base + s * FRAC_PI_4, (y - s) / (1.0 + s * y))
    } else {
        (base, y)
    };
    let y2 = y * y;
    let mut acc = 1.0 / 41.0;
    for k in (0..20).rev() {
        acc = 1.0 / (2 * k + 1) as f64 - y2 * acc;
    }
    let r = base + y * acc;
    if x < 0.0 {
        -r
    } else {
        r
    }
}
