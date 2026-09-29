//! World overlays (DESIGN.md §6f): what is picked, drawn in one language.
//!
//! `scene` turns the app's state into a list of marks in screen points,
//! and `draw` paints them over the lit world. The scene is plain data so
//! the autotest can check what the player would see without reading
//! pixels. Colours and sizes are theme tokens, so a mod restyles the
//! overlays the way it restyles a panel.

use crate::{draw, App};
use macroquad::prelude::*;
use rim_sim::command::{Blocker, Place, Target};
use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::Pawn;
use rim_sim::IVec;
use rim_ui::paint::Draw;
use rim_ui::theme::{Rgba, Theme};

/// Brackets and rings close in over this long, from `SELECT_FROM` points
/// further out than they settle.
const SELECT_IN_SECS: f64 = 0.12;
const SELECT_FROM: f32 = 4.0;
/// Hover fades in quickly and out a little slower, so a sweep across
/// things leaves a short trail rather than a flicker.
const HOVER_IN_SECS: f64 = 0.06;
const HOVER_OUT_SECS: f64 = 0.14;
/// Hover's edge, over the chalk's own strength.
const HOVER_ALPHA: f32 = 0.72;
/// An order's ring: from a third of a cell out to three quarters.
const ACK_SECS: f64 = 0.24;
/// An urgent mark's ring breathes out once in this long.
const BREATHE_SECS: f64 = 1.8;
/// A refused click shakes for this long, and says why for this long.
const SHAKE_SECS: f64 = 0.18;
const REFUSED_SECS: f64 = 1.4;
/// A cell with nothing for the tool in hand: a faint frame.
const FAINT_FRAME: f32 = 0.38;
/// A new target's dot, before the order makes it real.
const PREVIEW_DOT: f32 = 0.6;
/// Everything in a group but the one the inspector shows.
const GROUP_ALPHA: f32 = 0.7;
/// A colonist an Alt-drag will take out of the selection.
const LEAVING_ALPHA: f32 = 0.3;
/// A colonist a select drag will pick, before the button comes up.
const PICKING_ALPHA: f32 = 0.6;
/// Measuring's numbers are at least this far apart, in points.
const LABEL_SPACING: f32 = 36.0;
/// A select drag's box: its fill, over the chalk's own strength.
const MARQUEE_FILL: f32 = 0.05;
/// A path's dots, this many points apart.
const DOT_STEP: f32 = 5.0;

/// Overlay colours and sizes, read from the theme. Sizes are points: the
/// world is drawn in points, and the UI scale is for panels, not the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub chalk: Color,
    pub keyline: Color,
    /// Allowed but costly, and urgent marks: the theme's `bad`.
    pub caution: Color,
    /// Can't: the theme's `threat`.
    pub threat: Color,
    /// The player's plans: the theme's `accent`, and a ghost's wash.
    pub intent: Color,
    pub intent_fill: Color,
    /// Line weights: hover's, a selected or hovered stockpile's and a
    /// store's outline (`stroke`); a stockpile's edge and a drag box's
    /// (`hair`).
    pub stroke: f32,
    pub hair: f32,
    /// Stockpiles: their edge, and their wash at rest (deeper when
    /// selected or being added).
    pub zone: Color,
    pub zone_fill: Color,
    /// A grid line (`grid`), and measuring's every fifth.
    pub seam: Color,
    pub seam_major: Color,
    pub firm: f32,
    pub bracket_gap: f32,
    pub bracket_arm_min: f32,
    pub bracket_arm_max: f32,
    /// A chip is a small panel: the panels' ground, edge, text, corner
    /// and caption size.
    pub surface: Color,
    pub line: Color,
    pub text: Color,
    pub radius: f32,
    pub caption: f32,
    pub leading: f32,
}

impl Palette {
    /// The theme's tokens, falling back to core's values for any a theme
    /// leaves out, so a mod that replaces the theme still gets overlays.
    pub fn from_theme(t: &Theme) -> Palette {
        let c = |k: &str, hex: &str| {
            let v: Rgba = t.color.get(k).copied().or_else(|| rim_ui::theme::parse_color(hex)).unwrap_or([1.0; 4]);
            Color::new(v[0], v[1], v[2], v[3])
        };
        let shape = |k: &str, v: f32| t.shape.get(k).copied().unwrap_or(v);
        Palette {
            chalk: c("chalk", "#f2eee3"),
            keyline: c("keyline", "#080a0c8c"),
            caution: c("bad", "#ffb35a"),
            threat: c("threat", "#ff6b5a"),
            intent: c("accent", "#5ab4ff"),
            intent_fill: c("intent_fill", "#5ab4ff4d"),
            seam: c("seam", "#0000001f"),
            seam_major: c("seam_major", "#00000052"),
            stroke: shape("stroke", 1.5),
            hair: shape("hair", 1.0),
            zone: c("zone", "#a48fe0"),
            zone_fill: c("zone_fill", "#a48fe01f"),
            firm: shape("firm", 2.0),
            bracket_gap: shape("bracket_gap", 3.0),
            bracket_arm_min: shape("bracket_arm_min", 4.0),
            bracket_arm_max: shape("bracket_arm_max", 12.0),
            surface: c("surface_raised", "#1c1f24f2"),
            line: c("line_strong", "#ffffff40"),
            text: c("text", "#e7e7e4"),
            radius: shape("radius", 4.0),
            caption: t.text.get("caption").copied().unwrap_or(11.0),
            leading: t.leading(false),
        }
    }

    /// A bracket's arm: 28% of the footprint's short side, clamped.
    pub fn bracket_arm(&self, w: f32, h: f32) -> f32 {
        (w.min(h) * 0.28).clamp(self.bracket_arm_min, self.bracket_arm_max)
    }
}

/// One thing the overlay draws, in screen points.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// Selection on a thing: corner brackets `gap` outside its footprint.
    Brackets {
        rect: [f32; 4],
        gap: f32,
        alpha: f32,
    },
    /// Selection on a pawn: a ring `gap` outside its body.
    Ring {
        center: (f32, f32),
        r: f32,
        gap: f32,
        alpha: f32,
    },
    /// Hover on a thing: an edge on its footprint.
    Hover {
        rect: [f32; 4],
        alpha: f32,
    },
    /// Hover on a pawn: a ring on the edge of its body.
    HoverRing {
        center: (f32, f32),
        r: f32,
        alpha: f32,
    },
    /// What's left of a selected pawn's path, dotted.
    Path {
        points: Vec<(f32, f32)>,
        alpha: f32,
    },
    /// Where an order just landed: one chalk ring, growing and fading.
    Ack {
        center: (f32, f32),
        r: f32,
        alpha: f32,
    },
    /// An urgent mark's ring, breathing out from it.
    Breathe {
        center: (f32, f32),
        r: f32,
        alpha: f32,
    },
    /// A selected thing off screen: a chevron at the edge pointing at it,
    /// `angle` in radians from the screen's centre.
    Offscreen {
        at: (f32, f32),
        angle: f32,
    },
    /// A measuring line's cell number, where it crosses the pointer's
    /// row or column.
    Label {
        at: (f32, f32),
        text: String,
        alpha: f32,
    },
    /// A designate tool on a target: an edge in the designation's hue.
    Aimed {
        aim: Aim,
        color: Color,
    },
    /// What an order drag will newly mark (a ring in its hue and the dot
    /// to come) or, for Cancel, take back (dimmed).
    Target {
        aim: Aim,
        color: Color,
        cancel: bool,
    },
    /// What a build will put up: its footprint, whether a natural thing
    /// is cleared first, and which way it faces if it's bigger than a cell.
    Ghost {
        rect: [f32; 4],
        clears: bool,
        facing: Option<u8>,
    },
    /// A build that can't go up: grey over its footprint, a cross on the
    /// cell that blocks it.
    Blocked {
        rect: [f32; 4],
        cell: [f32; 4],
    },
    /// A cell's frame: faint where the tool in hand has nothing to do,
    /// shaking where a click was refused, or in the zone colour under a
    /// zone tool. Chalk when `color` is none.
    Frame {
        rect: [f32; 4],
        alpha: f32,
        color: Option<Color>,
    },
    /// A select drag's box, snapped to cells: dashed when it takes
    /// colonists out.
    Marquee {
        rect: [f32; 4],
        dashed: bool,
        /// Chalk when none; an order's drag takes its hue.
        color: Option<Color>,
    },
    /// A selected stack's way to where it will be stored, dashed.
    Haul {
        from: (f32, f32),
        to: (f32, f32),
    },
    /// A job no colonist can reach: a triangle `size` points on a side in
    /// the bottom-left corner at `at`.
    Notch {
        at: (f32, f32),
        size: f32,
    },
    Chip(Chip),
}

/// A notch's side, against the cell.
const NOTCH_CELL: f32 = 0.3;

