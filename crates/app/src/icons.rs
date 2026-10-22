//! Line icons painted with egui primitives (no icon font needed).
//!
//! The geometry is the design's icon set (SF-Symbols-like, see `ds-05-icons` in the Paper file):
//! a 20 × 20 grid with a 16 pt live area, 1.5-unit strokes with round caps and joins, copied
//! from the design's SVGs. An icon scales with its box, stroke included, exactly as the SVGs
//! do: 20 pt in the toolbar, 16 pt in rows and menus. A few small glyphs (disclosure chevrons)
//! use their own 12-unit grid.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, vec2};

/// Stroke width in grid units.
const STROKE: f32 = 1.5;

thread_local! {
    static WEIGHT: std::cell::Cell<f32> = const { std::cell::Cell::new(STROKE) };
}

/// Paints icons inside `f` with heavier 1.75-unit strokes, as the design does for 16 pt glyphs
/// in tiles, menus and fields.
pub fn heavy<R>(f: impl FnOnce() -> R) -> R {
    WEIGHT.with(|w| w.set(1.75));
    let r = f();
    WEIGHT.with(|w| w.set(STROKE));
    r
}

/// An `units`-square design grid mapped onto the centre of a box.
struct Grid<'a> {
    painter: &'a Painter,
    origin: Pos2,
    scale: f32,
    color: Color32,
    width: f32,
}

impl<'a> Grid<'a> {
    fn new(painter: &'a Painter, r: Rect, color: Color32) -> Self {
        Self::with(painter, r, color, 20.0, WEIGHT.with(|w| w.get()))
    }

    fn with(painter: &'a Painter, r: Rect, color: Color32, units: f32, stroke: f32) -> Self {
        let scale = r.width().min(r.height()) / units;
        Self {
            painter,
            origin: r.center() - vec2(units, units) * (scale / 2.0),
            scale,
            color,
            width: stroke * scale,
        }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn stroke(&self) -> Stroke {
        Stroke::new(self.width, self.color)
    }

    /// An open polyline with round caps and joins.
    fn line(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.polyline(pts, false);
    }

    fn polyline(&self, pts: Vec<Pos2>, closed: bool) {
        let r = self.width / 2.0;
        for (i, q) in pts.iter().enumerate() {
            // Caps, and joins at corners (a miter would poke out at sharp angles).
            if !closed && (i == 0 || i == pts.len() - 1) || pts.len() <= 8 {
                self.painter.circle_filled(*q, r, self.color);
            }
        }
        if closed {
            self.painter.add(Shape::closed_line(pts, self.stroke()));
        } else {
            self.painter.add(Shape::line(pts, self.stroke()));
        }
    }

    fn closed(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.polyline(pts, true);
    }

    fn fill(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.add(Shape::convex_polygon(pts, self.color, Stroke::NONE));
    }

    /// A rounded rectangle outline (SVG `rect` with `rx`), the stroke centred on its edge.
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, rx: f32) {
        let r = Rect::from_min_max(self.p(x, y), self.p(x + w, y + h));
        self.painter.rect_stroke(r, rx * self.scale, self.stroke(), StrokeKind::Middle);
    }

    fn circle(&self, cx: f32, cy: f32, r: f32) {
        self.painter.circle_stroke(self.p(cx, cy), r * self.scale, self.stroke());
    }

    fn dot(&self, cx: f32, cy: f32, r: f32) {
        self.painter.circle_filled(self.p(cx, cy), r * self.scale, self.color);
    }

