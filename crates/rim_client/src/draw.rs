//! Rendering. Read-only access to the simulation.

use crate::{rgb, App, Tool, TOOLBAR_H, TOPBAR_H};
use macroquad::prelude::*;
use rim_sim::defs::{Satisfier, Shape};
use rim_sim::hecs::Entity;
use rim_sim::rng::hash2_f;
use rim_sim::world::*;
use rim_sim::{IVec, TICKS_PER_DAY};

const PLAYER: Color = Color::new(0.35, 0.8, 1.0, 1.0);
const HOSTILE: Color = Color::new(1.0, 0.3, 0.25, 1.0);
const PANEL: Color = Color::new(0.07, 0.08, 0.09, 0.86);
const TEXT: Color = Color::new(0.92, 0.92, 0.9, 1.0);
const DIM: Color = Color::new(0.6, 0.62, 0.62, 1.0);

/// Interpolated position of a pawn's center, in tiles.
pub fn pawn_pos(p: &Pawn) -> (f32, f32) {
    let (x, y) = (p.pos.x as f32 + 0.5, p.pos.y as f32 + 0.5);
    match p.next {
        Some(n) => {
            let t = p.progress as f32 / p.step_ticks.max(1) as f32;
            (x + (n.x as f32 + 0.5 - x) * t, y + (n.y as f32 + 0.5 - y) * t)
        }
        None => (x, y),
    }
}

fn shade(c: Color, f: f32) -> Color {
    Color::new((c.r * f).min(1.0), (c.g * f).min(1.0), (c.b * f).min(1.0), c.a)
}

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// 0 at midday, up to ~0.55 in deep night.
fn darkness(hour: f64) -> f32 {
    let h = hour as f32;
    let d = if (7.0..19.0).contains(&h) {
        0.0
    } else if (19.0..22.0).contains(&h) {
        (h - 19.0) / 3.0
    } else if (4.0..7.0).contains(&h) {
        1.0 - (h - 4.0) / 3.0
    } else {
        1.0
    };
    d * 0.55
}

