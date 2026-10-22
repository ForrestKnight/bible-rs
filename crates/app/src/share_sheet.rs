//! Share Verse (frame 22): a live image card of the selected verses in one of four styles and
//! three shapes, with options, the text to copy in three formats, and Save Image… / Share…. Away
//! from macOS, which has the system share picker, Share… is Copy Image: the card goes to the
//! clipboard.
//!
//! The image is the card as drawn, read back from a screenshot of the window at its pixel density.

#[cfg(target_os = "macos")]
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32, FontId, Order, Rect, Sense, Stroke, StrokeKind, Ui, UserData, pos2, vec2};
use serde::{Deserialize, Serialize};

use bible_core::model::Style;
use bible_core::{Translation, VerseRef};

use crate::icons;
use crate::pages::text;
use crate::theme::{self, Palette};
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 760.0;
const HEIGHT: f32 = 604.0;
const WELL: f32 = 352.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CardStyle {
    Paper,
    #[default]
    Sepia,
    Night,
    Ink,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Shape {
    Square,
    #[default]
    Portrait,
    Wide,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CopyFormat {
    Plain,
    #[default]
    WithReference,
    Markdown,
}

/// The sheet's choices, kept between uses (in the settings).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShareOptions {
    pub style: CardStyle,
    pub shape: Shape,
    pub reference: bool,
    pub translation: bool,
    pub red_letters: bool,
    pub copy: CopyFormat,
}

impl Default for ShareOptions {
    fn default() -> Self {
        Self {
            style: CardStyle::Sepia,
            shape: Shape::Portrait,
            reference: true,
            translation: true,
            red_letters: false,
            copy: CopyFormat::WithReference,
        }
    }
}

/// A card style's colours: ground, verse, label, footer, brand, words of Christ.
struct Colours {
    ground: Color32,
    ink: Color32,
    label: Color32,
    footer: Color32,
    brand: Color32,
    rubric: Color32,
}

fn colours(style: CardStyle) -> Colours {
    let from = |p: &Palette| Colours {
        ground: p.paper,
        ink: p.ink,
        label: p.accent,
        footer: p.ink_2,
        brand: p.ink_3,
        rubric: p.rubric,
    };
    match style {
        CardStyle::Paper => from(&theme::LIGHT),
        CardStyle::Sepia => from(&theme::SEPIA),
        CardStyle::Night => from(&theme::DARK),
        CardStyle::Ink => Colours {
            ground: theme::LIGHT.accent,
            ink: Color32::WHITE,
            label: Color32::WHITE,
            footer: Color32::from_white_alpha(200),
            brand: Color32::from_white_alpha(160),
            rubric: Color32::from_rgb(255, 196, 186),
        },
    }
}

fn card_size(shape: Shape) -> egui::Vec2 {
    match shape {
        Shape::Square => vec2(300.0, 300.0),
        Shape::Portrait => vec2(300.0, 376.0),
        Shape::Wide => vec2(328.0, 205.0),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Save,
    Share,
}

/// What the sheet asks of the app.
pub enum Outcome {
    Open,
    Close,
    /// Show the system share picker for this image, by the Share button.
    #[cfg(target_os = "macos")]
    Share {
        path: PathBuf,
        anchor: Rect,
    },
    Toast(String),
}

pub struct ShareSheet {
    refs: Vec<VerseRef>,
    /// Where the card was drawn last frame (for the screenshot crop).
    card: Rect,
    #[cfg(target_os = "macos")]
    share_button: Rect,
    pending: Option<Purpose>,
}

impl ShareSheet {
    pub fn new(mut refs: Vec<VerseRef>) -> Self {
        refs.sort();
        refs.dedup();
        Self {
            refs,
            card: Rect::NOTHING,
            #[cfg(target_os = "macos")]
            share_button: Rect::NOTHING,
            pending: None,
        }
    }

    fn reference(&self) -> String {
        match (self.refs.first(), self.refs.last()) {
            (Some(a), Some(b)) => bible_core::reference::format_range(*a, *b),
            _ => String::new(),
        }
    }

    fn copy_text(&self, t: &Translation, format: CopyFormat) -> String {
        let plain: Vec<String> =
            self.refs.iter().filter_map(|r| t.find(*r)).map(|v| v.plain_text()).collect();
        let plain = plain.join(" ");
        let reference = format!("{} ({})", self.reference(), t.meta.abbrev);
        match format {
            CopyFormat::Plain => plain,
            CopyFormat::WithReference => bible_core::copy::format_verses(t, &self.refs)
                .unwrap_or_else(|| format!("“{plain}” — {reference}")),
            CopyFormat::Markdown => format!("> {plain}\n>\n> — {reference}"),
        }
    }

    /// Draws the sheet over a scrim; handles screenshots for the image.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        t: Option<&Arc<Translation>>,
        options: &mut ShareOptions,
    ) -> Outcome {
        // A requested screenshot: crop the card and save or share it.
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, user_data, .. } => {
                    let purpose = user_data.data.as_ref().and_then(|d| d.downcast_ref::<Purpose>()).copied();
                    Some((image.clone(), purpose))
                }
                _ => None,
            })
        });
        if let Some((image, Some(purpose))) = shot
            && self.pending == Some(purpose)
        {
            self.pending = None;
            let ppp = ctx.pixels_per_point();
            #[cfg(not(target_os = "macos"))]
            if purpose == Purpose::Share {
                let Some(card) = crop(&image, self.card, ppp) else {
                    return Outcome::Toast("Couldn't make the image".into());
                };
                let size = [card.width() as usize, card.height() as usize];
                ctx.copy_image(egui::ColorImage::from_rgba_unmultiplied(size, card.as_raw()));
                return Outcome::Toast("Copied the image".into());
            }
            let png = crop_png(&image, self.card, ppp);
            let name = format!("{}.png", self.reference().replace(':', "."));
            match (png, purpose) {
                (Some(png), Purpose::Save) => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&name)
                        .add_filter("PNG image", &["png"])
                        .save_file()
                    {
                        return match std::fs::write(&path, png) {
                            Ok(()) => Outcome::Toast(format!("Saved {}", path.display())),
                            Err(e) => Outcome::Toast(format!("Couldn't save the image: {e}")),
                        };
                    }
                }
                #[cfg(target_os = "macos")]
                (Some(png), Purpose::Share) => {
                    let path = std::env::temp_dir().join(name);
                    if std::fs::write(&path, png).is_ok() {
                        return Outcome::Share { path, anchor: self.share_button };
                    }
                }
                (None, _) => return Outcome::Toast("Couldn't make the image".into()),
                #[cfg(not(target_os = "macos"))]
                (Some(_), Purpose::Share) => {}
            }
        }

        let screen = ctx.content_rect();
        let mut close = false;
        egui::Area::new(egui::Id::new("share-scrim")).order(Order::Middle).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(r, 0.0, p.scrim);
                close |= resp.clicked();
            },
        );
        let at = pos2((screen.center().x - WIDTH / 2.0).round(), 72.0);
        let mut outcome = Outcome::Open;
        egui::Area::new(egui::Id::new("share-sheet")).order(Order::Foreground).fixed_pos(at).show(
            ctx,
            |ui| {
                outcome = self.sheet(ui, p, at, t, options);
            },
        );
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Close;
        }
        outcome
    }

    fn sheet(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        at: egui::Pos2,
        t: Option<&Arc<Translation>>,
        o: &mut ShareOptions,
    ) -> Outcome {
        let sheet = Rect::from_min_size(at, vec2(WIDTH, HEIGHT));
        ui.allocate_rect(sheet, Sense::hover());
        let painter = ui.painter().clone();
        p.shadows.modal.paint(&painter, sheet, 16.0);
        painter.rect_filled(sheet, 16.0, p.surface);
        let x = sheet.left() + 24.0;
        text(
            &painter,
            x,
            sheet.top() + 16.0,
            20.0,
            &format!("Share {}", self.reference()),
            theme::ui_semibold(15.0),
            p.ink,
            false,
        );
        painter.hline(sheet.x_range(), sheet.top() + 51.5, Stroke::new(1.0, p.separator));

        // The preview: the card centred in a well.
        let well = Rect::from_min_size(pos2(x, sheet.top() + 76.0), vec2(WELL, 440.0));
        painter.rect_filled(well, 14.0, p.fill);
        let size = card_size(o.shape);
        self.card = Rect::from_center_size(well.center(), size);
        if let Some(t) = t {
            draw_card(&painter, self.card, t, &self.refs, &self.reference(), o);
        }

        // Options.
        let ox = well.right() + 24.0;
        let ow = sheet.right() - 24.0 - ox;
        let mut y = well.top();
        text(&painter, ox, y, 16.0, "Style", theme::ui_semibold(12.0), p.ink_2, false);
        y += 24.0;
        for (i, (style, label)) in [
            (CardStyle::Paper, "Paper"),
            (CardStyle::Sepia, "Sepia"),
            (CardStyle::Night, "Night"),
            (CardStyle::Ink, "Ink"),
        ]
        .into_iter()
        .enumerate()
        {
            let tile = Rect::from_min_size(pos2(ox + 86.0 * i as f32, y), vec2(72.0, 56.0));
            let resp = ui.interact(
                tile.union(tile.translate(vec2(0.0, 22.0))),
                ui.id().with(("style", i)),
                Sense::click(),
            );
            let on = o.style == style;
            let c = colours(style);
            if on {
                painter.rect_filled(tile.expand(5.0), 15.0, p.focus);
                painter.rect_filled(tile.expand(2.0), 12.0, p.accent);
            } else {
                let ring = if resp.hovered() { p.ink_3 } else { p.border };
                painter.rect_stroke(tile, 10.0, Stroke::new(0.5, ring), StrokeKind::Outside);
            }
            painter.rect_filled(tile, 10.0, c.ground);
            for (k, w) in [40.0, 52.0, 32.0].into_iter().enumerate() {
                painter.rect_filled(
                    Rect::from_min_size(
                        pos2(tile.left() + 12.0, tile.top() + 12.0 + 8.0 * k as f32),
                        vec2(w, 3.0),
                    ),
                    2.0,
                    c.ink.gamma_multiply(0.6),
                );
            }
            let (font, ink) =
                if on { (theme::ui_semibold(12.0), p.accent) } else { (theme::ui_medium(12.0), p.ink_2) };
            let g = widgets::line(&painter, label, font, ink, 0.0);
            let gx = tile.center().x - g.size().x / 2.0;
            widgets::paint_on_baseline(
                &painter,
                gx,
                widgets::css_baseline(tile.bottom() + 6.0, 16.0, 12.0, false),
                g,
                ink,
            );
            if resp.clicked() {
                o.style = style;
            }
        }
        y += 78.0 + 18.0;
        text(&painter, ox, y, 16.0, "Shape", theme::ui_semibold(12.0), p.ink_2, false);
        y += 24.0;
        if let Some(i) = full_segmented(
            ui,
            p,
            Rect::from_min_size(pos2(ox, y), vec2(ow, 32.0)),
            "shape",
            &["Square", "Portrait", "Wide"],
            o.shape as usize,
        ) {
            o.shape = [Shape::Square, Shape::Portrait, Shape::Wide][i];
        }
        y += 32.0 + 18.0;
        for (label, value, id) in [
            ("Include reference", &mut o.reference, "ref"),
            ("Include translation", &mut o.translation, "tr"),
            ("Red letters", &mut o.red_letters, "red"),
        ] {
            text(&painter, ox, y + 2.0, 20.0, label, theme::ui_font(13.0), p.ink, false);
            widgets::toggle(ui, p, pos2(ox + ow, y + 12.0), ui.id().with(("share-opt", id)), value);
            y += 34.0;
        }
        y += 8.0;
        text(&painter, ox, y, 16.0, "Copy as text", theme::ui_semibold(12.0), p.ink_2, false);
        y += 24.0;
        if let Some(i) = full_segmented(
            ui,
            p,
            Rect::from_min_size(pos2(ox, y), vec2(ow, 32.0)),
            "copy",
            &["Plain", "With reference", "Markdown"],
            o.copy as usize,
        ) {
            o.copy = [CopyFormat::Plain, CopyFormat::WithReference, CopyFormat::Markdown][i];
        }
        y += 32.0 + 8.0;
        let preview = Rect::from_min_size(pos2(ox, y), vec2(ow, 52.0));
        painter.rect_filled(preview, 8.0, p.fill);
        let copy = t.map(|t| self.copy_text(t, o.copy)).unwrap_or_default();
        let shown = preview_text(&painter, &copy, ow - 24.0);
        let mut job = LayoutJob::simple(shown, FontId::monospace(12.0), p.ink_2, ow - 24.0);
        job.sections[0].format.line_height = Some(16.0);
        job.wrap.max_rows = 2;
        job.wrap.overflow_character = Some('…');
        painter.galley(pos2(ox + 12.0, y + 10.0), painter.layout_job(job), p.ink_2);

        // Footer.
        let footer = Rect::from_min_max(pos2(sheet.left(), sheet.bottom() - 64.0), sheet.max);
        painter.rect_filled(footer, egui::CornerRadius { nw: 0, ne: 0, sw: 16, se: 16 }, p.fill);
        painter.hline(footer.x_range(), footer.top() + 0.5, Stroke::new(1.0, p.separator));
        let cy = footer.center().y;
        let (_, copy_resp) = Button::new(Kind::Plain, "Copy Text").icon(icons::copy).show_left(
            ui,
            p,
            x,
            cy,
            ui.id().with("copy-text"),
        );
        #[cfg(target_os = "macos")]
        let share_label = Button::new(Kind::Primary, "Share…").icon(icons::share);
        #[cfg(not(target_os = "macos"))]
        let share_label = Button::new(Kind::Primary, "Copy Image").icon(icons::copy);
        let (share, share_resp) =
            share_label.show_right(ui, p, sheet.right() - 24.0, cy, ui.id().with("share"));
        #[cfg(target_os = "macos")]
        {
            self.share_button = share;
        }
        let (_, save_resp) = Button::new(Kind::Secondary, "Save Image…").show_right(
            ui,
            p,
            share.left() - 8.0,
            cy,
            ui.id().with("save-image"),
        );
        if copy_resp.clicked() {
            ui.ctx().copy_text(copy);
            return Outcome::Toast("Copied".into());
        }
        for (resp, purpose) in [(save_resp, Purpose::Save), (share_resp, Purpose::Share)] {
            if resp.clicked() {
                self.pending = Some(purpose);
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(UserData::new(purpose)));
            }
        }
        Outcome::Open
    }

    /// A screenshot is on its way (the app keeps repainting until it lands).
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }
}