    /// Points along a circular arc around `(cx, cy)` from angle `a0` to `a1` (radians, y down).
    /// Points along a cubic Bézier, as SVG's `C`.
    fn cubic(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> Vec<(f32, f32)> {
        (0..=8)
            .map(move |i| {
                let t = i as f32 / 8.0;
                let u = 1.0 - t;
                let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                (w0 * a.0 + w1 * b.0 + w2 * c.0 + w3 * d.0, w0 * a.1 + w1 * b.1 + w2 * c.1 + w3 * d.1)
            })
            .collect()
    }

    /// Points along SVG's endpoint arc `A r r 0 0 sweep x y` (a circle, the small arc) from
    /// `from` to `to`, `from` excluded.
    fn arc_to(from: (f32, f32), to: (f32, f32), r: f32, sweep: bool) -> Vec<(f32, f32)> {
        let (dx, dy) = ((to.0 - from.0) / 2.0, (to.1 - from.1) / 2.0);
        let d = (dx * dx + dy * dy).sqrt();
        let r = r.max(d);
        let h = (r * r - d * d).max(0.0).sqrt();
        // The centre lies on the perpendicular through the midpoint, on the side that makes the
        // small arc turn the way `sweep` says (clockwise on screen when set).
        let (nx, ny) = (-dy / d, dx / d);
        let sign = if sweep { 1.0 } else { -1.0 };
        let (cx, cy) = (from.0 + dx + sign * h * nx, from.1 + dy + sign * h * ny);
        let a0 = (from.1 - cy).atan2(from.0 - cx);
        let mut a1 = (to.1 - cy).atan2(to.0 - cx);
        if sweep && a1 < a0 {
            a1 += 2.0 * PI;
        } else if !sweep && a1 > a0 {
            a1 -= 2.0 * PI;
        }
        Self::arc(cx, cy, r, a0, a1).skip(1).collect()
    }

    fn arc(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> impl Iterator<Item = (f32, f32)> {
        let n = ((a1 - a0).abs() / 0.2).ceil().max(4.0) as usize;
        (0..=n).map(move |i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
    }
}

pub fn sidebar(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(2.75, 4.25, 14.5, 11.5, 2.5);
    g.line(&[(7.75, 4.25), (7.75, 15.75)]);
}

/// Side-by-side columns (parallel translations).
pub fn columns(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(2.75, 4.25, 14.5, 11.5, 2.5);
    g.line(&[(10.0, 4.25), (10.0, 15.75)]);
}

pub fn chevron_left(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(12.0, 4.5), (6.5, 10.0), (12.0, 15.5)]);
}

pub fn chevron_right(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(8.0, 4.5), (13.5, 10.0), (8.0, 15.5)]);
}

/// The small disclosure chevrons (12-unit grid): pickers, selects and source-list rows.
pub fn chevron_down(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::with(p, r, c, 12.0, STROKE);
    g.line(&[(3.0, 4.5), (6.0, 7.5), (9.0, 4.5)]);
}

pub fn chevron_up(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::with(p, r, c, 12.0, STROKE);
    g.line(&[(3.0, 7.5), (6.0, 4.5), (9.0, 7.5)]);
}

pub fn disclosure(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::with(p, r, c, 12.0, STROKE);
    g.line(&[(4.5, 3.0), (7.5, 6.0), (4.5, 9.0)]);
}

/// Up-and-down chevrons of a select (popup button).
pub fn chevrons_up_down(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(6.5, 8.0), (10.0, 4.5), (13.5, 8.0)]);
    g.line(&[(6.5, 12.0), (10.0, 15.5), (13.5, 12.0)]);
}

/// A large and a small "A" (text format).
pub fn text_size(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(2.75, 15.5), (7.0, 4.5), (11.25, 15.5)]);
    g.line(&[(4.4, 11.5), (9.6, 11.5)]);
    g.line(&[(12.5, 15.5), (15.0, 9.25), (17.5, 15.5)]);
    g.line(&[(13.4, 13.4), (16.6, 13.4)]);
}

/// A cloud struck through: an online text that can't be reached.
pub fn cloud_slash(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts = vec![(6.0, 15.25), (14.25, 15.25)];
    pts.extend(Grid::arc_to((14.25, 15.25), (14.75, 8.79), 3.25, false));
    pts.extend(Grid::arc_to((14.75, 8.79), (5.6, 8.1), 4.75, false));
    pts.extend(Grid::arc_to((5.6, 8.1), (6.0, 15.25), 3.6, false));
    pts.pop();
    g.closed(&pts);
    g.line(&[(3.25, 3.25), (16.75, 16.75)]);
}