pub fn world(app: &App) {
    let w = &app.sim.world;
    let defs = &w.defs;
    let cam = &app.cam;
    let z = cam.zoom;
    clear_background(Color::from_rgba(12, 14, 16, 255));

    let (x0, y0) = cam.to_world(0.0, 0.0);
    let (x1, y1) = cam.to_world(screen_width(), screen_height());
    let (tx0, ty0) = ((x0.floor() as i32).max(0), (y0.floor() as i32).max(0));
    let (tx1, ty1) = ((x1.ceil() as i32).min(w.map.w - 1), (y1.ceil() as i32).min(w.map.h - 1));

    // Terrain, with a little per-tile variation.
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let i = (ty * w.map.w + tx) as usize;
            let base = rgb(defs.terrain[w.map.terrain[i] as usize].rgb);
            let v = 0.94 + hash2_f(tx as i64, ty as i64, 99) as f32 * 0.1;
            let (sx, sy) = cam.to_screen(tx as f32, ty as f32);
            draw_rectangle(sx, sy, z + 0.5, z + 0.5, shade(base, v));
        }
    }

    let t = get_time() as f32;
    // Items first, then fixtures on top.
    for layer in 0..2 {
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let i = (ty * w.map.w + tx) as usize;
                let Some(e) = (if layer == 0 { w.map.item[i] } else { w.map.fixture[i] }) else { continue };
                let Ok(th) = w.ecs.get::<&Thing>(e) else { continue };
                let td = defs.thing(th.def);
                let c = rgb(td.rgb);
                let (sx, sy) = cam.to_screen(tx as f32, ty as f32);
                let (cx, cy) = (sx + z / 2.0, sy + z / 2.0);
                let bp = w.ecs.get::<&Blueprint>(e).ok();
                if let Some(bp) = &bp {
                    let cost = &td.build.as_ref().unwrap().cost_r;
                    let need: u32 = cost.iter().map(|c| c.1).sum();
                    let have: u32 = bp.delivered.iter().sum();
                    let ghost = Color::new(0.45, 0.7, 1.0, 0.35);
                    draw_rectangle(sx + 1.0, sy + 1.0, z - 2.0, z - 2.0, ghost);
                    draw_rectangle_lines(sx + 1.0, sy + 1.0, z - 2.0, z - 2.0, 1.5, Color::new(0.55, 0.8, 1.0, 0.8));
                    let frac = if have < need {
                        have as f32 / need.max(1) as f32 * 0.5
                    } else {
                        0.5 + 0.5 * (1.0 - bp.work_left as f32 / td.build.as_ref().unwrap().work.max(1) as f32)
                    };
                    draw_rectangle(sx + 2.0, sy + z - 4.0, (z - 4.0) * frac, 2.5, Color::new(0.6, 0.9, 1.0, 0.9));
                    continue;
                }
                let regrowing = w.ecs.get::<&Regrow>(e).is_ok();
                match td.shape {
                    Shape::Tree => {
                        draw_circle(cx + z * 0.06, cy + z * 0.08, z * 0.44, Color::new(0.0, 0.0, 0.0, 0.25));
                        draw_circle(cx, cy, z * 0.42, c);
                        draw_circle(cx - z * 0.1, cy - z * 0.1, z * 0.22, shade(c, 1.25));
                    }
                    Shape::Bush => {
                        draw_circle(cx, cy, z * 0.32, if regrowing { shade(c, 0.75) } else { c });
                        if !regrowing {
                            for (dx, dy) in [(-0.12, -0.08), (0.1, -0.1), (0.0, 0.12), (0.14, 0.08)] {
                                draw_circle(cx + dx * z, cy + dy * z, z * 0.06, Color::from_rgba(200, 40, 70, 255));
                            }
                        }
                    }
                    Shape::Rock => {
                        let v = 0.9 + hash2_f(tx as i64, ty as i64, 7) as f32 * 0.2;
                        draw_rectangle(sx, sy, z + 0.5, z + 0.5, shade(c, v));
                        draw_rectangle(sx, sy + z * 0.8, z + 0.5, z * 0.2, shade(c, v * 0.8));
                    }
                    Shape::Wall => {
                        draw_rectangle(sx, sy, z + 0.5, z + 0.5, c);
                        draw_rectangle_lines(sx + 0.5, sy + 0.5, z - 1.0, z - 1.0, 1.5, shade(c, 0.65));
                    }
                    Shape::Door => {
                        draw_rectangle(sx + z * 0.08, sy + z * 0.08, z * 0.84, z * 0.84, c);
                        draw_rectangle(sx + z * 0.45, sy + z * 0.1, z * 0.1, z * 0.8, shade(c, 0.6));
                    }
                    Shape::Bed => {
                        draw_rectangle(sx + z * 0.12, sy + z * 0.05, z * 0.76, z * 0.9, c);
                        draw_rectangle(
                            sx + z * 0.18,
                            sy + z * 0.1,
                            z * 0.64,
                            z * 0.22,
                            Color::from_rgba(230, 225, 210, 255),
                        );
                    }
                    Shape::Fire => {
                        draw_circle(cx, cy, z * 0.38, Color::from_rgba(70, 60, 55, 255));
                        let f = 1.0 + (t * 9.0 + tx as f32).sin() * 0.08;
                        draw_circle(cx, cy, z * 0.24 * f, c);
                        draw_circle(cx, cy, z * 0.12 * f, Color::from_rgba(255, 220, 120, 255));
                    }
                    Shape::Item | Shape::Blob => {
                        draw_rectangle(sx + z * 0.2, sy + z * 0.2, z * 0.6, z * 0.6, c);
                        draw_rectangle_lines(sx + z * 0.2, sy + z * 0.2, z * 0.6, z * 0.6, 1.0, shade(c, 0.6));
                        if z >= 22.0 && th.count > 1 {
                            draw_text(th.count.to_string(), sx + z * 0.22, sy + z * 0.95, 14.0, WHITE);
                        }
                    }
                }
                if let Ok(d) = w.ecs.get::<&Designated>(e) {
                    let dc = rgb(defs.designations[d.0 as usize].rgb);
                    draw_circle(sx + z * 0.82, sy + z * 0.18, z * 0.13 + 1.0, BLACK);
                    draw_circle(sx + z * 0.82, sy + z * 0.18, z * 0.13, dc);
                }
            }
        }
    }

    // Pawns.
    for &e in &w.pawns {
        let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
        if !p.active {
            continue;
        }
        let cd = defs.creature(p.def);
        let (px, py) = pawn_pos(&p);
        if px < x0 - 1.0 || px > x1 + 1.0 || py < y0 - 1.0 || py > y1 + 1.0 {
            continue;
        }
        let (sx, sy) = cam.to_screen(px, py);
        let r = cd.size * z;
        draw_circle(sx + 1.5, sy + 2.0, r, Color::new(0.0, 0.0, 0.0, 0.3));
        draw_circle(sx, sy, r, rgb(cd.rgb));
        let ring = match p.faction {
            Faction::Player => Some(PLAYER),
            Faction::Hostile => Some(HOSTILE),
            Faction::Wild => None,
        };
        if let Some(rc) = ring {
            draw_circle_lines(sx, sy, r, 2.0, rc);
        }
        if app.selected == Some(e) {
            draw_circle_lines(sx, sy, r + 4.0, 2.0, YELLOW);
            // Remaining path.
            let mut prev = (sx, sy);
            for step in p.path.iter().rev() {
                let s = cam.to_screen(step.x as f32 + 0.5, step.y as f32 + 0.5);
                draw_line(prev.0, prev.1, s.0, s.1, 1.5, Color::new(1.0, 1.0, 0.6, 0.35));
                prev = s;
            }
        }
        if p.drafted {
            draw_rectangle(sx - r, sy - r - 6.0, 6.0, 6.0, PLAYER);
        }
        if w.ecs.get::<&Designated>(e).is_ok() {
            draw_line(sx - r - 3.0, sy, sx + r + 3.0, sy, 1.5, HOSTILE);
            draw_line(sx, sy - r - 3.0, sx, sy + r + 3.0, 1.5, HOSTILE);
        }
        if p.hp < cd.max_hp {
            let f = (p.hp.max(0) as f32 / cd.max_hp as f32).clamp(0.0, 1.0);
            draw_rectangle(sx - r, sy + r + 3.0, r * 2.0, 3.0, Color::new(0.2, 0.0, 0.0, 0.8));
            draw_rectangle(sx - r, sy + r + 3.0, r * 2.0 * f, 3.0, Color::new(1.0 - f, f, 0.1, 1.0));
        }
        if p.asleep {
            draw_text("z", sx + r * 0.6, sy - r * 0.6, 16.0 + (t * 2.0).sin() * 2.0, WHITE);
        }
        if cd.intelligent && z >= 14.0 {
            let dims = measure_text(&p.name, None, 15, 1.0);
            let ly = sy + r + 15.0;
            draw_text(&p.name, sx - dims.width / 2.0 + 1.0, ly + 1.0, 15.0, BLACK);
            draw_text(&p.name, sx - dims.width / 2.0, ly, 15.0, ring.unwrap_or(WHITE));
        }
    }

    // Hit flashes.
    for (pos, tick) in &w.hits {
        let age = w.tick.saturating_sub(*tick);
        if age < 12 {
            let (sx, sy) = cam.to_screen(pos.x as f32 + 0.5, pos.y as f32 + 0.5);
            draw_circle_lines(sx, sy, z * 0.3 + age as f32, 2.0, Color::new(1.0, 1.0, 1.0, 1.0 - age as f32 / 12.0));
        }
    }

    // Night.
    let dark = darkness(w.hour());
    if dark > 0.0 {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.02, 0.03, 0.12, dark));
    }

    // Drag rectangle preview.
    if let Some(a) = app.drag_start {
        let (mx, my) = mouse_position();
        let b = cam.tile_at(mx, my);
        let (ax, ay) = (a.x.min(b.x) as f32, a.y.min(b.y) as f32);
        let (bx, by) = (a.x.max(b.x) as f32 + 1.0, a.y.max(b.y) as f32 + 1.0);
        let (s0x, s0y) = cam.to_screen(ax, ay);
        let (s1x, s1y) = cam.to_screen(bx, by);
        let c = tool_color(app);
        draw_rectangle(s0x, s0y, s1x - s0x, s1y - s0y, alpha(c, 0.18));
        draw_rectangle_lines(s0x, s0y, s1x - s0x, s1y - s0y, 2.0, c);
        let label = format!("{}x{}", (bx - ax) as i32, (by - ay) as i32);
        draw_text(&label, s1x + 6.0, s1y, 18.0, WHITE);
    } else if app.tool != Tool::Select {
        let (mx, my) = mouse_position();
        let tp = cam.tile_at(mx, my);
        let (sx, sy) = cam.to_screen(tp.x as f32, tp.y as f32);
        draw_rectangle_lines(sx, sy, z, z, 2.0, tool_color(app));
    }
}