/// The copied text in two lines: a long quotation is shortened in the middle so the reference
/// after it stays in view.
fn preview_text(painter: &egui::Painter, copy: &str, width: f32) -> String {
    let flat = copy.replace('\n', " ");
    let rows = |s: &str| {
        let job = LayoutJob::simple(s.to_string(), FontId::monospace(12.0), Color32::PLACEHOLDER, width);
        painter.layout_job(job).rows.len()
    };
    if rows(&flat) <= 2 {
        return flat;
    }
    let Some((quote, tail)) = flat.rsplit_once("” — ") else { return flat };
    let words: Vec<&str> = quote.split(' ').collect();
    for n in (1..words.len()).rev() {
        let s = format!("{}…” — {tail}", words[..n].join(" ").trim_end_matches([',', ';', ':']));
        if rows(&s) <= 2 {
            return s;
        }
    }
    flat
}

/// A segmented control whose track spans `r`, segments packed at the left.
fn full_segmented(
    ui: &mut Ui,
    p: &Palette,
    r: Rect,
    id: &str,
    labels: &[&str],
    selected: usize,
) -> Option<usize> {
    let painter = ui.painter().clone();
    painter.rect_filled(r, 16.0, p.fill);
    let mut x = r.left() + 2.0;
    let mut out = None;
    for (i, label) in labels.iter().enumerate() {
        let on = i == selected;
        let font = if on { theme::ui_semibold(13.0) } else { theme::ui_medium(13.0) };
        let w = painter.layout_no_wrap((*label).into(), font.clone(), Color32::PLACEHOLDER).size().x + 20.0;
        let seg = Rect::from_min_size(pos2(x, r.top() + 2.0), vec2(w, 28.0));
        let resp = ui.interact(seg, ui.id().with((id, i)), Sense::click());
        if on {
            p.shadows.knob.paint(&painter, seg, 14.0);
            painter.rect_filled(seg, 14.0, p.raised);
        } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
            painter.rect_filled(seg, 14.0, fill);
        }
        let ink = if on { p.ink } else { p.ink_2 };
        painter.text(seg.center(), egui::Align2::CENTER_CENTER, *label, font, ink);
        if resp.clicked() && !on {
            out = Some(i);
        }
        x += w + 2.0;
    }
    out
}

