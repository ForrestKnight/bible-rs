//! First launch (frames 10–11 of the design): a welcome with a choice of translation, then the
//! theme and reading size, in a 960 × 680 window; the reader opens after.
//!
//! The window keeps the light (or, with a dark system appearance, the dark) palette throughout;
//! the theme cards preview the others.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};

use bible_core::library::Library;
use bible_core::model::MarkKind;
use bible_core::{BookId, VerseRef, bundled};

use crate::actions::Action;
use crate::app::Settings;
use crate::icons;
use crate::theme::{self, Palette, Theme, text};
use crate::widgets::{self, css_baseline, flow, line, paint_on_baseline, paint_runs};

/// The onboarding window's size.
pub const SIZE: egui::Vec2 = vec2(960.0, 680.0);

/// Translations offered on the first step, in the design's order.
const OFFERED: [&str; 6] = ["kjv", "web", "bsb", "asv", "ylt", "dby"];

const STEPS: [&str; 3] = ["Translation", "Theme", "Ready"];

const FOOTER: f32 = 72.0;

/// Text shown in the cards, read from the bundled translations on a background thread.
struct Samples {
    /// John 3:16 per offered translation.
    john: Vec<(String, String)>,
    /// Psalm 23's heading and opening verses (BSB), for the theme previews.
    psalm: (String, String),
}

pub struct Onboarding {
    pub step: usize,
    samples: Option<Samples>,
    rx: Option<Receiver<Samples>>,
}

