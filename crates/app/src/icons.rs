//! Small vector icons painted with egui primitives (no icon font needed).

use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

fn stroke(c: Color32) -> Stroke {
    Stroke::new(1.5, c)
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
    let r = r.shrink(1.0);
    p.rect_stroke(r, CornerRadius::same(2), stroke(c), StrokeKind::Inside);
    for i in 0..3 {
        let y = r.top() + r.height() * (0.32 + 0.2 * i as f32);
        let x1 = if i == 2 { r.center().x } else { r.right() - 3.0 };
        p.line_segment([pos2(r.left() + 3.0, y), pos2(x1, y)], Stroke::new(1.2, c));
    }
}

pub fn chevron_left(p: &Painter, r: Rect, c: Color32) {
    let m = r.center();
    let s = r.height() * 0.22;
    p.add(Shape::line(vec![pos2(m.x + s * 0.6, m.y - s * 1.3), pos2(m.x - s * 0.7, m.y), pos2(m.x + s * 0.6, m.y + s * 1.3)], stroke(c)));
}

pub fn chevron_right(p: &Painter, r: Rect, c: Color32) {
    let m = r.center();
    let s = r.height() * 0.22;
    p.add(Shape::line(vec![pos2(m.x - s * 0.6, m.y - s * 1.3), pos2(m.x + s * 0.7, m.y), pos2(m.x - s * 0.6, m.y + s * 1.3)], stroke(c)));
}

pub fn chevron_down(p: &Painter, r: Rect, c: Color32) {
    let m = r.center();
    let s = r.height() * 0.16;
    p.add(Shape::line(vec![pos2(m.x - s * 1.4, m.y - s * 0.6), pos2(m.x, m.y + s * 0.7), pos2(m.x + s * 1.4, m.y - s * 0.6)], stroke(c)));
}

pub fn search(p: &Painter, r: Rect, c: Color32) {
    let s = r.height() * 0.22;
    let center = r.center() - vec2(s * 0.35, s * 0.35);
    p.circle_stroke(center, s * 1.15, stroke(c));
    let a = center + vec2(s * 0.85, s * 0.85);
    p.line_segment([a, a + vec2(s * 0.95, s * 0.95)], Stroke::new(1.8, c));
}

pub fn sidebar(p: &Painter, r: Rect, c: Color32) {
    let r = Rect::from_center_size(r.center(), vec2(r.height() * 0.78, r.height() * 0.6));
    p.rect_stroke(r, CornerRadius::same(3), stroke(c), StrokeKind::Inside);
    let x = r.left() + r.width() * 0.36;
    p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], stroke(c));
}

pub fn columns(p: &Painter, r: Rect, c: Color32) {
    let r = Rect::from_center_size(r.center(), vec2(r.height() * 0.78, r.height() * 0.6));
    p.rect_stroke(r, CornerRadius::same(3), stroke(c), StrokeKind::Inside);
    p.line_segment([pos2(r.center().x, r.top()), pos2(r.center().x, r.bottom())], stroke(c));
}

pub fn text_size(p: &Painter, r: Rect, c: Color32) {
    // "Aa"-style glyph drawn as two triangles of different size.
    let base = r.bottom() - r.height() * 0.25;
    let big = r.height() * 0.5;
    let small = r.height() * 0.32;
    let x = r.center().x - big * 0.55;
    p.add(Shape::line(vec![pos2(x - big * 0.4, base), pos2(x, base - big), pos2(x + big * 0.4, base)], stroke(c)));
    p.line_segment([pos2(x - big * 0.22, base - big * 0.4), pos2(x + big * 0.22, base - big * 0.4)], Stroke::new(1.2, c));
    let x2 = r.center().x + small * 0.9;
    p.add(Shape::line(vec![pos2(x2 - small * 0.4, base), pos2(x2, base - small), pos2(x2 + small * 0.4, base)], stroke(c)));
}

pub fn close(p: &Painter, r: Rect, c: Color32) {
    let s = r.height() * 0.2;
    let m = r.center();
    p.line_segment([m - vec2(s, s), m + vec2(s, s)], stroke(c));
    p.line_segment([m + vec2(-s, s), m + vec2(s, -s)], stroke(c));
}

pub fn book(p: &Painter, r: Rect, c: Color32) {
    let r = Rect::from_center_size(r.center(), vec2(r.height() * 0.8, r.height() * 0.58));
    let m = r.center().x;
    p.add(Shape::closed_line(
        vec![pos2(m, r.top() + 2.0), pos2(r.left(), r.top()), pos2(r.left(), r.bottom()), pos2(m, r.bottom() + 1.0)],
        stroke(c),
    ));
    p.add(Shape::closed_line(
        vec![pos2(m, r.top() + 2.0), pos2(r.right(), r.top()), pos2(r.right(), r.bottom()), pos2(m, r.bottom() + 1.0)],
        stroke(c),
    ));
}

pub fn dot(p: &Painter, center: Pos2, radius: f32, c: Color32) {
    p.circle_filled(center, radius, c);
}
