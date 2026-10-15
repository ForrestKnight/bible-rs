//! Small vector icons painted with egui primitives (no icon font needed).
//!
//! Each icon is drawn on a 16 × 16 grid scaled to fit its box, with one stroke weight
//! ([`metrics::ICON_STROKE`]) and round line ends. Lines sit on quarter points so that, in the
//! toolbar's 16 pt box, a 1.5 pt stroke covers whole pixels on a Retina display.

use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

use crate::theme::metrics;

/// A 16 × 16 design grid mapped onto the centre of a box.
struct Grid {
    origin: Pos2,
    scale: f32,
}

impl Grid {
    fn new(r: Rect) -> Self {
        let scale = r.width().min(r.height()) / 16.0;
        Self { origin: r.center() - vec2(8.0, 8.0) * scale, scale }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
}

fn stroke(c: Color32) -> Stroke {
    Stroke::new(metrics::ICON_STROKE, c)
}

/// An open path with round ends.
fn path(p: &Painter, pts: Vec<Pos2>, c: Color32) {
    let r = metrics::ICON_STROKE / 2.0;
    p.circle_filled(pts[0], r, c);
    p.circle_filled(pts[pts.len() - 1], r, c);
    p.add(Shape::line(pts, stroke(c)));
}

/// A rounded rectangle outline, the stroke centred on the rectangle's edge.
fn frame(p: &Painter, r: Rect, radius: f32, c: Color32) {
    p.rect_stroke(r, CornerRadius::same(radius.round() as u8), stroke(c), StrokeKind::Middle);
}

pub fn bookmark(p: &Painter, r: Rect, c: Color32, filled: bool) {
    let pts = vec![
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        pos2(r.center().x, r.bottom() - r.height() * 0.28),
        r.left_bottom(),
    ];
    if filled {
        p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
    } else {
        p.add(Shape::closed_line(pts, stroke(c)));
    }
}

pub fn note(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    frame(p, g.rect(2.75, 1.75, 13.25, 14.25), 2.0 * g.scale, c);
    for (y, x1) in [(5.75, 10.75), (8.75, 10.75), (11.75, 8.25)] {
        path(p, vec![g.p(5.25, y), g.p(x1, y)], c);
    }
}

pub fn chevron_left(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    path(p, vec![g.p(10.25, 2.75), g.p(5.25, 8.0), g.p(10.25, 13.25)], c);
}

pub fn chevron_right(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    path(p, vec![g.p(5.75, 2.75), g.p(10.75, 8.0), g.p(5.75, 13.25)], c);
}

pub fn chevron_down(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    path(p, vec![g.p(3.25, 5.75), g.p(8.0, 10.25), g.p(12.75, 5.75)], c);
}

pub fn search(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    p.circle_stroke(g.p(6.75, 6.75), 5.0 * g.scale, stroke(c));
    path(p, vec![g.p(10.5, 10.5), g.p(14.25, 14.25)], c);
}

pub fn sidebar(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    frame(p, g.rect(1.25, 2.75, 14.75, 13.25), 2.0 * g.scale, c);
    p.line_segment([g.p(5.75, 2.75), g.p(5.75, 13.25)], stroke(c));
}

pub fn columns(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    frame(p, g.rect(1.25, 2.75, 14.25, 13.25), 2.0 * g.scale, c);
    p.line_segment([g.p(7.75, 2.75), g.p(7.75, 13.25)], stroke(c));
}

/// A large and a small "A" (text size and theme).
pub fn text_size(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    path(p, vec![g.p(0.75, 13.25), g.p(4.75, 2.75), g.p(8.75, 13.25)], c);
    path(p, vec![g.p(2.25, 9.75), g.p(7.25, 9.75)], c);
    path(p, vec![g.p(9.75, 13.25), g.p(12.5, 6.25), g.p(15.25, 13.25)], c);
    path(p, vec![g.p(10.75, 10.75), g.p(14.25, 10.75)], c);
}

pub fn close(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    path(p, vec![g.p(4.25, 4.25), g.p(11.75, 11.75)], c);
    path(p, vec![g.p(11.75, 4.25), g.p(4.25, 11.75)], c);
}

pub fn book(p: &Painter, r: Rect, c: Color32) {
    let g = Grid::new(r);
    for side in [-1.0, 1.0] {
        let x = |dx: f32| 8.0 + side * dx;
        p.add(Shape::closed_line(
            vec![g.p(8.0, 4.25), g.p(x(6.75), 2.75), g.p(x(6.75), 12.25), g.p(8.0, 13.75)],
            stroke(c),
        ));
    }
}

pub fn dot(p: &Painter, center: Pos2, radius: f32, c: Color32) {
    p.circle_filled(center, radius, c);
}