/// Four corner brackets pointing out (focus mode).
pub fn expand(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(3.25, 7.25), (3.25, 3.25), (7.25, 3.25)]);
    g.line(&[(16.75, 7.25), (16.75, 3.25), (12.75, 3.25)]);
    g.line(&[(3.25, 12.75), (3.25, 16.75), (7.25, 16.75)]);
    g.line(&[(16.75, 12.75), (16.75, 16.75), (12.75, 16.75)]);
}

/// The magnifying glass in the usual 1.5-unit stroke (empty states, icon buttons).
pub fn search_thin(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(8.75, 8.75, 5.25);
    g.line(&[(12.75, 12.75), (16.75, 16.75)]);
}

/// The magnifying glass. Drawn a little heavier (1.75 units), as the design does in fields.
pub fn search(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::with(p, r, c, 20.0, 1.75);
    g.circle(8.75, 8.75, 5.25);
    g.line(&[(12.75, 12.75), (16.75, 16.75)]);
}

pub fn bookmark(p: &Painter, r: Rect, c: Color32, filled: bool) {
    let g = Grid::new(p, r, c);
    let pts = [(5.75, 3.25), (14.25, 3.25), (14.25, 16.75), (10.0, 13.25), (5.75, 16.75)];
    if filled {
        // Not convex at the notch: fill the two halves.
        g.fill(&[(5.75, 3.25), (10.0, 3.25), (10.0, 13.25), (5.75, 16.75)]);
        g.fill(&[(10.0, 3.25), (14.25, 3.25), (14.25, 16.75), (10.0, 13.25)]);
    }
    g.closed(&pts);
}

pub fn highlighter(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.closed(&[(11.75, 3.75), (16.25, 8.25), (8.75, 15.75), (4.25, 15.75), (4.25, 11.25)]);
    g.line(&[(9.5, 6.0), (14.0, 10.5)]);
}

pub fn note(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(4.0, 2.75, 12.0, 14.5, 2.25);
    g.line(&[(7.25, 7.0), (12.75, 7.0)]);
    g.line(&[(7.25, 10.0), (12.75, 10.0)]);
    g.line(&[(7.25, 13.0), (10.25, 13.0)]);
}

/// A closed book seen from the front, with its page edge curling under.
pub fn book(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts = vec![(4.25, 15.5), (4.25, 4.75)];
    pts.extend(Grid::arc(6.25, 4.75, 2.0, PI, PI + FRAC_PI_2));
    pts.extend([(15.75, 2.75), (15.75, 15.25), (6.25, 15.25)]);
    pts.extend(Grid::arc(6.25, 17.25, 2.0, -FRAC_PI_2, -3.0 * FRAC_PI_2));
    pts.push((15.75, 19.25));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

pub fn close(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(5.5, 5.5), (14.5, 14.5)]);
    g.line(&[(14.5, 5.5), (5.5, 14.5)]);
}

/// A clock face (recent places).
pub fn clock(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 7.25);
    g.line(&[(10.0, 6.0), (10.0, 10.0), (12.75, 11.75)]);
}

/// A filled circle with an × knocked out (clear a field).
pub fn clear(p: &Painter, r: Rect, circle: Color32, cross: Color32) {
    p.circle_filled(r.center(), r.width() / 2.0, circle);
    // A 10 pt × (on the 20-unit grid, 2.25 units) in the middle of the 16 pt circle.
    let g = Grid::with(p, Rect::from_center_size(r.center(), r.size() * 0.625), cross, 20.0, 2.25);
    g.line(&[(5.5, 5.5), (14.5, 14.5)]);
    g.line(&[(14.5, 5.5), (5.5, 14.5)]);
}

/// Three lines with bullets (a chapter, a list).
pub fn list(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    for y in [5.5, 10.0, 14.5] {
        g.line(&[(7.75, y), (16.75, y)]);
        g.circle(4.0, y, 0.75);
    }
}

/// Opposing arrows (compare across translations).
pub fn compare(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(3.25, 7.0), (15.75, 7.0)]);
    g.line(&[(12.75, 4.0), (15.75, 7.0), (12.75, 10.0)]);
    g.line(&[(16.75, 13.0), (4.25, 13.0)]);
    g.line(&[(7.25, 10.0), (4.25, 13.0), (7.25, 16.0)]);
}

