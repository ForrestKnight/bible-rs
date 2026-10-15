//! Line icons painted with egui primitives (no icon font needed).
//!
//! The geometry is the design's icon set (SF-Symbols-like, see `ds-05-icons` in the Paper file):
//! a 20 × 20 grid with a 16 pt live area, 1.5-unit strokes with round caps and joins, copied
//! from the design's SVGs. An icon scales with its box, stroke included, exactly as the SVGs
//! do: 20 pt in the toolbar, 16 pt in rows and menus. A few small glyphs (disclosure chevrons)
//! use their own 12-unit grid.

use std::f32::consts::{FRAC_PI_2, PI};

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
