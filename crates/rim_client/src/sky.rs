//! Weather on screen. Light is `light.rs`.
//!
//! The renderer reads the sim's fields and never names a weather type:
//!
//! - **Precipitation** from the `precipitation`, `temperature` and `wind`
//!   channels: rain, sleet or snow, slanted by the wind, never indoors.
//! - **Fog** from `fog`, and **lightning** in heavy precipitation with a
//!   strong wind.
//!
use crate::Cam;
use macroquad::prelude::*;
use rim_sim::world::World;
use rim_sim::IVec;

/// Most particles on screen at once.
const MAX_PARTICLES: usize = 3000;

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    /// 0 = rain, 1 = snow.
    snow: bool,
    phase: f32,
}

pub struct Sky {
    parts: Vec<Particle>,
    rng: u64,
    last: f64,
    flash: f32,
    /// Where the bolt is, as an azimuth in degrees: a new one each flash.
    flash_from: f32,
    next_flash: f64,
    /// Particles skipped last frame because they were over an enclosed room.
    pub hidden: usize,
}

impl Default for Sky {
    fn default() -> Self {
        Sky { parts: Vec::new(), rng: 0x5EED, last: 0.0, flash: 0.0, flash_from: 0.0, next_flash: 0.0, hidden: 0 }
    }
}

/// A lightning flash: how bright, 0 to about 1, and where the bolt is, as
/// an azimuth in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flash {
    pub strength: f32,
    pub azimuth: f32,
}

/// What the renderer needs from the world's outdoor values.
pub struct Air {
    pub light: f32,
    pub precipitation: f32,
    pub temperature: f32,
    pub wind: f32,
    pub wind_dir: f32,
    pub fog: f32,
    pub cloud: f32,
}

impl Air {
    pub fn read(w: &World) -> Air {
        let a = |id: &str| w.defs.lookup("field", id).map_or(0.0, |f| w.fields.ambient(f as usize) as f32);
        Air {
            light: a("light"),
            precipitation: a("precipitation"),
            temperature: a("temperature"),
            wind: a("wind"),
            wind_dir: a("wind_dir"),
            fog: a("fog"),
            cloud: a("cloud"),
        }
    }

    /// Screen-space wind vector, m/s.
    fn wind_vec(&self) -> (f32, f32) {
        let r = self.wind_dir.to_radians();
        (r.cos() * self.wind, r.sin() * self.wind)
    }
}

impl Sky {
    /// The lightning flash now, if any: `strength` 0 when there's none.
    pub fn flash(&self) -> Flash {
        Flash { strength: self.flash, azimuth: self.flash_from }
    }