/// A clock with an arrow running back round it (reading history).
pub fn history(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    // The arc of "M3.6 8.2A6.75 6.75 0 1 1 3.25 10": round the face from just above 9 o'clock.
    // Clockwise (y down) from its start angle round to 9 o'clock.
    let a0 = (8.2f32 - 10.0).atan2(3.6 - 10.0).rem_euclid(2.0 * PI);
    let pts: Vec<Pos2> = Grid::arc(10.0, 10.0, 6.75, a0, 3.0 * PI).map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
    g.line(&[(2.75, 4.75), (2.75, 8.5), (6.5, 8.5)]);
    g.line(&[(10.0, 6.5), (10.0, 10.25), (12.5, 11.75)]);
}

/// Two overlapping pages (copy).
pub fn copy(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(6.75, 6.75, 10.5, 10.5, 2.25);
    let mut pts = vec![(13.25, 3.25), (5.75, 3.25)];
    pts.extend(Grid::arc(5.75, 5.75, 2.5, -FRAC_PI_2, -PI));
    pts.push((3.25, 13.25));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

pub fn plus(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(10.0, 4.25), (10.0, 15.75)]);
    g.line(&[(4.25, 10.0), (15.75, 10.0)]);
}

/// Two quotation marks (copy with reference).
pub fn quote(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    for x in [0.0, 7.0] {
        // "M4.25 11.5c0-3.25 1.5-5.25 4-6": a tail curling up to the right.
        let pts: Vec<Pos2> = (0..=8)
            .map(|i| {
                let t = i as f32 / 8.0;
                let (x0, y0, x1, y1, x2, y2) = (4.25, 11.5, 4.25, 7.0, 8.25, 5.5);
                let a = (1.0 - t) * (1.0 - t);
                let b = 2.0 * (1.0 - t) * t;
                let d = t * t;
                g.p(x + a * x0 + b * x1 + d * x2, a * y0 + b * y1 + d * y2)
            })
            .collect();
        g.polyline(pts, false);
        g.circle(6.0 + x, 13.0, 1.75);
    }
}

/// A tray with an arrow out of it (share).
pub fn share(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(10.0, 2.75), (10.0, 12.75)]);
    g.line(&[(6.75, 6.0), (10.0, 2.75), (13.25, 6.0)]);
    let mut pts = vec![(6.5, 8.75), (5.75, 8.75)];
    pts.extend(Grid::arc(5.75, 10.25, 1.5, -FRAC_PI_2, -PI));
    pts.push((4.25, 15.75));
    pts.extend(Grid::arc(5.75, 15.75, 1.5, PI, FRAC_PI_2));
    pts.push((14.25, 17.25));
    pts.extend(Grid::arc(14.25, 15.75, 1.5, FRAC_PI_2, 0.0));
    pts.push((15.75, 10.25));
    pts.extend(Grid::arc(14.25, 10.25, 1.5, 0.0, -FRAC_PI_2));
    pts.push((13.5, 8.75));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

/// An I-beam (select a range of text).
pub fn text_cursor(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(10.0, 4.0), (10.0, 16.0)]);
    g.line(&[(7.5, 4.0), (12.5, 4.0)]);
    g.line(&[(7.5, 16.0), (12.5, 16.0)]);
}

/// A circle struck through (clear, none).
pub fn slash_circle(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 7.25);
    g.line(&[(4.9, 15.1), (15.1, 4.9)]);
}

/// A bin (delete).
pub fn trash(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(3.75, 5.5), (16.25, 5.5)]);
    g.line(&[(8.0, 5.5), (8.0, 3.75), (12.0, 3.75), (12.0, 5.5)]);
    g.line(&[(5.25, 5.5), (6.0, 16.25), (7.0, 17.2), (13.0, 17.2), (14.0, 16.25), (14.75, 5.5)]);
}