/// The image card: label, verse (sized to fit), translation and brand.
fn draw_card(
    painter: &egui::Painter,
    card: Rect,
    t: &Translation,
    refs: &[VerseRef],
    reference: &str,
    o: &ShareOptions,
) {
    let c = colours(o.style);
    painter.add(
        egui::epaint::RectShape::filled(card.translate(vec2(0.0, 12.0)), 16.0, Color32::from_black_alpha(46))
            .with_blur_width(64.0),
    );
    painter.rect_stroke(card, 16.0, Stroke::new(0.5, Color32::from_black_alpha(26)), StrokeKind::Outside);
    painter.rect_filled(card, 16.0, c.ground);
    let pad = if card.height() < 260.0 { 24.0 } else { 32.0 };
    let inner = card.shrink(pad);
    let mut top = inner.top();
    if o.reference {
        let g = widgets::line(painter, &reference.to_uppercase(), theme::ui_semibold(11.0), c.label, 0.12);
        widgets::paint_on_baseline(
            painter,
            inner.left(),
            widgets::css_baseline(top, 16.0, 11.0, false),
            g,
            c.label,
        );
        top += 16.0 + 16.0;
    }
    let footer_top = inner.bottom() - 16.0;
    // The verse, as large as fits (21 pt down to 12).
    let room = footer_top - 16.0 - top;
    let mut galley = None;
    let mut size = 21.0f32;
    while size >= 12.0 {
        let g = painter.layout_job(verse_job(t, refs, size, inner.width(), &c, o.red_letters));
        if g.size().y <= room || size <= 12.0 {
            galley = Some(g);
            break;
        }
        size -= 1.0;
    }
    if let Some(g) = galley {
        let lh = (size * 32.0 / 21.0).round();
        let y = top + (lh - size * 1.2) / 2.0 - 2.0;
        painter.galley(pos2(inner.left(), y), g, c.ink);
    }
    if o.translation {
        text(painter, inner.left(), footer_top, 16.0, &t.meta.name, theme::ui_medium(11.0), c.footer, false);
    }
    let bw = text(painter, inner.right(), footer_top, 16.0, "Bible", theme::ui_semibold(11.0), c.brand, true);
    let icon = Rect::from_min_size(pos2(inner.right() - bw - 4.0 - 12.0, footer_top + 2.0), vec2(12.0, 12.0));
    icons::book(painter, icon, c.brand);
}