impl Onboarding {
    pub fn new(ctx: &egui::Context) -> Self {
        if crate::offscreen() {
            return Self { step: 0, samples: Some(samples()), rx: None };
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(samples());
            ctx.request_repaint();
        });
        Self { step: 0, samples: None, rx: Some(rx) }
    }

    /// Draws the current step over `ui`'s whole rectangle; returns true when the reader should open.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        settings: &mut Settings,
        actions: &mut Vec<Action>,
    ) -> bool {
        // Past the last designed step, the reader opens.
        if self.step >= 2 {
            return true;
        }
        if let Some(s) = self.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.samples = Some(s);
            self.rx = None;
        }
        let p = palette(ui.ctx());
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, p.paper);
        steps(ui.painter(), p, full, self.step);

        let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOTER), full.max);
        ui.painter().hline(footer.x_range(), footer.top() + 0.5, Stroke::new(1.0, p.separator));
        let mut next = ui.input(|i| i.key_pressed(egui::Key::Enter));
        match self.step {
            0 => {
                self.welcome(ui, p, full, library, settings, actions);
                let hint = "Have a licensed translation (NASB, ESV, NLT…)? You can import it or add an API key \
                            later in Settings.";
                let cy = footer.center().y;
                let icon = Rect::from_center_size(pos2(footer.left() + 32.0 + 8.0, cy), vec2(16.0, 16.0));
                icons::heavy(|| icons::download(ui.painter(), icon, p.ink_2));
                ui.painter().text(
                    pos2(icon.right() + 8.0, cy),
                    Align2::LEFT_CENTER,
                    hint,
                    text::SMALL.font(),
                    p.ink_2,
                );
            }
            _ => {
                self.appearance(ui, p, full, settings, actions);
                let w = widgets::plain_button_width(ui, "Back");
                let r =
                    Rect::from_min_size(pos2(footer.left() + 32.0, footer.center().y - 16.0), vec2(w, 32.0));
                let back = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if widgets::plain_button(ui, p, r, "Back", ui.id().with("back")).clicked() || back {
                    self.step = 0;
                    next = false;
                }
            }
        }
        let w = widgets::primary_button_width(ui, "Continue", true);
        let r = Rect::from_min_size(pos2(footer.right() - 32.0 - w, footer.center().y - 16.0), vec2(w, 32.0));
        if widgets::primary_button(ui, p, r, "Continue", Some(icons::arrow_right), ui.id().with("continue"))
            .clicked()
        {
            next = true;
        }
        if next {
            if self.step == 0 {
                self.step = 1;
            } else {
                return true;
            }
        }
        false
    }

    /// Step 1: the app icon, a welcome, and a card per translation with John 3:16.
    fn welcome(
        &self,
        ui: &mut Ui,
        p: &Palette,
        full: Rect,
        library: &Library,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) {
        let painter = ui.painter().clone();
        let cx = full.center().x;
        let top = full.top();
        app_icon(&painter, Rect::from_center_size(pos2(cx, top + 68.0 + 32.0), vec2(64.0, 64.0)));
        centered(&painter, cx, top + 144.0, 48.0, "Welcome to Bible", display(40.0), p.ink, -0.015);
        centered(
            &painter,
            cx,
            top + 204.0,
            24.0,
            concat!(
                "Just the Bible — fast, beautifully typeset, and kept on ",
                crate::platform::this_computer!(),
                "."
            ),
            theme::ui_font(15.0),
            p.ink_2,
            0.0,
        );

        let left = cx - 416.0;
        let right = cx + 416.0;
        let y = top + 252.0;
        let g = line(&painter, "Choose a translation to start with", theme::ui_semibold(13.0), p.ink, 0.0);
        paint_on_baseline(&painter, left, css_baseline(y, 20.0, 13.0, false), g, p.ink);
        let g = line(
            &painter,
            "Public domain · works offline · more in Settings",
            text::SMALL.font(),
            p.ink_3,
            0.0,
        );
        let x = right - g.size().x;
        paint_on_baseline(&painter, x, css_baseline(y + 2.0, 16.0, 12.0, false), g, p.ink_3);

        let offered: Vec<&str> = OFFERED.iter().copied().filter(|id| library.contains(id)).collect();
        for (i, id) in offered.iter().enumerate() {
            let Some(entry) = library.get(id) else { continue };
            let card = Rect::from_min_size(
                pos2(left + (i % 3) as f32 * (266.0 + 16.0), top + 296.0 + (i / 3) as f32 * (114.0 + 16.0)),
                vec2(266.0, 114.0),
            );
            let resp = ui.interact(card, ui.id().with(("translation", *id)), Sense::click());
            let on = settings.translation == *id;
            if on {
                painter.rect_filled(card.expand(2.0), 14.0, p.accent);
                painter.rect_filled(card, 12.0, p.accent_soft);
            } else {
                painter.add(
                    egui::epaint::RectShape::filled(
                        card.translate(vec2(0.0, 1.0)),
                        12.0,
                        Color32::from_black_alpha(10),
                    )
                    .with_blur_width(4.0),
                );
                let ring = if resp.hovered() { p.ink_3 } else { p.border };
                painter.rect_filled(card.expand(0.5), 12.5, ring);
                painter.rect_filled(card, 12.0, p.surface);
            }
            // Abbreviation, year, and the radio at the right.
            let x = card.left() + 14.0;
            let row = card.top() + 14.0;
            let abbrev_color = if on { p.accent } else { p.ink };
            let g = line(&painter, &entry.meta.abbrev, theme::ui_bold(13.0), abbrev_color, 0.0);
            let ax = x + g.size().x + 6.0;
            paint_on_baseline(&painter, x, css_baseline(row + 1.0, 16.0, 13.0, false), g, abbrev_color);
            if let Some(year) = bundled::year(id) {
                let g = line(&painter, &year.to_string(), theme::ui_medium(11.0), p.ink_3, 0.0);
                paint_on_baseline(&painter, ax, css_baseline(row + 1.0, 16.0, 11.0, false), g, p.ink_3);
            }
            let radio = Rect::from_min_size(pos2(card.right() - 14.0 - 18.0, row), vec2(18.0, 18.0));
            if on {
                painter.circle_filled(radio.center(), 9.0, p.accent);
                icons::check(&painter, Rect::from_center_size(radio.center(), vec2(10.0, 10.0)), p.on_accent);
            } else {
                painter.circle_stroke(radio.center(), 8.5, Stroke::new(1.0, p.border));
            }
            let g = line(&painter, &entry.meta.name, theme::ui_medium(12.0), p.ink_2, 0.0);
            paint_on_baseline(&painter, x, css_baseline(card.top() + 36.0, 16.0, 12.0, false), g, p.ink_2);
            let sample = self.samples.as_ref().and_then(|s| s.john.iter().find(|(i, _)| i == id));
            if let Some((_, verse)) = sample {
                let font = FontId::new(13.0, theme::family(theme::SERIF));
                let lines = flow(&painter, verse, &font, 13.0, 238.0, 2);
                for (k, l) in lines.iter().enumerate() {
                    let base = css_baseline(card.top() + 60.0 + 20.0 * k as f32, 20.0, 13.0, true);
                    paint_runs(&painter, x, base, l, p.ink);
                }
            }
            if resp.clicked() {
                actions.push(Action::SetTranslation(id.to_string()));
            }
        }
    }

    /// Step 2: light, sepia and dark previews of Psalm 23, and the reading size.
    fn appearance(
        &self,
        ui: &mut Ui,
        p: &Palette,
        full: Rect,
        settings: &mut Settings,
        actions: &mut Vec<Action>,
    ) {
        let painter = ui.painter().clone();
        let cx = full.center().x;
        let top = full.top();
        centered(&painter, cx, top + 76.0, 40.0, "How should it read?", display(32.0), p.ink, -0.015);
        centered(
            &painter,
            cx,
            top + 122.0,
            24.0,
            &crate::keymap::shown("You can switch any time with ⇧⌘L."),
            theme::ui_font(15.0),
            p.ink_2,
            0.0,
        );

        let themes = [(Theme::Light, "Light"), (Theme::Sepia, "Sepia"), (Theme::Dark, "Dark")];
        let left = cx - (3.0 * 244.0 + 2.0 * 24.0) / 2.0;
        for (i, (t, label)) in themes.into_iter().enumerate() {
            let card =
                Rect::from_min_size(pos2(left + i as f32 * (244.0 + 24.0), top + 170.0), vec2(244.0, 288.0));
            let radio_row = Rect::from_min_size(pos2(card.left(), card.bottom() + 14.0), vec2(244.0, 16.0));
            let resp = ui.interact(card.union(radio_row), ui.id().with(("theme", i)), Sense::click());
            let on = settings.theme == t;
            if on {
                painter.rect_filled(card.expand(6.0), 20.0, p.focus);
                painter.rect_filled(card.expand(2.0), 16.0, p.accent);
            } else {
                painter.add(
                    egui::epaint::RectShape::filled(
                        card.translate(vec2(0.0, 4.0)),
                        14.0,
                        Color32::from_black_alpha(15),
                    )
                    .with_blur_width(32.0),
                );
                let ring = if resp.hovered() { p.ink_3 } else { p.border };
                painter.rect_filled(card.expand(0.5), 14.5, ring);
            }
            preview(&painter, card, t.palette(), settings.typo.size, self.samples.as_ref());

            // The radio and its label, centred under the card.
            let font = if on { theme::ui_semibold(13.0) } else { theme::ui_medium(13.0) };
            let color = if on { p.accent } else { p.ink };
            let g = line(&painter, label, font, color, 0.0);
            let w = 16.0 + 8.0 + g.size().x;
            let x = card.center().x - w / 2.0;
            widgets::radio(&painter, p, pos2(x + 8.0, radio_row.center().y), on, resp.hovered());
            paint_on_baseline(&painter, x + 24.0, css_baseline(radio_row.top(), 16.0, 13.0, false), g, color);
            if resp.clicked() {
                settings.theme_system = false;
                actions.push(Action::SetTheme(t));
            }
        }

        // Text size: label, small A, slider, large A, value | Match system appearance.
        let y = top + 520.0;
        let cy = y + 12.0;
        let label = line(&painter, "Text size", theme::ui_semibold(13.0), p.ink, 0.0);
        let small = line(&painter, "A", FontId::new(14.0, theme::family(theme::SERIF)), p.ink_2, 0.0);
        let large = line(&painter, "A", FontId::new(22.0, theme::family(theme::SERIF)), p.ink_2, 0.0);
        let system = line(&painter, "Match system appearance", text::UI.font(), p.ink, 0.0);
        let width = label.size().x
            + 16.0
            + small.size().x
            + 12.0
            + 280.0
            + 12.0
            + large.size().x
            + 16.0
            + 44.0
            + 16.0
            + 1.0
            + 16.0
            + 16.0
            + 8.0
            + system.size().x;
        let mut x = cx - width / 2.0;
        let lw = label.size().x;
        paint_on_baseline(&painter, x, css_baseline(y + 2.0, 20.0, 13.0, false), label, p.ink);
        x += lw + 16.0;
        let sw = small.size().x;
        paint_on_baseline(&painter, x, css_baseline(y, 24.0, 14.0, true), small, p.ink_2);
        x += sw + 12.0;
        let slider = Rect::from_min_size(pos2(x, y), vec2(280.0, 24.0));
        widgets::slider(ui, p, slider, ui.id().with("size"), &mut settings.typo.size, 14.0..=32.0, 1.0, 7);
        x += 280.0 + 12.0;
        let bw = large.size().x;
        paint_on_baseline(&painter, x, css_baseline(y, 24.0, 22.0, true), large, p.ink_2);
        x += bw + 16.0;
        let value = line(
            &painter,
            &format!("{} pt", settings.typo.size.round()),
            theme::ui_medium(13.0),
            p.ink_2,
            0.0,
        );
        paint_on_baseline(&painter, x, css_baseline(y + 4.0, 16.0, 13.0, false), value, p.ink_2);
        x += 44.0 + 16.0;
        painter.rect_filled(Rect::from_min_size(pos2(x, cy - 10.0), vec2(1.0, 20.0)), 0.0, p.separator);
        x += 1.0 + 16.0;
        let b = Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0));
        if widgets::checkbox(ui, p, b, settings.theme_system, "Match system appearance").clicked() {
            settings.theme_system = !settings.theme_system;
        }
    }
}