/// A sun (appearance).
pub fn sun(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 3.25);
    for k in 0..8 {
        let a = k as f32 * PI / 4.0;
        g.line(&[(10.0 + 5.6 * a.cos(), 10.0 + 5.6 * a.sin()), (10.0 + 7.4 * a.cos(), 10.0 + 7.4 * a.sin())]);
    }
}

/// A keyboard (shortcuts).
pub fn keyboard(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(2.25, 5.25, 15.5, 10.0, 2.0);
    for (x0, x1, y) in [
        (5.5, 6.0, 8.5),
        (8.25, 8.75, 8.5),
        (11.25, 11.75, 8.5),
        (14.0, 14.5, 8.5),
        (5.5, 6.0, 11.75),
        (14.0, 14.5, 11.75),
        (8.25, 11.75, 11.75),
    ] {
        g.line(&[(x0, y), (x1, y)]);
    }
}

/// An inbox tray (data).
pub fn tray(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.closed(&[(2.75, 11.25), (5.0, 4.75), (15.0, 4.75), (17.25, 11.25), (17.25, 15.25), (2.75, 15.25)]);
    g.line(&[(2.75, 11.25), (6.75, 11.25), (7.75, 13.25), (12.25, 13.25), (13.25, 11.25), (17.25, 11.25)]);
}

/// A cube in three-quarter view: an object from the text with a 3D model (the reader's glyph).
pub fn cube(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.closed(&[(10.0, 2.75), (16.25, 6.25), (16.25, 13.75), (10.0, 17.25), (3.75, 13.75), (3.75, 6.25)]);
    g.line(&[(3.75, 6.25), (10.0, 9.75), (16.25, 6.25)]);
    g.line(&[(10.0, 9.75), (10.0, 17.25)]);
}

/// A check mark, drawn heavier (2.5 units) as on the design's selected radios and done steps.
pub fn check(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::with(p, r, c, 20.0, 2.5);
    g.line(&[(4.75, 10.5), (8.25, 14.0), (15.25, 6.0)]);
}

/// An arrow pointing right (Continue).
pub fn arrow_right(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(3.75, 10.0), (16.25, 10.0)]);
    g.line(&[(11.75, 5.5), (16.25, 10.0), (11.75, 14.5)]);
}

/// An arrow down into a tray (import, download).
pub fn download(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(10.0, 2.75), (10.0, 12.25)]);
    g.line(&[(6.5, 8.75), (10.0, 12.25), (13.5, 8.75)]);
    let mut pts = vec![(3.25, 12.0), (3.25, 14.75)];
    pts.extend(Grid::arc(5.25, 14.75, 2.0, PI, FRAC_PI_2));
    pts.extend(Grid::arc(14.75, 14.75, 2.0, FRAC_PI_2, 0.0));
    pts.push((16.75, 12.0));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

/// A globe (the translation library).
pub fn globe(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 7.25);
    g.line(&[(2.75, 10.0), (17.25, 10.0)]);
    let mut pts = Grid::cubic((10.0, 2.75), (12.0, 4.75), (13.0, 7.25), (13.0, 10.0));
    pts.extend(Grid::cubic((13.0, 10.0), (13.0, 12.75), (12.0, 15.25), (10.0, 17.25)).into_iter().skip(1));
    pts.extend(Grid::cubic((10.0, 17.25), (8.0, 15.25), (7.0, 12.75), (7.0, 10.0)).into_iter().skip(1));
    pts.extend(Grid::cubic((7.0, 10.0), (7.0, 7.25), (8.0, 4.75), (10.0, 2.75)).into_iter().skip(1));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, true);
}

/// Three ticked lines (reading plans).
pub fn checklist(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(3.5, 5.5), (4.75, 6.75), (7.0, 4.25)]);
    g.line(&[(10.0, 5.5), (16.5, 5.5)]);
    g.line(&[(3.5, 10.5), (4.75, 11.75), (7.0, 9.25)]);
    g.line(&[(10.0, 10.5), (16.5, 10.5)]);
    g.line(&[(10.0, 15.5), (16.5, 15.5)]);
    g.line(&[(4.0, 15.5), (6.25, 15.5)]);
}