fn verse_job(t: &Translation, refs: &[VerseRef], size: f32, width: f32, c: &Colours, red: bool) -> LayoutJob {
    let lh = (size * 32.0 / 21.0).round();
    let base = TextFormat {
        // The heading cut: CSS sets this size at its own optical size, which runs narrower.
        font_id: FontId::new(size, theme::family(theme::SERIF_HEADING)),
        color: c.ink,
        line_height: Some(lh),
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    for (n, r) in refs.iter().enumerate() {
        let Some(v) = t.find(*r) else { continue };
        if n > 0 {
            job.append(" ", 0.0, base.clone());
        }
        let text = v.plain_text();
        // Words of Christ in the rubric, when asked for (offsets of plain text match the
        // stored text up to line-break spaces, so spans are applied per character position).
        let red_at = |i: usize| {
            red && v
                .spans
                .iter()
                .any(|s| s.style == Style::WordsOfJesus && (s.start as usize) <= i && i < s.end as usize)
        };
        let mut run = String::new();
        let mut run_red = false;
        for (i, ch) in text.char_indices() {
            let r = red_at(i);
            if r != run_red && !run.is_empty() {
                let f = TextFormat { color: if run_red { c.rubric } else { c.ink }, ..base.clone() };
                job.append(&std::mem::take(&mut run), 0.0, f);
            }
            run_red = r;
            run.push(ch);
        }
        if !run.is_empty() {
            let f = TextFormat { color: if run_red { c.rubric } else { c.ink }, ..base.clone() };
            job.append(&run, 0.0, f);
        }
    }
    job.wrap.max_width = width;
    job
}

/// The card's pixels from a window screenshot, as PNG.
fn crop_png(image: &egui::ColorImage, card: Rect, ppp: f32) -> Option<Vec<u8>> {
    let img = crop(image, card, ppp)?;
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).ok()?;
    Some(out)
}