/// A small label by the pointer or a mark: "3 selected", "Chop · 4 trees".
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    /// Its top-left corner, or its top-right when `right`; it's kept on
    /// screen when drawn.
    pub at: (f32, f32),
    pub text: String,
    pub right: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub marks: Vec<Mark>,
}

/// What a click with the select tool would pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hovered {
    Thing(Entity),
    /// A stockpile, by its id.
    Zone(u32),
}

/// What the overlays remember between frames.
#[derive(Default)]
pub struct State {
    /// The frame's clock, for anything that moves.
    now: f64,
    /// When each selected thing was selected, to close its brackets in.
    since: Vec<(Entity, f64)>,
    /// Everything hovered lately, with how strongly it shows: the one
    /// under the pointer fading in, the rest fading out from wherever
    /// they had got to.
    hovers: Vec<(Hovered, f32)>,
    /// The player's reduce-motion setting, as of the last update: hover
    /// and selection are simply there or not.
    pub instant: bool,
}

impl State {
    /// Keep `since` in step with the selection, and follow the hover.
    pub fn update(&mut self, selected: &[Entity], hovered: Option<Hovered>, now: f64, instant: bool) {
        self.instant = instant;
        let dt = (now - self.now).clamp(0.0, 0.1);
        self.now = now;
        if let Some(h) = hovered.filter(|h| !self.hovers.iter().any(|(s, _)| s == h)) {
            self.hovers.push((h, 0.0));
        }
        let instant = self.instant;
        for (h, a) in &mut self.hovers {
            *a = if instant {
                if Some(*h) == hovered {
                    1.0
                } else {
                    0.0
                }
            } else if Some(*h) == hovered {
                (*a + (dt / HOVER_IN_SECS) as f32).min(1.0)
            } else {
                (*a - (dt / HOVER_OUT_SECS) as f32).max(0.0)
            };
        }
        self.hovers.retain(|&(h, a)| a > 0.0 || Some(h) == hovered);
        self.since.retain(|(e, _)| selected.contains(e));
        for &e in selected {
            if !self.since.iter().any(|(s, _)| *s == e) {
                self.since.push((e, now));
            }
        }
    }

    /// The frame's clock.
    pub fn now(&self) -> f64 {
        self.now
    }

    fn since(&self, e: Entity) -> Option<f64> {
        self.since.iter().find(|(s, _)| *s == e).map(|&(_, t)| t)
    }

    /// Everything hovered lately, each with how strongly it shows.
    pub fn hovers(&self) -> impl Iterator<Item = (Hovered, f32)> + '_ {
        self.hovers.iter().copied().filter(|&(_, a)| a > 0.0)
    }

    /// How strongly stockpile `id` shows as hovered, 0 to 1.
    pub fn zone_hover(&self, id: u32) -> f32 {
        self.hovers().find(|&(h, _)| h == Hovered::Zone(id)).map_or(0.0, |(_, a)| a)
    }
}

/// How far out a selection mark sits, `t` seconds after it was made:
/// from `SELECT_FROM` points beyond `gap` in to `gap`, easing out.
fn closing(gap: f32, t: f64) -> f32 {
    let k = (t / SELECT_IN_SECS).clamp(0.0, 1.0) as f32;
    let ease = 1.0 - (1.0 - k).powi(3);
    gap + SELECT_FROM * (1.0 - ease)
}

/// A thing's footprint on screen, turned as it stands.
fn footprint(app: &App, t: &rim_sim::world::Thing) -> [f32; 4] {
    let [fw, fh] = app.sim.world.defs.thing(t.def).size_facing(t.facing);
    let (sx, sy) = app.cam.to_screen(t.pos.x as f32, t.pos.y as f32);
    let z = app.cam.zoom;
    [sx, sy, z * fw as f32, z * fh as f32]
}

/// What the pointer's hint says during a drag: what the drag will do,
/// counted, else its size. The UI draws it, so there's one chip by the
/// pointer and a theme styles it.
pub fn drag_hint(app: &App) -> Option<String> {
    if let Some((text, _, t0)) = &app.refused {
        if app.chalk.now() - t0 < REFUSED_SECS {
            return Some(text.clone());
        }
    }
    if let Some(op) = &app.order_preview {
        return op.hint(app);
    }
    if let Some(bp) = &app.build_preview {
        return bp.hint();
    }
    if let Some(a) = app.drag_start.filter(|_| crate::is_box(app, app.pointer)) {
        let b = app.cam.tile_at(app.pointer.0, app.pointer.1);
        let boxed = crate::boxed_colonists(app, a, b);
        let picked = crate::selection(app);
        let n = |k: usize| format!("{k} {}", if k == 1 { "colonist" } else { "colonists" });
        return Some(if app.subtract {
            format!("−{}", n(boxed.iter().filter(|e| picked.contains(e)).count()))
        } else if app.shift {
            format!("+{}", n(boxed.iter().filter(|e| !picked.contains(e)).count()))
        } else {
            format!("{} · {}", size(a, b), n(boxed.len()))
        });
    }
    if let Some(zp) = &app.zone_preview {
        let n = zp.cells.len();
        return Some(match zp.joins {
            Some(_) => format!("Stockpile · +{n}"),
            None => format!("Clear · {n} {}", if n == 1 { "cell" } else { "cells" }),
        });
    }
    let a = app.drag_start.filter(|_| app.tool != crate::Tool::Select)?;
    Some(size(a, app.cam.tile_at(app.pointer.0, app.pointer.1)))
}

/// Where an order's target is on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    Rect([f32; 4]),
    Disc((f32, f32), f32),
}

fn aim_of(app: &App, t: &Target) -> Option<Aim> {
    let z = app.cam.zoom;
    match *t {
        Target::Thing(e) => Some(Aim::Rect(footprint(app, &app.sim.world.thing(e)?))),
        Target::Rock(p) => {
            let (x, y) = app.cam.to_screen(p.x as f32, p.y as f32);
            Some(Aim::Rect([x, y, z, z]))
        }
        Target::Creature(e) => draw::pawn_disc(app, e).map(|(c, r)| Aim::Disc(c, r)),
    }
}

/// What a designate or cancel tool would do at the pointer, or across its
/// drag, from the sim's own previews (DESIGN.md §6f). Made once a frame.
pub struct OrderPreview {
    /// The designation, or none for Cancel.
    pub designation: Option<DefId>,
    pub dragging: bool,
    /// The drag's first and last cells (the pointer's cell twice when not
    /// dragging).
    pub a: IVec,
    pub b: IVec,
    /// What it would newly mark, or for Cancel take back.
    pub targets: Vec<Target>,
    /// Hovering one cell: what's there is marked this way already.
    pub marked: bool,
}

/// The rectangle a designation's order covers: a click for a creature
/// takes a generous box, so moving targets are caught.
pub fn designate_box(app: &App, d: DefId, a: IVec, b: IVec) -> (IVec, IVec) {
    if app.sim.world.defs.designations[d as usize].targets == rim_sim::defs::Targets::Creature && a == b {
        (a.offset(-1, -1), b.offset(1, 1))
    } else {
        (a, b)
    }
}

impl OrderPreview {
    /// The tool in hand's preview at the pointer, or across its drag.
    pub fn of(app: &App) -> Option<OrderPreview> {
        let designation = match app.tool {
            crate::Tool::Designate(d) => Some(d),
            crate::Tool::Cancel => None,
            _ => return None,
        };
        if app.drag_start.is_none() && app.hover_cell.is_none() {
            return None;
        }
        let cell = app.cam.tile_at(app.pointer.0, app.pointer.1);
        let a = app.drag_start.unwrap_or(cell);
        Some(OrderPreview::between(app, designation, a, cell, app.drag_start.is_some()))
    }

    /// What `designation`, or Cancel for none, would do from `a` to `b`.
    pub fn between(app: &App, designation: Option<DefId>, a: IVec, b: IVec, dragging: bool) -> OrderPreview {
        let w = &app.sim.world;
        let targets = match designation {
            Some(d) => {
                let (a, b) = designate_box(app, d, a, b);
                rim_sim::command::designate_preview(w, d, a, b)
            }
            None => rim_sim::command::cancel_preview(w, a, b)
                .into_iter()
                .map(|e| if w.ecs.get::<&Pawn>(e).is_ok() { Target::Creature(e) } else { Target::Thing(e) })
                .collect(),
        };
        let marked = designation.is_some_and(|d| {
            let has = |e: Entity| w.ecs.get::<&rim_sim::world::Designated>(e).is_ok_and(|m| m.0 == d);
            if !targets.is_empty() {
                return false;
            }
            if w.defs.designations[d as usize].targets == rim_sim::defs::Targets::Creature {
                // A creature's click takes a box: anything marked in it.
                let (lo, hi) = designate_box(app, d, a, b);
                let inside = |p: IVec| {
                    p.x >= lo.x.min(hi.x) && p.x <= lo.x.max(hi.x) && p.y >= lo.y.min(hi.y) && p.y <= lo.y.max(hi.y)
                };
                return w.pawns.iter().any(|&e| w.pawn_pos(e).is_some_and(inside) && has(e));
            }
            [w.map.fixture_at(b), w.map.floor_at(b)].into_iter().flatten().any(has)
        });
        OrderPreview { designation, dragging, a, b, targets, marked }
    }