/// A key (API keys).
pub fn key(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(7.0, 12.5, 3.75);
    g.line(&[(9.75, 9.75), (16.25, 3.25)]);
    g.line(&[(14.25, 5.25), (16.25, 7.25)]);
    g.line(&[(12.5, 7.0), (14.0, 8.5)]);
}

/// An arrow up out of a tray (export, share a file).
pub fn upload(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(10.0, 12.25), (10.0, 2.75)]);
    g.line(&[(6.5, 6.25), (10.0, 2.75), (13.5, 6.25)]);
    let mut pts = vec![(3.25, 12.0), (3.25, 14.75)];
    pts.extend(Grid::arc(5.25, 14.75, 2.0, PI, FRAC_PI_2));
    pts.extend(Grid::arc(14.75, 14.75, 2.0, FRAC_PI_2, 0.0));
    pts.push((16.75, 12.0));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

/// Two chain links (linked verses): the design's path and its half-turn copy.
pub fn link(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut half: Vec<(f32, f32)> = Grid::arc(10.625, 9.382, 3.0, 2.358, 0.784).collect();
    half.extend(Grid::arc(13.125, 6.875, 3.005, FRAC_PI_4, -3.0 * FRAC_PI_4));
    half.push((10.25, 5.5));
    for flip in [false, true] {
        let pts: Vec<Pos2> =
            half.iter().map(|&(x, y)| if flip { g.p(20.0 - x, 20.0 - y) } else { g.p(x, y) }).collect();
        g.polyline(pts, false);
    }
}

/// A calendar page.
pub fn calendar(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(3.0, 4.25, 14.0, 12.5, 2.25);
    g.line(&[(3.0, 8.25), (17.0, 8.25)]);
    g.line(&[(6.75, 2.75), (6.75, 5.75)]);
    g.line(&[(13.25, 2.75), (13.25, 5.75)]);
}

/// A flame (streaks).
pub fn flame(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts: Vec<(f32, f32)> = Vec::new();
    pts.extend(Grid::cubic((10.0, 17.25), (7.0, 17.25), (4.75, 15.15), (4.75, 12.25)));
    pts.extend(Grid::cubic((4.75, 12.25), (4.75, 8.65), (8.0, 6.85), (8.5, 3.25)).into_iter().skip(1));
    pts.extend(Grid::cubic((8.5, 3.25), (11.0, 4.85), (12.5, 6.95), (12.75, 9.25)).into_iter().skip(1));
    pts.extend(Grid::cubic((12.75, 9.25), (13.5, 8.75), (14.0, 7.85), (14.25, 7.0)).into_iter().skip(1));
    pts.extend(Grid::cubic((14.25, 7.0), (15.5, 8.3), (16.0, 10.0), (16.0, 12.25)).into_iter().skip(1));
    pts.extend(Grid::cubic((16.0, 12.25), (16.0, 15.15), (13.5, 17.25), (10.0, 17.25)).into_iter().skip(1));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, true);
}

/// A crescent moon (the dark theme).
pub fn moon(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts: Vec<(f32, f32)> = Grid::arc(13.94, 6.06, 6.75, 1.22, 3.49).collect();
    pts.extend(Grid::arc(9.91, 10.09, 6.75, 4.36, 0.35));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, true);
}

/// A circle half filled (following the system's appearance).
pub fn auto(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 7.25);
    g.line(&[(10.0, 2.75), (10.0, 17.25)]);
    let half: Vec<(f32, f32)> = Grid::arc(10.0, 10.0, 5.5, -FRAC_PI_2, FRAC_PI_2).collect();
    g.fill(&half);
}

/// A page with lines of text (the sepia theme).
pub fn page(p: &Painter, r: Rect, c: Color32) {
    doc(p, r, c);
    let g = Grid::new(p, r, c);
    g.line(&[(7.75, 10.0), (12.25, 10.0)]);
    g.line(&[(7.75, 13.0), (12.25, 13.0)]);
}