/// The palette the onboarding window itself uses.
fn palette(ctx: &egui::Context) -> &'static Palette {
    if ctx.system_theme() == Some(egui::Theme::Dark) { &theme::DARK } else { &theme::LIGHT }
}

fn display(size: f32) -> FontId {
    FontId::new(size, theme::family(theme::SERIF_DISPLAY))
}

/// One line of text centred on `cx` in a line box `lh` tall at `top`.
#[allow(clippy::too_many_arguments)]
fn centered(
    painter: &Painter,
    cx: f32,
    top: f32,
    lh: f32,
    s: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) {
    let size = font.size;
    let reading = matches!(&font.family, egui::FontFamily::Name(n) if n.starts_with("serif"));
    let g = line(painter, s, font, color, tracking);
    paint_on_baseline(painter, cx - g.size().x / 2.0, css_baseline(top, lh, size, reading), g, color);
}

/// The step indicator, centred in the title bar: numbered dots (a check once done), names, and
/// hairlines between.
fn steps(painter: &Painter, p: &Palette, full: Rect, current: usize) {
    let items: Vec<(Arc<egui::Galley>, Color32)> = STEPS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let (font, color) = if i == current {
                (theme::ui_semibold(12.0), p.ink)
            } else {
                (theme::ui_medium(12.0), p.ink_3)
            };
            (line(painter, name, font, color, 0.0), color)
        })
        .collect();
    let width: f32 =
        items.iter().map(|(g, _)| 20.0 + 8.0 + g.size().x).sum::<f32>() + 2.0 * 32.0 + 4.0 * 10.0;
    let cy = full.top() + 26.0;
    let mut x = full.center().x - width / 2.0;
    for (i, (g, color)) in items.into_iter().enumerate() {
        let dot = Rect::from_min_size(pos2(x, cy - 10.0), vec2(20.0, 20.0));
        if i < current {
            painter.circle_filled(dot.center(), 10.0, p.accent);
            icons::check(painter, Rect::from_center_size(dot.center(), vec2(10.0, 10.0)), p.on_accent);
        } else {
            let (fill, ink) = if i == current { (p.accent, p.on_accent) } else { (p.fill, p.ink_3) };
            painter.circle_filled(dot.center(), 10.0, fill);
            let n = line(painter, &(i + 1).to_string(), theme::ui_semibold(11.0), ink, 0.0);
            let nx = dot.center().x - n.size().x / 2.0;
            paint_on_baseline(painter, nx, css_baseline(cy - 8.0, 16.0, 11.0, false), n, ink);
        }
        x = dot.right() + 8.0;
        let w = g.size().x;
        paint_on_baseline(painter, x, css_baseline(cy - 8.0, 16.0, 12.0, false), g, color);
        x += w + 10.0;
        if i + 1 < STEPS.len() {
            painter.rect_filled(Rect::from_min_size(pos2(x, cy - 0.5), vec2(32.0, 1.0)), 0.0, p.separator);
            x += 32.0 + 10.0;
        }
    }
}