    /// What its targets are called: "oak tree", "oak trees", "things".
    fn noun(&self, app: &App) -> String {
        let w = &app.sim.world;
        let name = |t: &Target| match *t {
            Target::Thing(e) => w.thing(e).map(|th| w.defs.thing(th.def).label.clone()),
            Target::Rock(_) => Some("rock".to_string()),
            Target::Creature(e) => w.ecs.get::<&Pawn>(e).ok().map(|p| w.defs.creature(p.def).label.clone()),
        };
        let names: Vec<String> = self.targets.iter().filter_map(name).collect();
        match (names.first(), names.iter().all(|n| Some(n) == names.first())) {
            (Some(n), true) => plural(n, names.len()),
            _ => plural("thing", names.len()),
        }
    }

    /// The pointer's hint: "Chop · oak tree", "Chop · 4 oak trees",
    /// "Cancel · 3 walls", "Already marked"; nothing over a cell with
    /// nothing to do.
    pub fn hint(&self, app: &App) -> Option<String> {
        let verb = match self.designation {
            Some(d) => app.sim.world.defs.designations[d as usize].label.clone(),
            None => "Cancel".to_string(),
        };
        let n = self.targets.len();
        if self.marked && !self.dragging {
            return Some("Already marked".into());
        }
        Some(match (n, self.dragging) {
            (0, false) => return None,
            (0, true) if self.designation.is_some() => format!("{verb} · nothing new"),
            (0, true) => format!("{verb} · nothing"),
            (1, false) => format!("{verb} · {}", self.noun(app)),
            _ => format!("{verb} · {n} {}", self.noun(app)),
        })
    }

    /// Why a click here did nothing.
    pub fn refusal(&self, app: &App) -> String {
        match self.designation {
            _ if self.marked => "Already marked".into(),
            Some(d) => format!("Nothing to {} here", app.sim.world.defs.designations[d as usize].label.to_lowercase()),
            None => "Nothing to cancel here".into(),
        }
    }
}

/// What a build tool would put up at the pointer, or across its drag, cell
/// by cell from the sim's own `build_preview` (DESIGN.md §6f). Made once a
/// frame.
pub struct BuildPreview {
    pub thing: DefId,
    pub dragging: bool,
    pub cells: Vec<(IVec, Place)>,
    pub facing: u8,
    /// What it was worked out for: from, to, facing, material and the
    /// world's tick. The same question isn't asked twice.
    key: (IVec, IVec, u8, Option<DefId>, u64),
    /// Its hint, and why a click would do nothing, worked out once.
    hint: Option<String>,
    refusal: String,
}

impl BuildPreview {
    /// The build tool's preview at the pointer or across its drag; `last`
    /// is kept when nothing it depends on changed.
    pub fn of(app: &App, last: Option<BuildPreview>) -> Option<BuildPreview> {
        let crate::Tool::Build(thing) = app.tool else { return None };
        if app.drag_start.is_none() && app.hover_cell.is_none() {
            return None;
        }
        let cell = app.cam.tile_at(app.pointer.0, app.pointer.1);
        let a = app.drag_start.unwrap_or(cell);
        let key = (a, cell, app.build_facing, crate::chosen_material(app, thing), app.sim.world.tick);
        if let Some(bp) =
            last.filter(|bp| bp.key == key && bp.thing == thing && bp.dragging == app.drag_start.is_some())
        {
            return Some(bp);
        }
        Some(BuildPreview::between(app, thing, a, cell, app.drag_start.is_some()))
    }

    /// What building `thing` from `a` to `b` would put up, as the sim does
    /// it: rectangle by rectangle, a later one finding the earlier ones'
    /// plans in its way.
    pub fn between(app: &App, thing: DefId, a: IVec, b: IVec, dragging: bool) -> BuildPreview {
        let w = &app.sim.world;
        let td = w.defs.thing(thing);
        let facing = app.build_facing;
        let stuff = crate::chosen_material(app, thing);
        let foot = |p: IVec| -> Vec<IVec> { td.footprint(p, facing).collect() };
        let mut claimed: Vec<IVec> = Vec::new();
        let mut cells = Vec::new();
        for (ra, rb) in crate::build_rects(td.blocks, a, b) {
            let rect = rim_sim::command::build_preview(w, thing, stuff, ra, rb, facing);
            let mut mine = Vec::new();
            for (p, pl) in rect {
                let pl = match pl {
                    pl if goes_up(pl) && foot(p).iter().any(|c| claimed.contains(c)) => {
                        Place::Blocked(Blocker::Overlap)
                    }
                    other => other,
                };
                if goes_up(pl) {
                    mine.extend(foot(p));
                }
                cells.push((p, pl));
            }
            claimed.extend(mine);
        }
        let mut bp = BuildPreview {
            thing,
            dragging,
            cells,
            facing,
            key: (a, b, facing, stuff, w.tick),
            hint: None,
            refusal: String::new(),
        };
        bp.refusal = bp.blocked(app).unwrap_or_else(|| "Nothing can be built here".into());
        bp.hint = bp.work_out_hint(app);
        bp
    }

    /// How many go up, of how many the drag asked for (a cell a footprint
    /// already covers isn't asked).
    pub fn counts(&self) -> (usize, usize) {
        let asked = self.cells.iter().filter(|(_, pl)| *pl != Place::Blocked(Blocker::Overlap)).count();
        let up = self.cells.iter().filter(|(_, pl)| goes_up(*pl)).count();
        (up, asked)
    }

    /// Why the first blocked cell is blocked: "Blocked by a table".
    fn blocked(&self, app: &App) -> Option<String> {
        let w = &app.sim.world;
        let why = self.cells.iter().find_map(|(_, pl)| match pl {
            Place::Blocked(b) if *b != Blocker::Overlap => Some(*b),
            _ => None,
        })?;
        Some(match why {
            Blocker::Terrain(p) => {
                format!("Can't build on {}", w.defs.terrain[w.map.terrain[w.map.idx(p)] as usize].label.to_lowercase())
            }
            Blocker::Solid(_) => "Can't build in solid rock".into(),
            Blocker::Occupied(e) => match w.thing(e) {
                Some(t) => format!("Blocked by {}", with_article(&w.defs.thing(t.def).label)),
                None => "Something is in the way".into(),
            },
            Blocker::OutOfBounds => "Off the edge of the map".into(),
            Blocker::Overlap => unreachable!("filtered out above"),
            Blocker::NoMaterial => "No material chosen to build it from".into(),
            Blocker::NotBuildable => "This can't be built".into(),
            Blocker::NoDig => "Nothing here to dig into".into(),
            // A bridge goes over a pit and nowhere else.
            Blocker::NotOverAir => "It only spans an open pit".into(),
            // Research or a plugin holds it back: its own reason, "Research joinery".
            Blocker::Locked => w.build_lock(self.thing).unwrap_or_else(|| "Can't be built yet".into()),
        })
    }

    fn work_out_hint(&self, app: &App) -> Option<String> {
        let (up, asked) = self.counts();
        if !self.dragging {
            return if up == 0 { self.blocked(app) } else { None };
        }
        let noun = plural(&app.sim.world.defs.thing(self.thing).label, asked);
        let count = if up == asked { format!("{asked} {noun}") } else { format!("{up} of {asked} {noun}") };
        let cost = crate::build_cost_for(app, self.thing, up as u32);
        Some(if cost.is_empty() { count } else { format!("{count} · {cost}") })
    }

    /// The pointer's hint: "18 walls · 90 logs", "16 of 18 walls · 80 logs"
    /// during a drag; why over a cell it can't go.
    pub fn hint(&self) -> Option<String> {
        self.hint.clone()
    }

    /// Why a click here put nothing up.
    pub fn refusal(&self) -> String {
        self.refusal.clone()
    }
}

/// Does a plan go up in this cell: now, after clearing, or in place of
/// what stands there?
fn goes_up(pl: Place) -> bool {
    matches!(pl, Place::Open | Place::Clears(_) | Place::Replaces(_))
}

/// A label with "a" or "an" before it.
fn with_article(label: &str) -> String {
    let an = label.chars().next().is_some_and(|c| "aeiouAEIOU".contains(c));
    format!("{} {label}", if an { "an" } else { "a" })
}

/// `label`, counted: "wall", "walls", "workbenches", "berry bushes".
fn plural(label: &str, n: usize) -> String {
    if n == 1 {
        return label.to_string();
    }
    let consonant_y = label.ends_with('y') && !["ay", "ey", "oy", "uy"].iter().any(|e| label.ends_with(e));
    if consonant_y {
        format!("{}ies", &label[..label.len() - 1])
    } else if ["s", "x", "ch", "sh"].iter().any(|e| label.ends_with(e)) {
        format!("{label}es")
    } else {
        format!("{label}s")
    }
}