/// The card's pixels from a window screenshot, with its rounded corners cut out.
fn crop(image: &egui::ColorImage, card: Rect, ppp: f32) -> Option<image::RgbaImage> {
    let [w, h] = image.size;
    let x0 = (card.left() * ppp).round().max(0.0) as usize;
    let y0 = (card.top() * ppp).round().max(0.0) as usize;
    let x1 = ((card.right() * ppp).round() as usize).min(w);
    let y1 = ((card.bottom() * ppp).round() as usize).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    // Rounded corners: outside the card's radius is transparent.
    let radius = 16.0 * ppp;
    let (cw, ch) = (x1 - x0, y1 - y0);
    let mut rgba = Vec::with_capacity(cw * ch * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let px = image.pixels[y * w + x];
            let (lx, ly) = ((x - x0) as f32 + 0.5, (y - y0) as f32 + 0.5);
            let dx = (radius - lx).max(lx - (cw as f32 - radius)).max(0.0);
            let dy = (radius - ly).max(ly - (ch as f32 - radius)).max(0.0);
            let cover = (radius - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            let [r, g, b, a] = px.to_srgba_unmultiplied();
            rgba.extend_from_slice(&[r, g, b, (a as f32 * cover).round() as u8]);
        }
    }
    image::RgbaImage::from_raw(cw as u32, ch as u32, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_cropped_with_round_corners() {
        let image = egui::ColorImage::new([100, 80], vec![Color32::from_rgb(200, 100, 50); 100 * 80]);
        let card = Rect::from_min_size(pos2(10.0, 5.0), vec2(40.0, 50.0));
        let png = crop_png(&image, card, 2.0).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        // 2 px per point, clipped to the image.
        assert_eq!(decoded.dimensions(), (80, 70));
        assert_eq!(decoded.get_pixel(0, 0)[3], 0, "outside the corner radius is transparent");
        assert_eq!(decoded.get_pixel(40, 40).0, [200, 100, 50, 255]);
    }
}