/// A folder.
pub fn folder(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts: Vec<(f32, f32)> = Grid::arc(4.25, 6.25, 1.5, PI, 1.5 * PI).collect();
    pts.extend([(7.75, 4.75), (9.5, 6.75)]);
    pts.extend(Grid::arc(15.75, 8.25, 1.5, 1.5 * PI, 2.0 * PI));
    pts.extend(Grid::arc(15.75, 14.75, 1.5, 0.0, FRAC_PI_2));
    pts.extend(Grid::arc(4.25, 14.75, 1.5, FRAC_PI_2, PI));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, true);
}

/// A box with an arrow out of its corner (opens elsewhere).
pub fn external(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts = vec![(8.25, 4.25), (5.25, 4.25)];
    pts.extend(Grid::arc(5.25, 5.75, 1.5, 1.5 * PI, PI));
    pts.push((3.75, 14.75));
    pts.extend(Grid::arc(5.25, 14.75, 1.5, PI, FRAC_PI_2));
    pts.push((14.25, 16.25));
    pts.extend(Grid::arc(14.25, 14.75, 1.5, FRAC_PI_2, 0.0));
    pts.push((15.75, 11.75));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
    g.line(&[(11.25, 3.25), (16.75, 3.25), (16.75, 8.75)]);
    g.line(&[(16.5, 3.5), (9.25, 10.75)]);
}

/// Four corners pointing in (leave full screen).
pub fn contract(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.line(&[(7.25, 3.25), (7.25, 7.25), (3.25, 7.25)]);
    g.line(&[(12.75, 3.25), (12.75, 7.25), (16.75, 7.25)]);
    g.line(&[(7.25, 16.75), (7.25, 12.75), (3.25, 12.75)]);
    g.line(&[(12.75, 16.75), (12.75, 12.75), (16.75, 12.75)]);
}

/// A page with a folded corner (a file).
pub fn doc(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.closed(&[(5.25, 2.75), (11.5, 2.75), (14.75, 6.0), (14.75, 17.25), (5.25, 17.25)]);
    g.line(&[(11.5, 2.75), (11.5, 6.0), (14.75, 6.0)]);
}

/// A padlock (locked modules, unlock keys).
pub fn lock(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.rect(4.25, 8.75, 11.5, 8.5, 2.0);
    let mut pts = vec![(6.75, 8.75), (6.75, 6.5)];
    pts.extend(Grid::arc(10.0, 6.5, 3.25, PI, 2.0 * PI));
    pts.push((13.25, 8.75));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
}

/// A warning triangle with an exclamation mark.
pub fn warning(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let mut pts = Vec::new();
    pts.extend(Grid::arc(10.0, 4.35, 1.5, PI + 0.52, 2.0 * PI - 0.52));
    pts.extend(Grid::arc(16.0, 14.75, 1.5, -0.52, FRAC_PI_2));
    pts.extend(Grid::arc(4.0, 14.75, 1.5, FRAC_PI_2, PI + 0.52));
    let pts: Vec<Pos2> = pts.into_iter().map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, true);
    g.line(&[(10.0, 7.75), (10.0, 11.0)]);
    g.dot(10.0, 13.65, g.width / g.scale / 2.0 + 0.1);
}

/// A circular arrow (try again).
pub fn retry(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    let pts: Vec<Pos2> = Grid::arc(10.0, 10.0, 6.1, 0.208, 5.475).map(|(x, y)| g.p(x, y)).collect();
    g.polyline(pts, false);
    g.line(&[(14.75, 2.75), (14.75, 6.25), (11.25, 6.25)]);
}

/// An "i" in a circle (about).
pub fn info(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    g.circle(10.0, 10.0, 7.25);
    g.line(&[(10.0, 9.25), (10.0, 13.75)]);
    g.dot(10.0, 6.45, 0.9);
}

pub fn ellipsis(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(p, r, c);
    for x in [5.0, 10.0, 15.0] {
        g.dot(x, 10.0, 1.4);
    }
}

pub fn dot(p: &Painter, center: Pos2, radius: f32, c: Color32) {
    p.circle_filled(center, radius, c);
}

/// A rectangle of `size` points centred on `center`, for placing an icon.
pub fn icon_rect(center: Pos2, size: f32) -> Rect {
    Rect::from_center_size(center, vec2(size, size))
}