fn tool_color(app: &App) -> Color {
    app.buttons.iter().find(|b| b.tool == app.tool).map_or(WHITE, |b| b.color)
}

// ================================================================== HUD

pub fn hud(app: &App) {
    let w = &app.sim.world;
    top_bar(app);
    messages(app);
    toolbar(app);
    if let Some(e) = app.selected {
        pawn_panel(app, e);
    }
    hover_info(app);
    if app.show_profiler {
        profiler(app);
    }
    if w.colony_lost {
        let msg = format!("The colony is lost after {} days.", w.day());
        let d = measure_text(&msg, None, 40, 1.0);
        let (cx, cy) = (screen_width() / 2.0, screen_height() / 2.0);
        draw_rectangle(cx - d.width / 2.0 - 30.0, cy - 50.0, d.width + 60.0, 80.0, PANEL);
        draw_text(&msg, cx - d.width / 2.0, cy, 40.0, HOSTILE);
    }
}

fn top_bar(app: &App) {
    let w = &app.sim.world;
    draw_rectangle(0.0, 0.0, screen_width(), TOPBAR_H, PANEL);
    let hour = w.hour();
    let clock = format!("{:02}:{:02}", hour as u32, ((hour.fract()) * 60.0) as u32);
    let speed = if app.paused { "paused".to_string() } else { format!("{}x", app.speed) };
    let left = format!("Day {}  {}  {}   Wealth {:.0}", w.day() + 1, clock, speed, w.wealth);
    draw_text(&left, 10.0, 19.0, 20.0, if app.paused { YELLOW } else { TEXT });

    // Colonist bar, centered.
    for (e, rect) in colonist_rects(app) {
        let sel = app.selected == Some(e);
        let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if sel { Color::new(0.25, 0.3, 0.35, 1.0) } else { Color::new(0.14, 0.16, 0.18, 1.0) },
        );
        let f = p.hp.max(0) as f32 / w.defs.creature(p.def).max_hp as f32;
        draw_rectangle(rect.x, rect.y + rect.h - 3.0, rect.w * f, 3.0, if f > 0.5 { GREEN } else { ORANGE });
        draw_text(&p.name, rect.x + 6.0, rect.y + 16.0, 17.0, if p.drafted { PLAYER } else { TEXT });
    }

    let help = "Space pause  1/2/3 speed  R draft  Tab next  F3 profiler";
    let d = measure_text(help, None, 15, 1.0);
    draw_text(help, screen_width() - d.width - 10.0, 18.0, 15.0, DIM);
}