/// The app icon: a blue gradient tile with a soft blue shadow, a white hairline and the book glyph.
pub fn app_icon(painter: &Painter, r: Rect) {
    // The design's icon at 64 pt: radius 14.4, a 32 pt glyph; it scales with its box.
    let k = r.width() / 64.0;
    let radius = 14.4 * k;
    let top = Color32::from_rgb(61, 93, 186);
    let bottom = Color32::from_rgb(36, 57, 122);
    painter.add(
        egui::epaint::RectShape::filled(
            r.translate(vec2(0.0, 8.0)),
            radius,
            Color32::from_rgba_unmultiplied(36, 57, 122, 77),
        )
        .with_blur_width(48.0),
    );
    painter.rect_filled(r, radius, top.lerp_to_gamma(bottom, 0.5));
    // The gradient runs at 160° (CSS): down and slightly right.
    let dir = egui::Vec2::angled(160f32.to_radians() - std::f32::consts::FRAC_PI_2);
    let span = r.width() * dir.x.abs() + r.height() * dir.y.abs();
    let inner = r.shrink(0.5);
    let pts = rounded_rect_points(inner, radius - 0.5);
    let mut mesh = Mesh::default();
    let color_at =
        |q: Pos2| top.lerp_to_gamma(bottom, ((q - r.center()).dot(dir) / span + 0.5).clamp(0.0, 1.0));
    mesh.colored_vertex(inner.center(), color_at(inner.center()));
    for q in &pts {
        mesh.colored_vertex(*q, color_at(*q));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(mesh);
    painter.rect_stroke(
        r.shrink(0.25),
        radius,
        Stroke::new(0.5, Color32::from_white_alpha(64)),
        egui::StrokeKind::Middle,
    );
    icons::book(painter, Rect::from_center_size(r.center(), vec2(32.0 * k, 32.0 * k)), Color32::WHITE);
}

fn rounded_rect_points(r: Rect, radius: f32) -> Vec<Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let corners = [
        (pos2(r.right() - radius, r.top() + radius), -FRAC_PI_2),
        (pos2(r.right() - radius, r.bottom() - radius), 0.0),
        (pos2(r.left() + radius, r.bottom() - radius), FRAC_PI_2),
        (pos2(r.left() + radius, r.top() + radius), PI),
    ];
    let mut pts = Vec::with_capacity(4 * 9);
    for (c, a0) in corners {
        for k in 0..=8 {
            let a = a0 + FRAC_PI_2 * k as f32 / 8.0;
            pts.push(c + vec2(a.cos(), a.sin()) * radius);
        }
    }
    pts
}