/// A drag's size in cells: "5 × 4".
fn size(a: rim_sim::IVec, b: rim_sim::IVec) -> String {
    format!("{} × {}", (a.x - b.x).abs() + 1, (a.y - b.y).abs() + 1)
}

/// A selected thing's mark on screen: a pawn's ring's bounds, a thing's
/// footprint, whether or not it's in view.
fn bounds(app: &App, e: Entity) -> Option<[f32; 4]> {
    let gap = app.palette.bracket_gap + app.palette.firm;
    if let Some(((x, y), r)) = draw::pawn_disc(app, e) {
        let out = r + gap;
        return Some([x - out, y - out, out * 2.0, out * 2.0]);
    }
    let [x, y, w, h] = footprint(app, &app.sim.world.thing(e)?);
    Some([x - gap, y - gap, w + 2.0 * gap, h + 2.0 * gap])
}

/// Chevrons sit at least this far inside the screen's edge, and further
/// in where a panel is.
const EDGE_INSET: f32 = 16.0;
/// Chevrons closer than this are one chevron for all they point at.
const CHEVRON_MERGE: f32 = 28.0;

/// Where the line from the screen's centre to an off-screen `target`
/// leaves the screen, `EDGE_INSET` inside it, and the line's angle.
pub fn edge_point((sw, sh): (f32, f32), target: (f32, f32)) -> ((f32, f32), f32) {
    let (cx, cy) = (sw / 2.0, sh / 2.0);
    let (dx, dy) = (target.0 - cx, target.1 - cy);
    let reach = ((cx - EDGE_INSET) / dx.abs().max(1e-6)).min((cy - EDGE_INSET) / dy.abs().max(1e-6));
    ((cx + dx * reach, cy + dy * reach), dy.atan2(dx))
}

/// A chevron for selected things off screen: where it sits, which way it
/// points, what it points at (the inspector's first), and how many cells
/// past the view's edge the nearest is.
#[derive(Clone, Debug, PartialEq)]
pub struct Offscreen {
    pub at: (f32, f32),
    pub angle: f32,
    pub of: Vec<Entity>,
    pub cells: i32,
}

/// The selection's chevrons. Each steps in from the edge until no panel
/// covers it, and chevrons that land together merge.
pub fn offscreen(app: &App) -> Vec<Offscreen> {
    let (sw, sh, dpi) = (screen_width(), screen_height(), screen_dpi_scale());
    let (v0, v1) = (app.cam.to_world(0.0, 0.0), app.cam.to_world(sw, sh));
    let mut picked = crate::selection(app);
    // The inspector's one first, so a merged chevron leads to it.
    if let Some(i) = picked.iter().position(|&e| Some(e) == app.selected) {
        picked.swap(0, i);
    }
    let mut out: Vec<Offscreen> = Vec::new();
    for e in picked {
        let Some(b) = bounds(app, e).filter(|&b| !on_screen(b)) else { continue };
        let target = (b[0] + b[2] / 2.0, b[1] + b[3] / 2.0);
        let ((mut x, mut y), angle) = edge_point((sw, sh), target);
        let (ux, uy) = (-angle.cos(), -angle.sin());
        for _ in 0..40 {
            if !app.ui.covers(x * dpi, y * dpi) {
                break;
            }
            (x, y) = (x + ux * 8.0, y + uy * 8.0);
        }
        let (wx, wy) = app.cam.to_world(target.0, target.1);
        let past = |v: f32, lo: f32, hi: f32| (lo - v).max(v - hi).max(0.0);
        let cells = past(wx, v0.0, v1.0).hypot(past(wy, v0.1, v1.1)).round() as i32;
        match out.iter_mut().find(|o| (o.at.0 - x).hypot(o.at.1 - y) < CHEVRON_MERGE) {
            Some(o) => {
                o.of.push(e);
                o.cells = o.cells.min(cells);
            }
            None => out.push(Offscreen { at: (x, y), angle, of: vec![e], cells }),
        }
    }
    out
}

/// A chevron's chip: its text, and whether it hangs to the chevron's left
/// (for a chevron on the right, pointing out).
fn offscreen_chip(app: &App, o: &Offscreen) -> (String, bool) {
    let w = &app.sim.world;
    let name = |e: Entity| {
        w.ecs
            .get::<&Pawn>(e)
            .map(|p| p.name.clone())
            .ok()
            .or_else(|| w.thing(e).map(|t| w.defs.thing(t.def).label.clone()))
    };
    let who = match o.of.len() {
        1 => name(o.of[0]).unwrap_or_default(),
        n => format!("{n} selected"),
    };
    let cells = if o.cells == 1 { "1 cell".to_string() } else { format!("{} cells", o.cells) };
    (format!("{who} · {cells}"), o.angle.cos() > 0.0)
}

/// Where a chevron's chip goes: on the screen side of it.
fn chip_anchor(o: &Offscreen) -> (f32, f32) {
    (o.at.0 - o.angle.cos() * 16.0, o.at.1 - o.angle.sin() * 16.0 - 10.0)
}

/// The selected thing a click on a chevron, or its chip, would bring back.
pub fn offscreen_at(app: &mut App, (x, y): (f32, f32)) -> Option<Entity> {
    const HIT: f32 = 14.0;
    let dpi = screen_dpi_scale();
    for o in offscreen(app) {
        let (text, right) = offscreen_chip(app, &o);
        let chip = Chip { at: chip_anchor(&o), text, right };
        let [cx, cy, cw, ch] = chip_box(&app.palette, &mut app.ui.text, &chip, dpi);
        let on_chip = (cx..=cx + cw).contains(&x) && (cy..=cy + ch).contains(&y);
        if on_chip || (o.at.0 - x).hypot(o.at.1 - y) <= HIT {
            return Some(o.of[0]);
        }
    }
    None
}

/// Does a screen rectangle touch the screen?
fn on_screen([x, y, w, h]: [f32; 4]) -> bool {
    x + w >= 0.0 && y + h >= 0.0 && x <= screen_width() && y <= screen_height()
}