fn colonist_rects(app: &App) -> Vec<(Entity, Rect)> {
    let cols: Vec<Entity> = app.sim.world.colonists().collect();
    let mut x = 360.0;
    cols.into_iter()
        .map(|e| {
            let name = app.sim.world.ecs.get::<&Pawn>(e).map(|p| p.name.clone()).unwrap_or_default();
            let wd = measure_text(&name, None, 17, 1.0).width + 12.0;
            let r = Rect::new(x, 3.0, wd.max(60.0), TOPBAR_H - 6.0);
            x += r.w + 4.0;
            (e, r)
        })
        .collect()
}

pub fn colonist_bar_hit(app: &App, mx: f32, my: f32) -> Option<Entity> {
    colonist_rects(app).into_iter().find(|(_, r)| r.contains(vec2(mx, my))).map(|(e, _)| e)
}

fn messages(app: &App) {
    let w = &app.sim.world;
    let mut y = TOPBAR_H + 22.0;
    for m in w.messages.iter().rev().take(8) {
        let age = w.tick.saturating_sub(m.tick) as f32 / TICKS_PER_DAY as f32;
        if age > 1.0 {
            break;
        }
        let c = match m.kind {
            MsgKind::Info => TEXT,
            MsgKind::Good => Color::new(0.5, 0.95, 0.5, 1.0),
            MsgKind::Threat => Color::new(1.0, 0.45, 0.35, 1.0),
            MsgKind::Bad => Color::new(1.0, 0.7, 0.3, 1.0),
        };
        let a = (1.0 - age).clamp(0.35, 1.0);
        let d = measure_text(&m.text, None, 18, 1.0);
        draw_rectangle(8.0, y - 16.0, d.width + 12.0, 22.0, alpha(PANEL, 0.6 * a));
        draw_text(&m.text, 14.0, y, 18.0, alpha(c, a));
        y += 24.0;
    }
}

