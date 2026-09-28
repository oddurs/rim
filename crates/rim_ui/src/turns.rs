//! Turns drawn round (DESIGN.md §6h): where a walking pawn is drawn, and
//! which way it's heading, when its path turns a corner. Presentation only,
//! so it lives beside the UI rather than in the sim, which never reads it
//! back (§6a) and keeps its maths free of the platform's libm.

use rim_sim::world::Pawn;
use rim_sim::IVec;

/// How much of each step beside a corner is drawn as part of the turn
/// (DESIGN.md §6h). Under a half, so the curve stays inside the corner cell.
pub const TURN_ROUND: f32 = 0.35;

fn centre(c: IVec) -> (f32, f32) {
    (c.x as f32 + 0.5, c.y as f32 + 0.5)
}

/// A turn at a corner cell: the unit directions in and out, and how far
/// along each the curve reaches.
struct Turn {
    into: (f32, f32),
    out: (f32, f32),
    r: f32,
}

impl Turn {
    /// The turn made at `corner` by stepping in from `from` and on to `to`,
    /// if it is one: neighbouring cells on one level, neither straight on nor
    /// straight back.
    fn at(from: IVec, corner: IVec, to: IVec) -> Option<Turn> {
        if from.z != corner.z || to.z != corner.z {
            return None;
        }
        let a = (corner.x - from.x, corner.y - from.y);
        let b = (to.x - corner.x, to.y - corner.y);
        let step = |d: (i32, i32)| d != (0, 0) && d.0.abs() <= 1 && d.1.abs() <= 1;
        if !step(a) || !step(b) || a == b || a == (-b.0, -b.1) {
            return None;
        }
        let unit = |d: (i32, i32)| {
            let l = ((d.0 * d.0 + d.1 * d.1) as f32).sqrt();
            ((d.0 as f32 / l, d.1 as f32 / l), l)
        };
        let ((into, la), (out, lb)) = (unit(a), unit(b));
        Some(Turn { into, out, r: TURN_ROUND * la.min(lb) })
    }

    /// The point `u` of the way (0 to 1) along the curve round `c`, and its
    /// direction: a quadratic from `r` short of the centre to `r` past it,
    /// pulled by the centre, so it lies in the triangle those three make.
    fn point(&self, c: (f32, f32), u: f32) -> ((f32, f32), Option<(f32, f32)>) {
        let p0 = (c.0 - self.into.0 * self.r, c.1 - self.into.1 * self.r);
        let p2 = (c.0 + self.out.0 * self.r, c.1 + self.out.1 * self.r);
        let v = 1.0 - u;
        let at = (v * v * p0.0 + 2.0 * v * u * c.0 + u * u * p2.0, v * v * p0.1 + 2.0 * v * u * c.1 + u * u * p2.1);
        let d = (v * self.into.0 + u * self.out.0, v * self.into.1 + u * self.out.1);
        (at, Some(d))
    }
}

/// Where to draw the pawn with its turns drawn round (DESIGN.md §6h).
/// Around a corner, the last and first `TURN_ROUND` of the two steps
/// become one curve through the corner cell's centre, so the pawn never
/// leaves the cell the sim has it in, and at every step's end it is in
/// the cell it stepped into. `from` is the cell the pawn last stepped
/// out of: only the renderer remembers it, and without it the turn the
/// pawn is leaving is drawn square. Presentation only: the sim never
/// reads this back (§6a).
pub fn drawn_through(p: &Pawn, from: Option<IVec>, frac: f32) -> (f32, f32) {
    through(p, from, frac).0
}

/// Which way the drawn pawn is moving, in radians clockwise from north
/// (screen up), or None when it isn't stepping.
pub fn heading_through(p: &Pawn, from: Option<IVec>, frac: f32) -> Option<f32> {
    let (dx, dy) = through(p, from, frac).1?;
    Some(dx.atan2(-dy))
}