/// The overlay for this frame.
pub fn scene(app: &App) -> Scene {
    let p = &app.palette;
    let w = &app.sim.world;
    let cam = &app.cam;
    let now = app.chalk.now;
    let mut marks = Vec::new();
    let picked = crate::selection(app);
    let group = picked.len() > 1;
    let gap_for = |e: Entity| match app.chalk.since(e) {
        Some(t0) if !app.chalk.instant => closing(p.bracket_gap, now - t0),
        _ => p.bracket_gap,
    };
    // A select drag: its box, and who it will pick or take out.
    let boxing = app.drag_start.filter(|_| crate::is_box(app, app.pointer)).map(|a| {
        let b = cam.tile_at(app.pointer.0, app.pointer.1);
        (a, b, crate::boxed_colonists(app, a, b))
    });
    let leaving = |e: Entity| app.subtract && boxing.as_ref().is_some_and(|(_, _, boxed)| boxed.contains(&e));
    // The inspector's one, when it's on screen: where a group's chip goes.
    let mut primary = None;
    for &e in &picked {
        let first = !group || app.selected == Some(e);
        let alpha = if leaving(e) {
            LEAVING_ALPHA
        } else if first {
            1.0
        } else {
            GROUP_ALPHA
        };
        let gap = gap_for(e);
        if let Some((center, r)) = draw::pawn_disc(app, e) {
            if let Ok(pawn) = w.ecs.get::<&Pawn>(e) {
                let mut points = vec![center];
                // The way on this level: stairs take it out of view.
                let here = pawn.path.iter().rev().take_while(|s| s.z == cam.z);
                points.extend(here.map(|s| cam.to_screen(s.x as f32 + 0.5, s.y as f32 + 0.5)));
                if points.len() > 1 {
                    marks.push(Mark::Path { points, alpha });
                }
            }
            let out = r + gap;
            if on_screen([center.0 - out, center.1 - out, out * 2.0, out * 2.0]) {
                marks.push(Mark::Ring { center, r, gap, alpha });
                if first {
                    primary = Some((center.0 + out + 6.0, center.1 - 10.0));
                }
            }
        } else if let Some(t) = w.thing(e).filter(|t| t.pos.z == cam.z) {
            let rect = footprint(app, &t);
            if on_screen([rect[0] - gap, rect[1] - gap, rect[2] + 2.0 * gap, rect[3] + 2.0 * gap]) {
                marks.push(Mark::Brackets { rect, gap, alpha });
                if first {
                    primary = Some((rect[0] + rect[2] + gap + 6.0, rect[1] - gap));
                }
            }
        }
    }
    if let Some((a, b, boxed)) = &boxing {
        let (lo, hi) = ((a.x.min(b.x), a.y.min(b.y)), (a.x.max(b.x) + 1, a.y.max(b.y) + 1));
        let (x0, y0) = cam.to_screen(lo.0 as f32, lo.1 as f32);
        let (x1, y1) = cam.to_screen(hi.0 as f32, hi.1 as f32);
        marks.push(Mark::Marquee { rect: [x0, y0, x1 - x0, y1 - y0], dashed: app.subtract, color: None });
        // Who the box will pick: with Shift, only the newcomers.
        if !app.subtract {
            for &e in boxed.iter().filter(|e| !(app.shift && picked.contains(e))) {
                if let Some((center, r)) = draw::pawn_disc(app, e) {
                    marks.push(Mark::HoverRing { center, r, alpha: PICKING_ALPHA });
                }
            }
        }
    }
    for (h, a) in app.chalk.hovers() {
        let Hovered::Thing(e) = h else { continue };
        if let Some((center, r)) = draw::pawn_disc(app, e) {
            marks.push(Mark::HoverRing { center, r, alpha: a });
        } else if let Some(t) = w.thing(e) {
            marks.push(Mark::Hover { rect: footprint(app, &t), alpha: a });
        }
    }
    // Where the last order landed: one ring, a quarter of a second.
    if let Some((cell, at)) = app.order_flash {
        let k = ((now - at) / ACK_SECS) as f32;
        if (0.0..1.0).contains(&k) {
            let center = cam.to_screen(cell.x as f32 + 0.5, cell.y as f32 + 0.5);
            let (r, alpha) = if app.chalk.instant { (0.5, 1.0) } else { (0.3 + 0.45 * k, 1.0 - k) };
            marks.push(Mark::Ack { center, r: r * cam.zoom, alpha });
        }
    }
    // Urgent marks breathe: the one mark that keeps moving.
    let (x0, y0, x1, y1) = draw::visible(app);
    for (e, _) in w.ecs.query::<(Entity, &rim_sim::world::Urgent)>().iter() {
        // A creature's ring is around it; a thing's around the amber disc
        // at its top-left. A blueprint draws no disc, so it gets no ring.
        let (center, r) = if let Some(disc) = draw::pawn_disc(app, e) {
            disc
        } else {
            let Some(t) = w.thing(e).filter(|_| w.ecs.get::<&rim_sim::world::Blueprint>(e).is_err()) else { continue };
            if !((x0..=x1).contains(&t.pos.x) && (y0..=y1).contains(&t.pos.y)) {
                continue;
            }
            let (sx, sy) = cam.to_screen(t.pos.x as f32, t.pos.y as f32);
            ((sx + cam.zoom * 0.2, sy + cam.zoom * 0.2), cam.zoom * 0.17)
        };
        let phase = if app.chalk.instant { 0.35 } else { ((now / BREATHE_SECS) % 1.0) as f32 };
        marks.push(Mark::Breathe { center, r: r + 1.0 + phase * r * 1.4, alpha: 0.35 * (1.0 - phase) });
    }
    // A selection off screen: a chevron at the edge toward it, and how far.
    for o in offscreen(app) {
        let (text, right) = offscreen_chip(app, &o);
        marks.push(Mark::Offscreen { at: o.at, angle: o.angle });
        marks.push(Mark::Chip(Chip { at: chip_anchor(&o), text, right }));
    }
    // Beside the inspector's one: speech sits above a pawn and its name
    // below, so a chip there would be covered.
    if let (true, Some(at)) = (group, primary) {
        marks.push(Mark::Chip(Chip { at, text: format!("{} selected", picked.len()), right: false }));
    }
    // A job no one can reach carries a notch at its bottom-left, and says
    // why under the pointer. The map's regions answer: a few lookups a job.
    let (x0, y0, x1, y1) = draw::visible(app);
    for e in rim_sim::ai::unreachable_jobs(w, IVec::at(x0, y0, cam.z), IVec::at(x1, y1, cam.z)) {
        let (rect, cells) = if let Some((c, r)) = draw::pawn_disc(app, e) {
            ([c.0 - r, c.1 - r, 2.0 * r, 2.0 * r], w.pawn_pos(e).map(|p| (p, [1, 1])))
        } else if let Some(t) = w.thing(e) {
            let size = w.defs.thing(t.def).size_facing(t.facing);
            (footprint(app, &t), Some((t.pos, size)))
        } else {
            continue;
        };
        let at = (rect[0], rect[1] + rect[3]);
        marks.push(Mark::Notch { at, size: (cam.zoom * NOTCH_CELL).max(4.0) });
        let over = |(p, [fw, fh]): (IVec, [u32; 2])| {
            let (fw, fh) = (fw as i32, fh as i32);
            app.hover_cell.is_some_and(|c| (p.x..p.x + fw).contains(&c.x) && (p.y..p.y + fh).contains(&c.y))
        };
        if cells.is_some_and(over) {
            marks.push(Mark::Chip(Chip { at: (at.0, at.1 + 4.0), text: "No one can reach this".into(), right: false }));
        }
    }
    // Measuring: each heavier line's number where it crosses the
    // pointer's row (above it) and column (beside it), fading with the
    // lines. The screen's edges are under panels, and the hover readout
    // already names the cell. Zoomed out, every other number or more
    // goes, so they never run together.
    let majors = crate::grid::strength(app).majors;
    if let (true, Some(cell)) = (majors > 0.0, app.hover_cell.filter(|&c| w.map.inb(c))) {
        let (_, row_top) = cam.to_screen(0.0, cell.y as f32);
        let (col_right, _) = cam.to_screen(cell.x as f32 + 1.0, 0.0);
        let (x0, y0, x1, y1) = draw::visible(app);
        let every = crate::grid::MAJOR_EVERY;
        let apart = every * (LABEL_SPACING / (every as f32 * cam.zoom)).ceil().max(1.0) as i32;
        for gx in (x0..=x1 + 1).filter(|g| g % apart == 0) {
            let (x, _) = cam.to_screen(gx as f32, 0.0);
            marks.push(Mark::Label { at: (x + 3.0, row_top - p.caption - 5.0), text: gx.to_string(), alpha: majors });
        }
        for gy in (y0..=y1 + 1).filter(|g| g % apart == 0) {
            let (_, y) = cam.to_screen(0.0, gy as f32);
            marks.push(Mark::Label { at: (col_right + 3.0, y + 2.0), text: gy.to_string(), alpha: majors });
        }
    }
    // An order tool: what it would mark, or take back, in its hue.
    if let Some(op) = &app.order_preview {
        let hue = match op.designation {
            Some(d) => crate::rgb(w.defs.designations[d as usize].rgb),
            None => p.chalk,
        };
        // Only what's on screen: a big drag is mostly off it.
        let aims = op.targets.iter().filter_map(|t| aim_of(app, t)).filter(|aim| match *aim {
            Aim::Rect(r) => on_screen(r),
            Aim::Disc((x, y), r) => on_screen([x - r, y - r, 2.0 * r, 2.0 * r]),
        });
        if op.dragging {
            let (lo, hi) = ((op.a.x.min(op.b.x), op.a.y.min(op.b.y)), (op.a.x.max(op.b.x) + 1, op.a.y.max(op.b.y) + 1));
            let (x0, y0) = cam.to_screen(lo.0 as f32, lo.1 as f32);
            let (x1, y1) = cam.to_screen(hi.0 as f32, hi.1 as f32);
            let cancel = op.designation.is_none();
            marks.push(Mark::Marquee { rect: [x0, y0, x1 - x0, y1 - y0], dashed: cancel, color: Some(hue) });
            marks.extend(aims.map(|aim| Mark::Target { aim, color: hue, cancel }));
        } else if op.targets.is_empty() {
            let (x, y) = cam.to_screen(op.b.x as f32, op.b.y as f32);
            marks.push(Mark::Frame { rect: [x, y, cam.zoom, cam.zoom], alpha: FAINT_FRAME, color: None });
        } else {
            marks.extend(aims.map(|aim| Mark::Aimed { aim, color: hue }));
        }
    }
    // A build tool: a ghost of what goes up, cell by cell; a refused click
    // shakes it.
    if let Some(bp) = &app.build_preview {
        let td = w.defs.thing(bp.thing);
        let [fw, fh] = td.size_facing(bp.facing);
        let multi = fw * fh > 1;
        // Only the refused cell's ghost shakes.
        let shake_at = |p: IVec| match &app.refused {
            Some((_, at, t0)) if *at == p && !app.chalk.instant && ((now - t0) / SHAKE_SECS) < 1.0 => {
                let k = ((now - t0) / SHAKE_SECS) as f32;
                (k * std::f32::consts::TAU * 2.0).sin() * 3.0 * (1.0 - k)
            }
            _ => 0.0,
        };
        let cell_rect = |p: IVec, dx: f32| {
            let (x, y) = cam.to_screen(p.x as f32, p.y as f32);
            [x + dx, y, cam.zoom, cam.zoom]
        };
        let foot = |p: IVec, dx: f32| {
            let [x, y, _, _] = cell_rect(p, dx);
            [x, y, cam.zoom * fw as f32, cam.zoom * fh as f32]
        };
        for &(p, pl) in &bp.cells {
            let dx = shake_at(p);
            let rect = if multi { foot(p, dx) } else { cell_rect(p, dx) };
            // Only what's on screen: a big drag is mostly off it.
            if !on_screen(rect) {
                continue;
            }
            match pl {
                Place::Open => marks.push(Mark::Ghost { rect, clears: false, facing: multi.then_some(bp.facing) }),
                // A replacement goes up once the old piece comes down: it
                // clears first too.
                Place::Clears(_) | Place::Replaces(_) => {
                    marks.push(Mark::Ghost { rect, clears: true, facing: multi.then_some(bp.facing) })
                }
                Place::Blocked(Blocker::Overlap) => {}
                Place::Blocked(b) => {
                    // The cross goes on the cell that's in the way.
                    let at = match b {
                        Blocker::Terrain(q) | Blocker::Solid(q) => q,
                        Blocker::Occupied(e) => {
                            td.footprint(p, bp.facing).find(|&c| w.map.fixture_at(c) == Some(e)).unwrap_or(p)
                        }
                        _ => p,
                    };
                    marks.push(Mark::Blocked { rect, cell: cell_rect(at, dx) });
                }
            }
        }
    }
    // A zone tool's cell under the pointer, before a drag starts.
    if matches!(app.tool, crate::Tool::Stockpile | crate::Tool::ClearZone) && app.drag_start.is_none() {
        if let Some(c) = app.hover_cell {
            let (x, y) = cam.to_screen(c.x as f32, c.y as f32);
            marks.push(Mark::Frame { rect: [x, y, cam.zoom, cam.zoom], alpha: 0.8, color: Some(p.zone) });
        }
    }
    // A refused click: its cell's frame shakes, and the hint says why.
    if let Some((_, cell, t0)) = &app.refused {
        let k = ((now - t0) / SHAKE_SECS) as f32;
        if k < 1.0 {
            let shake = if app.chalk.instant { 0.0 } else { (k * std::f32::consts::TAU * 2.0).sin() * 3.0 * (1.0 - k) };
            let (x, y) = cam.to_screen(cell.x as f32, cell.y as f32);
            marks.push(Mark::Frame { rect: [x + shake, y, cam.zoom, cam.zoom], alpha: 1.0, color: None });
        }
    }
    // With the storage overlay on, a selected loose stack shows where it
    // will be carried.
    if app.storage_overlay {
        if let Some(e) = app.selected {
            let here = w.thing(e).filter(|t| t.pos.z == cam.z);
            if let (Some(t), Some(rim_sim::ai::HaulPlan::Moves { to, .. })) = (here, rim_sim::ai::haul_plan(w, e)) {
                let from = cam.to_screen(t.pos.x as f32 + 0.5, t.pos.y as f32 + 0.5);
                marks.push(Mark::Haul { from, to: cam.to_screen(to.x as f32 + 0.5, to.y as f32 + 0.5) });
            }
        }
    }
    Scene { marks }
}