fn toolbar(app: &App) {
    let y = screen_height() - TOOLBAR_H;
    draw_rectangle(0.0, y, screen_width(), TOOLBAR_H, PANEL);
    for b in &app.buttons {
        let r = Rect::new(b.rect.x, y + 6.0, b.rect.w, b.rect.h);
        let active = app.tool == b.tool;
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            if active { alpha(b.color, 0.45) } else { Color::new(0.15, 0.16, 0.18, 1.0) },
        );
        draw_rectangle(r.x, r.y + r.h - 3.0, r.w, 3.0, b.color);
        draw_text(&b.label, r.x + 11.0, r.y + 21.0, 18.0, TEXT);
    }
}

fn bar(x: f32, y: f32, w: f32, label: &str, f: f32, c: Color) {
    draw_text(label, x, y + 12.0, 16.0, DIM);
    draw_rectangle(x + 60.0, y + 2.0, w, 11.0, Color::new(0.15, 0.15, 0.15, 1.0));
    draw_rectangle(x + 60.0, y + 2.0, w * f.clamp(0.0, 1.0), 11.0, c);
}

fn pawn_panel(app: &App, e: Entity) {
    let w = &app.sim.world;
    let Ok(p) = w.ecs.get::<&Pawn>(e) else { return };
    let cd = w.defs.creature(p.def);
    let h = 70.0 + p.needs.len() as f32 * 18.0 + if p.faction == Faction::Player { 20.0 } else { 0.0 };
    let (x, y) = (8.0, screen_height() - TOOLBAR_H - h - 8.0);
    draw_rectangle(x, y, 280.0, h, PANEL);
    let title = if p.founder { format!("{} (founder)", p.name) } else { p.name.clone() };
    draw_text(&title, x + 10.0, y + 22.0, 22.0, TEXT);
    let sub = format!("{} · {} · {}", cd.label, p.faction.name(), if p.drafted { "drafted" } else { p.job.label() });
    draw_text(&sub, x + 10.0, y + 40.0, 16.0, DIM);
    let mut yy = y + 48.0;
    bar(x + 10.0, yy, 190.0, "health", p.hp as f32 / cd.max_hp as f32, Color::new(0.4, 0.8, 0.4, 1.0));
    yy += 18.0;
    for (nid, v) in &p.needs {
        let nd = w.defs.need(*nid);
        let c = rgb(nd.rgb);
        bar(
            x + 10.0,
            yy,
            190.0,
            &nd.label,
            *v as f32 / NEED_MAX as f32,
            if nd.satisfier == Satisfier::Food && *v < 2000 { ORANGE } else { c },
        );
        yy += 18.0;
    }
    if p.faction == Faction::Player {
        let hint = if p.drafted { "R undraft · right-click move/attack" } else { "R draft" };
        draw_text(hint, x + 10.0, yy + 14.0, 15.0, DIM);
    }
}