/// A theme card: Psalm 23 set in `tp`'s colours at three quarters of the reading size.
fn preview(painter: &Painter, card: Rect, tp: &Palette, size: f32, samples: Option<&Samples>) {
    painter.rect_filled(card, 14.0, tp.paper);
    let painter = painter.with_clip_rect(card);
    let s = size / 20.0;
    let x = card.left() + 24.0;
    let mut y = card.top() + 24.0;
    let g = line(&painter, "PSALM 23", theme::ui_semibold(9.0), tp.accent, 0.12);
    paint_on_baseline(&painter, x, css_baseline(y, 12.0, 9.0, false), g, tp.accent);
    y += 12.0 + 2.0;
    let Some((heading, verses)) = samples.map(|s| &s.psalm) else { return };
    let width = card.width() - 48.0;
    let title = FontId::new((17.0 * s).round(), theme::family(theme::SERIF));
    let title_lh = (28.0 * s).round();
    if !heading.is_empty() {
        // One line, clipped by the card rather than wrapped.
        let small = (13.0 * s).round();
        for l in flow(&painter, heading, &title, small, f32::INFINITY, 1) {
            paint_runs(&painter, x, css_baseline(y, title_lh, title.size, true), &l, tp.ink);
        }
        // CSS aligns the smaller capitals' baseline with the title's, which pushes their line box
        // (and so the row) down.
        let caps = !crate::reader::layout::caps_words(heading).is_empty();
        let push = if caps { crate::reader::layout::LITERATA_BASELINE * (title.size - small) } else { 0.0 };
        y += title_lh + push + 4.0 + 2.0;
    }
    let body = FontId::new((15.0 * s).round(), theme::family(theme::SERIF));
    let pitch = (24.0 * s).round() + 2.0;
    let lh = (20.0 * s).round();
    let rows = ((card.bottom() - 24.0 - y + 2.0) / pitch).floor().max(1.0) as usize;
    for (k, l) in flow(&painter, verses, &body, (11.0 * s).round(), width, rows).iter().enumerate() {
        paint_runs(&painter, x, css_baseline(y + k as f32 * pitch, lh, body.size, true), l, tp.ink);
    }
}