/// `c` with its alpha scaled by `a`, at most opaque.
pub(crate) fn fade(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, (c.a * a).min(1.0))
}

/// Four corner brackets around `rect`, `gap` outside it, each on a
/// keyline. An arm is a pair of rectangles, so corners are square at any
/// width; each corner's keyline is two rectangles that don't overlap, so
/// its translucency doesn't double at the corner.
pub fn brackets(p: &Palette, [x, y, w, h]: [f32; 4], gap: f32, c: Color, a: f32) {
    let (x0, y0, x1, y1) = (x - gap, y - gap, x + w + gap, y + h + gap);
    let (arm, t) = (p.bracket_arm(w, h), p.firm);
    let h2 = t / 2.0;
    let key = fade(p.keyline, a);
    for (cx, cy, sx, sy) in [(x0, y0, 1.0, 1.0), (x1, y0, -1.0, 1.0), (x0, y1, 1.0, -1.0), (x1, y1, -1.0, -1.0)] {
        // The corner's outer edge, and how far each arm runs in from it.
        let (ox, oy) = (cx - sx * h2, cy - sy * h2);
        // `k` is the keyline's width beyond the stroke.
        let (len, k) = (arm + h2, 1.0);
        let span = |o: f32, s: f32, n: f32| if s > 0.0 { (o, n) } else { (o - n, n) };
        // Across: the arm along x, the full thickness, keyline on both sides.
        let (ax, aw) = span(ox - sx * k, sx, len + k * 2.0);
        let (ay, ah) = span(oy - sy * k, sy, t + k * 2.0);
        draw_rectangle(ax, ay, aw, ah, key);
        // Down: the arm along y, starting where the across keyline ends.
        let (bx, bw) = span(ox - sx * k, sx, t + k * 2.0);
        let (by, bh) = span(oy + sy * (t + k), sy, len - t);
        draw_rectangle(bx, by, bw, bh, key);
        let (hx, hw) = span(ox, sx, len);
        let (hy, hh) = span(oy, sy, t);
        draw_rectangle(hx, hy, hw, hh, fade(c, a));
        let (vx, vw) = span(ox, sx, t);
        let (vy, vh) = span(oy + sy * t, sy, len - t);
        draw_rectangle(vx, vy, vw, vh, fade(c, a));
    }
}

/// A rectangle's outline with round corners, as triangles that never
/// overlap, so a translucent edge is even all the way round. `r` is the
/// corner radius of the line's centre.
fn rounded_edge([x, y, w, h]: [f32; 4], r: f32, t: f32, c: Color) {
    const STEPS: usize = 5;
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let corners =
        [(x + w - r, y + r, -0.25), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 0.25), (x + r, y + r, 0.5)];
    let mut ring = Vec::with_capacity(4 * (STEPS + 1));
    for (cx, cy, turn) in corners {
        for i in 0..=STEPS {
            let a = (turn + 0.25 * i as f32 / STEPS as f32) * std::f32::consts::TAU;
            let (dx, dy) = (a.cos(), a.sin());
            let (ro, ri) = (r + t / 2.0, (r - t / 2.0).max(0.0));
            ring.push((vec2(cx + dx * ro, cy + dy * ro), vec2(cx + dx * ri, cy + dy * ri)));
        }
    }
    for i in 0..ring.len() {
        let ((o0, i0), (o1, i1)) = (ring[i], ring[(i + 1) % ring.len()]);
        draw_triangle(o0, o1, i0, c);
        draw_triangle(i0, o1, i1, c);
    }
}

/// A ring with a keyline.
pub fn ring(p: &Palette, (x, y): (f32, f32), r: f32, t: f32, c: Color, a: f32) {
    draw_circle_lines(x, y, r, t + 2.0, fade(p.keyline, a));
    draw_circle_lines(x, y, r, t, fade(c, a));
}

/// Dots every few points along a polyline, each on a keyline. Only the
/// dots on screen are drawn: a long walk is mostly elsewhere.
fn dotted(p: &Palette, points: &[(f32, f32)], c: Color, a: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let mut carry = 0.0;
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let seen = on_screen([x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()]);
        let mut d = carry;
        while d < len {
            if seen {
                let f = d / len;
                let (x, y) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                if (0.0..=sw).contains(&x) && (0.0..=sh).contains(&y) {
                    draw_circle(x, y, 1.6, fade(p.keyline, a));
                    draw_circle(x, y, 0.9, fade(c, a));
                }
            }
            d += DOT_STEP;
        }
        carry = d - len;
    }
}

/// Dashes a third of a cell long from `a` to `b`, on a keyline.
fn dashed(p: &Palette, (ax, ay): (f32, f32), (bx, by): (f32, f32), z: f32) {
    let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1.0);
    let dash = (z * 0.3).max(4.0);
    let mut d = 0.0;
    while d < len {
        let (f, g) = (d / len, (d + dash).min(len) / len);
        let (x0, y0, x1, y1) = (ax + (bx - ax) * f, ay + (by - ay) * f, ax + (bx - ax) * g, ay + (by - ay) * g);
        draw_line(x0, y0, x1, y1, p.firm + 2.0, p.keyline);
        draw_line(x0, y0, x1, y1, p.firm, p.chalk);
        d += dash * 2.0;
    }
}

/// A right triangle in `threat` filling the corner at `at` (a footprint's
/// bottom-left), on a keyline that stays outside the footprint's edge.
fn notch(p: &Palette, (x, y): (f32, f32), s: f32) {
    let k = 1.0;
    draw_triangle(vec2(x - k, y + k), vec2(x + s + 2.0 * k, y + k), vec2(x - k, y - s - 2.0 * k), p.keyline);
    draw_triangle(vec2(x, y), vec2(x + s, y), vec2(x, y - s), p.threat);
}

/// Where a ring mark's line is centred. Like a thing's, a pawn's hover
/// lies on the edge of its body, and its selection `gap` outside it; the
/// two lines and their keylines never touch.
pub fn ring_radius(p: &Palette, m: &Mark) -> Option<f32> {
    match m {
        Mark::HoverRing { r, .. } => Some(r - p.stroke / 2.0),
        Mark::Ring { r, gap, .. } => Some(r + gap + p.firm / 2.0),
        _ => None,
    }
}