fn hover_info(app: &App) {
    let (mx, my) = mouse_position();
    if my > screen_height() - TOOLBAR_H || my < TOPBAR_H {
        return;
    }
    let w = &app.sim.world;
    let tp: IVec = app.cam.tile_at(mx, my);
    if !w.map.inb(tp) {
        return;
    }
    let i = w.map.idx(tp);
    let mut lines = vec![format!("{} ({}, {})", w.defs.terrain[w.map.terrain[i] as usize].label, tp.x, tp.y)];
    for e in [w.map.fixture[i], w.map.item[i]].into_iter().flatten() {
        if let Ok(t) = w.ecs.get::<&Thing>(e) {
            let td = w.defs.thing(t.def);
            let mut s = td.label.clone();
            if let Ok(bp) = w.ecs.get::<&Blueprint>(e) {
                let cost = &td.build.as_ref().unwrap().cost_r;
                let parts: Vec<String> = cost
                    .iter()
                    .zip(&bp.delivered)
                    .map(|(c, d)| format!("{}/{} {}", d, c.1, w.defs.thing(c.0).label))
                    .collect();
                s = format!("{s} (blueprint: {})", parts.join(", "));
            } else if t.count > 1 {
                s = format!("{s} x{}", t.count);
            }
            if w.ecs.get::<&Regrow>(e).is_ok() {
                s.push_str(" (regrowing)");
            }
            lines.push(s);
        }
    }
    let wd = lines.iter().map(|l| measure_text(l, None, 16, 1.0).width).fold(0.0, f32::max) + 16.0;
    let h = lines.len() as f32 * 18.0 + 8.0;
    let (x, y) = (screen_width() - wd - 8.0, screen_height() - TOOLBAR_H - h - 8.0);
    draw_rectangle(x, y, wd, h, PANEL);
    for (k, l) in lines.iter().enumerate() {
        draw_text(l, x + 8.0, y + 18.0 + k as f32 * 18.0, 16.0, if k == 0 { DIM } else { TEXT });
    }
}

fn profiler(app: &App) {
    let s = &app.sim;
    let w = &s.world;
    let mut lines: Vec<(String, Color)> = Vec::new();
    lines.push(("PROFILER (smoothed µs per call)".into(), YELLOW));
    let mut entries = s.profile.entries.clone();
    entries.sort_by(|a, b| a.0.starts_with("mod:").cmp(&b.0.starts_with("mod:")).then(b.1.partial_cmp(&a.1).unwrap()));
    for (name, us) in entries {
        let c = if name.starts_with("mod:") { Color::new(0.7, 0.85, 1.0, 1.0) } else { TEXT };
        lines.push((format!("{name:<16} {us:>9.1}"), c));
    }
    lines.push((String::new(), TEXT));
    lines.push((format!("tick {}   pawns {}   entities {}", w.tick, w.pawns.len(), w.ecs.len()), TEXT));
    lines.push((format!("paths {}   nodes {}", w.pf.searches, w.pf.expanded), TEXT));
    lines.push((format!("reservations {}", w.reservations.len()), TEXT));
    let (hooks, handlers) = s.scripts.hook_count();
    lines.push((format!("script hooks {hooks}   handlers {handlers}"), TEXT));
    lines.push((String::new(), TEXT));
    lines.push(("MODS (load order)".into(), YELLOW));
    for m in &s.mods {
        lines.push((format!("{} {} ({})", m.id, m.version, m.name), TEXT));
    }
    if !s.warnings.is_empty() {
        lines.push((String::new(), TEXT));
        lines.push(("WARNINGS".into(), ORANGE));
        for wn in &s.warnings {
            lines.push((wn.clone(), ORANGE));
        }
    }
    let wd = lines.iter().map(|l| measure_text(&l.0, None, 16, 1.0).width).fold(0.0, f32::max) + 20.0;
    let h = lines.len() as f32 * 18.0 + 12.0;
    let x = screen_width() - wd - 8.0;
    let y = TOPBAR_H + 8.0;
    draw_rectangle(x, y, wd, h, PANEL);
    for (k, (l, c)) in lines.iter().enumerate() {
        draw_text(l, x + 10.0, y + 20.0 + k as f32 * 18.0, 16.0, *c);
    }
}