    fn rand(&mut self) -> f32 {
        self.rng = rim_sim::rng::mix(self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15));
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Precipitation, fog and lightning. Call before `Light::multiply`, so
    /// the weather is lit (and darkened) like the world.
    pub fn weather(&mut self, w: &World, cam: &Cam, air: &Air) {
        let now = get_time();
        let dt = ((now - self.last) as f32).clamp(0.0, 0.1);
        self.last = now;
        let (sw, sh) = (screen_width(), screen_height());

        // Fog: a flat veil plus a few slow, soft banks drifting with the wind.
        if air.fog > 1.0 {
            let f = (air.fog / 100.0).clamp(0.0, 1.0);
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.78, 0.8, 0.84, f * 0.35));
            let (wx, wy) = air.wind_vec();
            for k in 0..6 {
                let k = k as f32;
                let x = ((k * 397.0 + now as f32 * (8.0 + wx * 4.0)) % (sw + 600.0)) - 300.0;
                let y = ((k * 211.0 + now as f32 * (3.0 + wy * 4.0)) % (sh + 400.0)) - 200.0;
                draw_circle(x, y, 220.0 + k * 30.0, Color::new(0.82, 0.84, 0.88, f * 0.12));
            }
        }

        // Precipitation: density follows the channel; below freezing it's
        // snow, around freezing a mix.
        let target = ((air.precipitation / 8.0).clamp(0.0, 1.0) * MAX_PARTICLES as f32 * (sw * sh) / (1600.0 * 960.0))
            .min(MAX_PARTICLES as f32) as usize;
        let snow_share = ((1.0 - air.temperature) / 2.0).clamp(0.0, 1.0);
        let (wx, wy) = air.wind_vec();
        while self.parts.len() < target {
            let snow = self.rand() < snow_share;
            let (x, y) = (self.rand() * sw, self.rand() * sh);
            let p = self.spawn(snow, x, y, wx, wy);
            self.parts.push(p);
        }
        self.parts.truncate(target.max(self.parts.len().min(target + 50)));
        let mut i = 0;
        self.hidden = 0;
        while i < self.parts.len() {
            let mut p = self.parts[i];
            p.phase += dt;
            // Drift toward the current mix of rain and snow.
            if self.rand() < 0.03 {
                let snow = self.rand() < snow_share;
                if snow != p.snow {
                    p = self.spawn(snow, p.x, p.y, wx, wy);
                }
            }
            let flutter = if p.snow { (p.phase * 2.3).sin() * 18.0 } else { 0.0 };
            p.x += (p.vx + flutter) * dt;
            p.y += p.vy * dt;
            if p.x < -40.0 || p.x > sw + 40.0 || p.y > sh + 20.0 || p.y < -60.0 {
                if self.parts.len() > target {
                    self.parts.swap_remove(i);
                    continue;
                }
                // Re-enter from the top, or from the side the wind comes from.
                let snow = self.rand() < snow_share;
                let from_side = self.rand() < (wx.abs() / (wx.abs() + 8.0));
                let (x, y) = if from_side {
                    (if wx > 0.0 { -20.0 } else { sw + 20.0 }, self.rand() * sh)
                } else {
                    (self.rand() * sw, -20.0)
                };
                p = self.spawn(snow, x, y, wx, wy);
            }
            self.parts[i] = p;
            i += 1;
            // Nothing falls in an enclosed room.
            let (tx, ty) = cam.to_world(p.x, p.y);
            if w.map.indoors(IVec::new(tx.floor() as i32, ty.floor() as i32)) {
                self.hidden += 1;
                continue;
            }
            if p.snow {
                draw_circle(p.x, p.y, 1.6, Color::new(0.95, 0.97, 1.0, 0.85));
            } else {
                let len = 0.018;
                draw_line(p.x, p.y, p.x - p.vx * len, p.y - p.vy * len, 1.0, Color::new(0.7, 0.8, 0.95, 0.4));
            }
        }

        // Lightning: storms (heavy precipitation and a strong wind).
        self.flash = (self.flash - dt * 4.0).max(0.0);
        if air.precipitation > 4.0 && air.wind > 10.0 {
            if now >= self.next_flash {
                if self.next_flash > 0.0 {
                    self.flash = 0.9;
                    self.flash_from = self.rand() * 360.0;
                }
                self.next_flash = now + 3.0 + self.rand() as f64 * 9.0;
            }
        } else {
            self.next_flash = 0.0;
        }
    }

    fn spawn(&mut self, snow: bool, x: f32, y: f32, wx: f32, wy: f32) -> Particle {
        let (vx, vy) = if snow {
            (wx * 9.0, 55.0 + self.rand() * 30.0 + wy * 6.0)
        } else {
            (wx * 30.0, 820.0 + self.rand() * 240.0 + wy * 20.0)
        };
        Particle { x, y, vx, vy, snow, phase: self.rand() * 6.0 }
    }

    /// A lightning flash now (tools and the autotest; storms flash on their own).
    pub fn strike(&mut self) {
        self.flash = 0.9;
        self.flash_from = self.rand() * 360.0;
        self.next_flash = get_time() + 3.0;
    }

    pub fn particles(&self) -> usize {
        self.parts.len()
    }
}