/// A select drag's box: a faint chalk fill and a hairline on a keyline,
/// broken into dashes when it takes colonists out.
fn marquee(p: &Palette, [x, y, w, h]: [f32; 4], broken: bool, c: Color) {
    draw_rectangle(x, y, w, h, fade(c, MARQUEE_FILL));
    let t = p.hair;
    // Top and bottom run the full width; the sides fit between them, so
    // no two pieces share a pixel and the keyline is even at the corners.
    let sides = [
        (x, y, w, t, true),
        (x, y + h - t, w, t, true),
        (x, y + t, t, h - 2.0 * t, false),
        (x + w - t, y + t, t, h - 2.0 * t, false),
    ];
    for (sx, sy, sw, sh, along) in sides {
        let len = if along { sw } else { sh };
        let (dash, gap) = if broken { (4.0, 3.0) } else { (len, 0.0) };
        let mut d = 0.0;
        while d < len {
            let n = dash.min(len - d);
            let (rx, ry, rw, rh) = if along { (sx + d, sy, n, sh) } else { (sx, sy + d, sw, n) };
            // Where a side meets the top or the bottom, its keyline stops
            // at theirs.
            let (start, end) = (!along && d == 0.0, !along && d + n >= len);
            let ky0 = if start { ry + 1.0 } else { ry - 1.0 };
            let ky1 = if end { ry + rh - 1.0 } else { ry + rh + 1.0 };
            let key = if along {
                (rx - 1.0, ry - 1.0, rw + 2.0, rh + 2.0)
            } else {
                (rx - 1.0, ky0, rw + 2.0, (ky1 - ky0).max(0.0))
            };
            draw_rectangle(key.0, key.1, key.2, key.3, p.keyline);
            draw_rectangle(rx, ry, rw, rh, c);
            d += dash + gap;
        }
    }
}

/// A chalk chevron on a keyline at `at`, pointing along `angle`.
fn chevron(p: &Palette, (x, y): (f32, f32), angle: f32) {
    let (c, s) = (angle.cos(), angle.sin());
    let pt = |u: f32, v: f32| vec2(x + u * c - v * s, y + u * s + v * c);
    let (tip, back, notch, fore) = (pt(7.0, 0.0), pt(-5.0, -7.0), pt(-2.0, 0.0), pt(-5.0, 7.0));
    for (a, b) in [(tip, back), (back, notch), (notch, fore), (fore, tip)] {
        draw_line(a.x, a.y, b.x, b.y, 3.0, p.keyline);
    }
    draw_triangle(tip, back, notch, p.chalk);
    draw_triangle(tip, notch, fore, p.chalk);
}

/// A ghost of a plan: the intent wash and edge, a caution triangle where a
/// natural thing is cleared first, and for a thing bigger than a cell a
/// chevron on the side it faces.
fn ghost(p: &Palette, [x, y, w, h]: [f32; 4], clears: bool, facing: Option<u8>) {
    draw_rectangle(x + 1.0, y + 1.0, w - 2.0, h - 2.0, p.intent_fill);
    let inner = [x + 1.5, y + 1.5, w - 3.0, h - 3.0];
    draw_rectangle_lines(
        inner[0] - 1.0,
        inner[1] - 1.0,
        inner[2] + 2.0,
        inner[3] + 2.0,
        1.25 + 2.0,
        fade(p.keyline, 0.45),
    );
    draw_rectangle_lines(inner[0], inner[1], inner[2], inner[3], 1.25, fade(p.intent, 0.92));
    if clears {
        let q = (w.min(h) * 0.3).max(6.0);
        let (tx, ty) = (x + 3.0, y + 3.0);
        let (a, b, c) = (vec2(tx, ty), vec2(tx + q, ty), vec2(tx, ty + q));
        for (u, v) in [(a, b), (b, c), (c, a)] {
            draw_line(u.x, u.y, v.x, v.y, 2.0, p.keyline);
        }
        draw_triangle(a, b, c, p.caution);
    }
    if let Some(f) = facing {
        // Quarter turns clockwise from south: the chevron sits inside the
        // edge it faces, pointing out.
        let angle = std::f32::consts::FRAC_PI_2 * (f as f32 + 1.0);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let (dx, dy) = (angle.cos(), angle.sin());
        let reach = (w / 2.0 * dx.abs() + h / 2.0 * dy.abs()) - 7.0;
        chevron(p, (cx + dx * reach, cy + dy * reach), angle);
    }
}

/// A red cross over a cell, on a keyline.
fn cross(p: &Palette, [x, y, w, h]: [f32; 4]) {
    let i = (w * 0.28).max(3.0);
    for (a, b) in [((x + i, y + i), (x + w - i, y + h - i)), ((x + w - i, y + i), (x + i, y + h - i))] {
        draw_line(a.0, a.1, b.0, b.1, 1.75 + 2.0, fade(p.keyline, 0.6));
        draw_line(a.0, a.1, b.0, b.1, 1.75, p.threat);
    }
}

/// A target an order drag will mark: a ring in its hue and its dot to
/// come, or for Cancel a dimming over what it takes back.
fn target(p: &Palette, aim: Aim, c: Color, cancel: bool) {
    match (aim, cancel) {
        (Aim::Rect([x, y, w, h]), true) => draw_rectangle(x, y, w, h, fade(p.keyline, 0.8)),
        (Aim::Disc((x, y), r), true) => draw_circle(x, y, r, fade(p.keyline, 0.8)),
        (Aim::Rect([x, y, w, h]), false) => {
            ring(p, (x + w / 2.0, y + h / 2.0), w.min(h) / 2.0 + 0.5, p.stroke, c, 1.0);
            // The designation's dot, as it will be: top-right, on a keyline.
            let (dx, dy, r) = (x + w - w.min(h) * 0.18, y + w.min(h) * 0.18, (w.min(h) * 0.13).max(2.0));
            draw_circle(dx, dy, r + 1.0, fade(p.keyline, PREVIEW_DOT));
            draw_circle(dx, dy, r, fade(c, PREVIEW_DOT));
        }
        (Aim::Disc(center, r), false) => ring(p, center, r + 2.0, p.stroke, c, 1.0),
    }
}

/// Paint the scene's world marks. Chips are text: `chips` turns them into
/// the UI's draw list.
pub fn draw(scene: &Scene, p: &Palette, zoom: f32) {
    for m in &scene.marks {
        match m {
            Mark::Path { points, alpha } => dotted(p, points, p.chalk, 0.6 * alpha),
            Mark::Haul { from, to } => dashed(p, *from, *to, zoom),
            Mark::Marquee { rect, dashed: broken, color } => marquee(p, *rect, *broken, color.unwrap_or(p.chalk)),
            Mark::Aimed { aim, color } => match *aim {
                Aim::Rect(r) => {
                    let inner = [r[0] + p.stroke / 2.0, r[1] + p.stroke / 2.0, r[2] - p.stroke, r[3] - p.stroke];
                    let radius = (r[2].min(r[3]) * 0.16).min(5.0);
                    rounded_edge(inner, radius, p.stroke + 2.0, p.keyline);
                    rounded_edge(inner, radius, p.stroke, *color);
                }
                Aim::Disc(c, r) => ring(p, c, r - p.stroke / 2.0, p.stroke, *color, 1.0),
            },
            Mark::Target { aim, color, cancel } => target(p, *aim, *color, *cancel),
            Mark::Ghost { rect, clears, facing } => ghost(p, *rect, *clears, *facing),
            Mark::Blocked { rect, cell } => {
                let [x, y, w, h] = *rect;
                if [x, y, w, h] != *cell {
                    // A footprint that can't go: grey, not blue.
                    draw_rectangle(x, y, w, h, fade(p.chalk, 0.1));
                    draw_rectangle_lines(x + 1.0, y + 1.0, w - 2.0, h - 2.0, p.stroke, fade(p.chalk, 0.5));
                }
                let [cx, cy, cw, ch] = *cell;
                draw_rectangle(cx + 1.0, cy + 1.0, cw - 2.0, ch - 2.0, fade(p.threat, 0.13));
                cross(p, *cell);
            }
            Mark::Frame { rect, alpha, color } => {
                let [x, y, w, h] = *rect;
                draw_rectangle_lines(x - 1.0, y - 1.0, w + 2.0, h + 2.0, p.hair + 2.0, fade(p.keyline, *alpha));
                draw_rectangle_lines(x, y, w, h, p.hair, fade(color.unwrap_or(p.chalk), *alpha));
            }
            Mark::Hover { rect, alpha } => {
                let a = HOVER_ALPHA * alpha;
                // On the footprint's edge: the line sits just inside it.
                let inner =
                    [rect[0] + p.stroke / 2.0, rect[1] + p.stroke / 2.0, rect[2] - p.stroke, rect[3] - p.stroke];
                let r = (rect[2].min(rect[3]) * 0.16).min(5.0);
                rounded_edge(inner, r, p.stroke + 2.0, fade(p.keyline, a));
                rounded_edge(inner, r, p.stroke, fade(p.chalk, a));
            }
            Mark::HoverRing { center, alpha, .. } => {
                ring(p, *center, ring_radius(p, m).unwrap_or(0.0), p.stroke, p.chalk, 0.8 * alpha)
            }
            Mark::Brackets { rect, gap, alpha } => brackets(p, *rect, *gap, p.chalk, *alpha),
            Mark::Ring { center, alpha, .. } => {
                ring(p, *center, ring_radius(p, m).unwrap_or(0.0), p.firm, p.chalk, *alpha)
            }
            Mark::Notch { at, size } => notch(p, *at, *size),
            Mark::Offscreen { at, angle, .. } => chevron(p, *at, *angle),
            Mark::Ack { center, r, alpha } => ring(p, *center, *r, p.stroke, p.chalk, *alpha),
            Mark::Breathe { center, r, alpha } => {
                draw_circle_lines(center.0, center.1, *r, p.stroke, fade(p.caution, *alpha))
            }
            Mark::Chip(_) | Mark::Label { .. } => {}
        }
    }
}