/// The drawn point and, while stepping, its direction of travel.
fn through(p: &Pawn, from: Option<IVec>, frac: f32) -> ((f32, f32), Option<(f32, f32)>) {
    let Some(next) = p.next else { return (centre(p.pos), None) };
    let t = ((p.progress as f32 + frac) / p.step_ticks.max(1) as f32).min(1.0);
    let (a, b) = (centre(p.pos), centre(next));
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    // Into a corner at `next`, then out of one at `pos`: at most one
    // applies, since each blend covers under half a step.
    if let Some(c) = p.path.last().and_then(|&after| Turn::at(p.pos, next, after)) {
        let left = (1.0 - t) * len;
        if left < c.r {
            return c.point(b, (c.r - left) / (2.0 * c.r));
        }
    }
    if let Some(c) = from.and_then(|f| Turn::at(f, p.pos, next)) {
        let gone = t * len;
        if gone < c.r {
            return c.point(a, 0.5 + gone / (2.0 * c.r));
        }
    }
    let dir = if len > 0.0 { ((b.0 - a.0) / len, (b.1 - a.1) / len) } else { (0.0, 0.0) };
    ((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), (len > 0.0).then_some(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pawn stepping from `pos` to `next`, then on to `after`, at
    /// `progress` of a 100-tick step.
    fn stepping(pos: IVec, next: IVec, after: Option<IVec>, progress: u32) -> Pawn {
        Pawn {
            pos,
            next: Some(next),
            progress,
            step_ticks: 100,
            path: after.into_iter().collect(),
            ..Default::default()
        }
    }

    fn inside(c: IVec, (x, y): (f32, f32)) -> bool {
        (c.x as f32..=c.x as f32 + 1.0).contains(&x) && (c.y as f32..=c.y as f32 + 1.0).contains(&y)
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5
    }

    #[test]
    fn a_turn_is_drawn_round_inside_the_corner_cell() {
        // East along y = 3, then south at (3, 3): first the step into the
        // corner, then the step out of it, remembering where it came from.
        let (w, c, s) = (IVec::new(2, 3), IVec::new(3, 3), IVec::new(3, 4));
        for prog in 0..=100 {
            let into = stepping(w, c, Some(s), prog);
            let out = stepping(c, s, None, prog);
            let (a, b) = (drawn_through(&into, None, 0.0), drawn_through(&out, Some(w), 0.0));
            // Within TURN_ROUND of the corner the curve is drawn, and stays
            // in the corner cell; elsewhere it is the straight step.
            if prog as f32 > 100.0 * (1.0 - TURN_ROUND) {
                assert!(inside(c, a), "into the corner at {prog}: {a:?}");
                assert!(a.1 > 3.5, "cut toward the inside of the turn at {prog}: {a:?}");
            } else {
                assert!(close(a, into.drawn_at(0.0)), "straight before the turn at {prog}: {a:?}");
            }
            if (prog as f32) < 100.0 * TURN_ROUND {
                assert!(inside(c, b), "out of the corner at {prog}: {b:?}");
            } else {
                assert!(close(b, out.drawn_at(0.0)), "straight after the turn at {prog}: {b:?}");
            }
        }
    }

    #[test]
    fn a_turn_is_one_unbroken_curve() {
        let (w, c, s) = (IVec::new(2, 3), IVec::new(3, 3), IVec::new(3, 4));
        let end = drawn_through(&stepping(w, c, Some(s), 100), None, 0.0);
        let start = drawn_through(&stepping(c, s, None, 0), Some(w), 0.0);
        assert!(close(end, start), "the step's end {end:?} is the next step's start {start:?}");
        // Each frame moves it a little: no jump anywhere along the turn.
        let pts: Vec<(f32, f32)> = (0..=100)
            .map(|p| drawn_through(&stepping(w, c, Some(s), p), None, 0.0))
            .chain((0..=100).map(|p| drawn_through(&stepping(c, s, None, p), Some(w), 0.0)))
            .collect();
        for q in pts.windows(2) {
            let d = ((q[1].0 - q[0].0).powi(2) + (q[1].1 - q[0].1).powi(2)).sqrt();
            assert!(d < 0.02, "a jump of {d} between {:?} and {:?}", q[0], q[1]);
        }
    }

    #[test]
    fn straight_on_straight_back_and_between_levels_are_drawn_as_before() {
        let (a, b) = (IVec::new(2, 3), IVec::new(3, 3));
        for (after, from) in [
            (Some(IVec::new(4, 3)), Some(IVec::new(1, 3))),      // straight on
            (Some(IVec::new(2, 3)), Some(IVec::new(3, 3))),      // back the way it came
            (Some(IVec::at(3, 3, -1)), Some(IVec::at(2, 3, 1))), // a portal either side
            (None, None),                                        // nothing remembered
        ] {
            for prog in 0..=100 {
                let p = stepping(a, b, after, prog);
                assert!(close(drawn_through(&p, from, 0.0), p.drawn_at(0.0)), "{after:?} {from:?} at {prog}");
            }
        }
    }

    #[test]
    fn a_diagonal_turn_stays_in_its_cell() {
        // North-east, then south-east, through (3, 3).
        let (a, c, b) = (IVec::new(2, 4), IVec::new(3, 3), IVec::new(4, 4));
        for prog in 0..=100 {
            let into = drawn_through(&stepping(a, c, Some(b), prog), None, 0.0);
            let out = drawn_through(&stepping(c, b, None, prog), Some(a), 0.0);
            if prog as f32 > 100.0 * (1.0 - TURN_ROUND) {
                assert!(inside(c, into), "{into:?} at {prog}");
            }
            if (prog as f32) < 100.0 * TURN_ROUND {
                assert!(inside(c, out), "{out:?} at {prog}");
            }
        }
    }

    #[test]
    fn the_heading_follows_the_turn() {
        let (w, c, s) = (IVec::new(2, 3), IVec::new(3, 3), IVec::new(3, 4));
        let east = std::f32::consts::FRAC_PI_2;
        let south = std::f32::consts::PI;
        let h = |p: &Pawn, from| heading_through(p, from, 0.0).unwrap();
        assert!((h(&stepping(w, c, Some(s), 10), None) - east).abs() < 1e-5, "east before the turn");
        assert!((h(&stepping(c, s, None, 90), Some(w)) - south).abs() < 1e-5, "south after it");
        let mid = h(&stepping(w, c, Some(s), 100), None);
        assert!(mid > east + 0.1 && mid < south - 0.1, "between the two at the corner: {mid}");
        assert_eq!(
            heading_through(&Pawn { pos: w, ..Default::default() }, None, 0.0),
            None,
            "a standing pawn has none"
        );
    }
}