fn samples() -> Samples {
    let john = OFFERED
        .iter()
        .filter_map(|id| {
            let t = bundled::load(id).ok()?;
            let v = t.find(VerseRef::new(BookId(42), 3, 16))?;
            Some((id.to_string(), v.plain_text()))
        })
        .collect();
    let psalm = bundled::load("bsb")
        .ok()
        .map(|t| {
            let mut heading = String::new();
            let mut text = String::new();
            for n in 1..=4 {
                let Some(v) = t.find(VerseRef::new(BookId(18), 23, n)) else { continue };
                if n == 1
                    && let Some(m) = v.marks.iter().find(|m| m.kind == MarkKind::Heading)
                {
                    heading = t.mark_text(m).trim().to_string();
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                // The psalm's opening line keeps a line of its own; the rest runs on.
                let plain = v.plain_text();
                let first_break =
                    v.marks.iter().find(|m| m.kind.breaks_line() && m.offset > 0).map(|m| m.offset);
                match first_break {
                    Some(at) if n == 1 && (at as usize) < plain.len() => {
                        let (a, b) = plain.split_at(at as usize);
                        text.push_str(a.trim_end());
                        text.push_str("\n ");
                        text.push_str(b.trim_start());
                    }
                    _ => text.push_str(&plain),
                }
            }
            (heading, text)
        })
        .unwrap_or_default();
    Samples { john, psalm }
}