/// A chip's box on screen, in points: its text measured, placed from its
/// anchor, and kept on screen.
pub fn chip_box(p: &Palette, text: &mut rim_ui::text::Text, c: &Chip, dpi: f32) -> [f32; 4] {
    let (pad, size) = (p.caption * 0.64, p.caption);
    let h = (size * p.leading + pad).round();
    let quads = text.quads(&c.text, size * dpi, 500, 0.0, None, 0.0, 0.0);
    let tw = quads.iter().map(|q| (q.dst[0] + q.dst[2]) / dpi).fold(0.0f32, f32::max);
    let w = (tw + pad * 2.0).ceil();
    let x0 = if c.right { c.at.0 - w } else { c.at.0 };
    let x = x0.clamp(4.0, (screen_width() - w - 4.0).max(4.0));
    let y = c.at.1.clamp(4.0, (screen_height() - h - 4.0).max(4.0));
    [x, y, w, h]
}

/// Text on the map with a dark shadow a point down and right, in the
/// UI's font: `look` is (shadow, text). Stack counts, worksite readouts
/// and measuring's numbers all draw this way.
pub fn shadowed(
    text: &mut rim_ui::text::Text,
    s: &str,
    size: f32,
    weight: u16,
    (x, y): (f32, f32),
    (shadow, colour): (Rgba, Rgba),
    dpi: f32,
) -> [Draw; 2] {
    let mut at = |d: f32| text.quads(s, size * dpi, weight, 0.0, None, (x + d) * dpi, (y + d) * dpi);
    [Draw::Glyphs { quads: at(1.0), color: shadow }, Draw::Glyphs { quads: at(0.0), color: colour }]
}

/// The scene's chips as UI draws, in physical pixels: a small panel with
/// the text in the UI's font, kept on screen. Measuring's numbers go
/// here too, as they're text.
pub fn chips(scene: &Scene, p: &Palette, text: &mut rim_ui::text::Text, dpi: f32) -> Vec<Draw> {
    let rgba = |c: Color| [c.r, c.g, c.b, c.a];
    let (pad, size) = (p.caption * 0.64, p.caption);
    let mut out = Vec::new();
    for m in &scene.marks {
        if let Mark::Label { at, text: t, alpha } = m {
            let look = (rgba(fade(p.keyline, *alpha)), rgba(fade(p.chalk, 0.85 * alpha)));
            out.extend(shadowed(text, t, size * 0.9, 500, *at, look, dpi));
            continue;
        }
        let Mark::Chip(c) = m else { continue };
        let [x, y, w, h] = chip_box(p, text, c, dpi);
        let mut quads = text.quads(&c.text, size * dpi, 500, 0.0, None, 0.0, 0.0);
        let r = [x * dpi, y * dpi, w * dpi, h * dpi];
        out.push(Draw::Rect { rect: r, color: rgba(p.surface), radius: p.radius * dpi });
        out.push(Draw::Outline { rect: r, color: rgba(p.line), width: dpi, radius: p.radius * dpi });
        // Quads are laid out from the line's top; centre the line in the chip.
        let (dx, dy) = (((x + pad) * dpi).round(), ((y + (h - size * p.leading) / 2.0) * dpi).round());
        for q in &mut quads {
            q.dst[0] += dx;
            q.dst[1] += dy;
        }
        out.push(Draw::Glyphs { quads, color: rgba(p.text) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_labels_read_as_english() {
        assert_eq!(plural("wall", 1), "wall");
        assert_eq!(plural("wall", 2), "walls");
        assert_eq!(plural("workbench", 3), "workbenches");
        assert_eq!(plural("berry bush", 2), "berry bushes");
        assert_eq!(plural("quarry", 2), "quarries");
        assert_eq!(plural("tray", 2), "trays");
        assert_eq!(plural("thing", 0), "things");
        assert_eq!(with_article("oak tree"), "an oak tree");
        assert_eq!(with_article("table"), "a table");
    }

    #[test]
    fn a_theme_that_sets_chalk_changes_the_selection_colour() {
        let mut t = Theme::default();
        let core = Palette::from_theme(&t);
        assert_eq!(core.chalk, Color::new(0xf2 as f32 / 255.0, 0xee as f32 / 255.0, 0xe3 as f32 / 255.0, 1.0));
        t.color.insert("chalk".into(), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(Palette::from_theme(&t).chalk, Color::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn hover_fades_in_and_out_from_where_it_got_to() {
        let (a, b) = (Hovered::Zone(1), Hovered::Zone(2));
        let mut s = State::default();
        let mut t = 0.0;
        let mut step = |s: &mut State, h: Option<Hovered>, secs: f64| {
            t += secs;
            s.update(&[], h, t, false);
        };
        step(&mut s, Some(a), 0.0);
        step(&mut s, Some(a), HOVER_IN_SECS);
        assert_eq!(s.zone_hover(1), 1.0);
        // Across a gap to another: the first keeps fading out meanwhile.
        step(&mut s, None, 0.02);
        step(&mut s, Some(b), 0.02);
        let half = s.zone_hover(1);
        assert!(half > 0.0 && half < 1.0, "{half}");
        // Back before it's gone: it fades in from there, not from nothing.
        step(&mut s, Some(a), 0.01);
        assert!(s.zone_hover(1) > half);
        step(&mut s, Some(a), HOVER_OUT_SECS);
        assert_eq!(s.zone_hover(1), 1.0);
        assert_eq!(s.zone_hover(2), 0.0);
    }

    #[test]
    fn hover_and_selection_rings_never_touch() {
        let p = Palette::from_theme(&Theme::default());
        let (c, r) = ((0.0, 0.0), 9.0);
        let hover = ring_radius(&p, &Mark::HoverRing { center: c, r, alpha: 1.0 }).unwrap();
        let select = ring_radius(&p, &Mark::Ring { center: c, r, gap: p.bracket_gap, alpha: 1.0 }).unwrap();
        // Each line's keyline reaches a point past its half-width.
        let (hover_out, select_in) = (hover + p.stroke / 2.0 + 1.0, select - p.firm / 2.0 - 1.0);
        assert!(select_in - hover_out >= 0.5, "{hover_out} {select_in}");
        assert_eq!(hover + p.stroke / 2.0, r, "hover's outside edge is the body's");
    }

    #[test]
    fn a_chevron_sits_inside_the_edge_toward_its_target() {
        let screen = (800.0, 600.0);
        let ((x, y), angle) = edge_point(screen, (-500.0, 300.0));
        assert_eq!((x, y), (EDGE_INSET, 300.0));
        assert!((angle - std::f32::consts::PI).abs() < 1e-6, "points left: {angle}");
        let ((x, y), angle) = edge_point(screen, (400.0, 5000.0));
        assert_eq!((x, y), (400.0, 600.0 - EDGE_INSET));
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 1e-6, "points down: {angle}");
    }

    #[test]
    fn bracket_arms_follow_the_footprint_within_limits() {
        let p = Palette::from_theme(&Theme::default());
        assert_eq!(p.bracket_arm(10.0, 10.0), 4.0);
        assert_eq!(p.bracket_arm(100.0, 100.0), 12.0);
        assert!((p.bracket_arm(28.0, 56.0) - 7.84).abs() < 1e-4);
    }

    #[test]
    fn selection_closes_in_to_its_gap_whatever_the_gap() {
        for gap in [3.0, 10.0] {
            assert_eq!(closing(gap, 0.0), gap + SELECT_FROM);
            assert_eq!(closing(gap, SELECT_IN_SECS), gap);
            assert_eq!(closing(gap, 5.0), gap);
            let mid = closing(gap, SELECT_IN_SECS / 2.0);
            assert!(mid > gap && mid < gap + SELECT_FROM);
        }
    }
}
