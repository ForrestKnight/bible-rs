//! The import sheet (frames 13–15): choose or drop a file, watch it convert book by book, then
//! review anything that needs a look before the translation joins the library.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use eframe::egui::{self, Color32, Order, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use bible_core::BookId;
use bible_core::import::{ImportOptions, Imported, Peek, Progress};

use crate::icons;
use crate::pages::text;
use crate::theme::{self, Palette};
use crate::widgets::{self, BadgeKind, Button, Kind};

const WIDTH: f32 = 560.0;
const PAD: f32 = 24.0;
const TITLE_BAR: f32 = 52.0;
const FOOTER: f32 = 64.0;

const FORMATS: &str = "USFM (.usfm, .sfm, folder, .zip) · OSIS XML · Zefania XML · SWORD module";

/// What the sheet asks of the app when it closes.
pub enum Outcome {
    Open,
    Close,
    /// Add this to the library.
    Install(Box<Imported>),
}

enum Stage {
    Choose,
    Converting { progress: Arc<Progress>, rx: Receiver<Result<Imported, String>>, steps: StepTimes },
    Review { result: Result<Box<Imported>, String>, key_tried: Option<String> },
}

/// When each step began, to show how long the finished ones took.
#[derive(Default)]
struct StepTimes {
    started: Vec<Instant>,
}

pub struct ImportSheet {
    stage: Stage,
    path: Option<PathBuf>,
    peek: Option<Result<Peek, String>>,
    abbrev: String,
    name: String,
    key: String,
    /// The sheet is hidden while a conversion finishes in the background.
    pub hidden: bool,
    /// Screenshots: draw the drop zone as if the chosen file were being dragged over it.
    pub demo_drag: bool,
}

impl Default for ImportSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportSheet {
    pub fn new() -> Self {
        Self {
            stage: Stage::Choose,
            path: None,
            peek: None,
            abbrev: String::new(),
            name: String::new(),
            key: String::new(),
            hidden: false,
            demo_drag: false,
        }
    }

    /// A sheet with `path` already chosen (a file dropped on the window, or picked elsewhere).
    pub fn with_path(path: PathBuf) -> Self {
        let mut s = Self::new();
        s.choose(path);
        s
    }

    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn choose(&mut self, path: PathBuf) {
        let peek = bible_core::import::peek(&path).map_err(|e| format!("{e:#}"));
        if let Ok(p) = &peek {
            self.abbrev = p.abbrev.clone();
            self.name = p.name.clone();
        }
        self.peek = Some(peek);
        self.path = Some(path);
    }

    fn start(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return };
        let opts = ImportOptions {
            abbrev: Some(self.abbrev.trim().to_string()).filter(|s| !s.is_empty()),
            name: Some(self.name.trim().to_string()).filter(|s| !s.is_empty()),
            unlock_key: Some(self.key.trim().to_string()).filter(|s| !s.is_empty()),
            ..Default::default()
        };
        let progress = Arc::new(Progress::default());
        let (tx, rx) = channel();
        let p = progress.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = bible_core::import::import_with_progress(&path, &opts, &p).map_err(|e| format!("{e:#}"));
            let _ = tx.send(r);
            ctx.request_repaint();
        });
        self.stage = Stage::Converting { progress, rx, steps: StepTimes { started: vec![Instant::now()] } };
    }

    /// A demo state for screenshots: converting, `books` of 66 done.
    pub fn demo_converting(&mut self, books: u16) {
        let progress = Arc::new(Progress::default());
        progress.step.store(2, Ordering::Relaxed);
        progress.books.store(books, Ordering::Relaxed);
        progress.book.store(books.min(66) as u8, Ordering::Relaxed);
        let (_tx, rx) = channel();
        std::mem::forget(_tx);
        let now = Instant::now();
        let steps = StepTimes {
            started: vec![
                now - std::time::Duration::from_millis(1600),
                now - std::time::Duration::from_millis(1400),
                now - std::time::Duration::from_millis(1350),
            ],
        };
        self.stage = Stage::Converting { progress, rx, steps };
    }

    /// A demo state for screenshots: the review after a failed or partial import.
    pub fn demo_review(&mut self, error: Option<&str>, key: Option<&str>) {
        let result = match error {
            Some(e) => Err(e.to_string()),
            None => {
                let Some(path) = self.path.clone() else { return };
                bible_core::import::import_path(&path, &ImportOptions::default())
                    .map(Box::new)
                    .map_err(|e| format!("{e:#}"))
            }
        };
        if let Some(k) = key {
            self.key = k.to_string();
        }
        self.stage = Stage::Review { result, key_tried: key.map(str::to_string) };
    }

    /// Draws the sheet over a scrim (or, while hidden, only checks on the conversion).
    pub fn show(&mut self, ctx: &egui::Context, p: &Palette) -> Outcome {
        // A finished conversion: straight into the library when nothing needs a look.
        if let Stage::Converting { rx, .. } = &self.stage
            && let Ok(r) = rx.try_recv()
        {
            let key_tried = Some(self.key.trim().to_string()).filter(|k| !k.is_empty());
            match r {
                Ok(imp) if issues(&imp) == 0 => return Outcome::Install(Box::new(imp)),
                r => {
                    self.hidden = false;
                    self.stage = Stage::Review { result: r.map(Box::new), key_tried };
                }
            }
        }
        if self.hidden {
            if matches!(self.stage, Stage::Converting { .. }) {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
            return Outcome::Open;
        }

        let screen = ctx.content_rect();
        let converting = matches!(self.stage, Stage::Converting { .. });
        let mut outcome = Outcome::Open;
        egui::Area::new(egui::Id::new("import-scrim")).order(Order::Middle).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(r, 0.0, p.scrim);
                // Clicking away while converting lets the reader be used; the sheet returns when done.
                if resp.clicked() && converting {
                    self.hidden = true;
                }
            },
        );
        let top = if matches!(self.stage, Stage::Review { .. }) { 56.0 } else { 72.0 };
        let left = (screen.center().x - WIDTH / 2.0).round();
        egui::Area::new(egui::Id::new("import-sheet"))
            .order(Order::Foreground)
            .fixed_pos(pos2(left, top))
            .show(ctx, |ui| {
                outcome = self.sheet(ui, p, pos2(left, top));
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            match self.stage {
                Stage::Converting { .. } => self.hidden = true,
                _ => outcome = Outcome::Close,
            }
        }
        if converting {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        outcome
    }

    fn sheet(&mut self, ui: &mut Ui, p: &Palette, at: egui::Pos2) -> Outcome {
        let body_h = match &self.stage {
            Stage::Choose => 199.0 + 20.0 + 54.0 + 20.0 + 54.0 + 20.0 + 32.0,
            Stage::Converting { .. } => 64.0 + 20.0 + 58.0 + 20.0 + 5.0 * 32.0 + 4.0 * 4.0,
            Stage::Review { result, .. } => self.review_height(ui, result),
        };
        let height = TITLE_BAR + PAD + body_h + PAD + FOOTER;
        let sheet = Rect::from_min_size(at, vec2(WIDTH, height));
        ui.allocate_rect(sheet, Sense::hover());
        let painter = ui.painter().clone();
        p.shadows.modal.paint(&painter, sheet, 16.0);
        painter.rect_filled(sheet, 16.0, p.surface);

        let title = match &self.stage {
            Stage::Choose => "Import a Translation".to_string(),
            Stage::Converting { .. } => format!("Importing {}", self.display_name()),
            Stage::Review { result: Ok(_), .. } => "Import finished with problems".into(),
            Stage::Review { result: Err(_), .. } => "Import didn’t finish".into(),
        };
        let x = sheet.left() + PAD;
        text(&painter, x, sheet.top() + 16.0, 20.0, &title, theme::ui_semibold(15.0), p.ink, false);
        painter.hline(sheet.x_range(), sheet.top() + TITLE_BAR - 0.5, Stroke::new(1.0, p.separator));

        // The footer: `fill`, a hairline, buttons at the right.
        let footer = Rect::from_min_max(pos2(sheet.left(), sheet.bottom() - FOOTER), sheet.max);
        painter.rect_filled(footer, egui::CornerRadius { nw: 0, ne: 0, sw: 16, se: 16 }, p.fill);
        painter.hline(footer.x_range(), footer.top() + 0.5, Stroke::new(1.0, p.separator));

        let body = Rect::from_min_max(
            pos2(x, sheet.top() + TITLE_BAR + PAD),
            pos2(sheet.right() - PAD, footer.top() - PAD),
        );
        match &self.stage {
            Stage::Choose => self.choose_stage(ui, p, body, footer),
            Stage::Converting { .. } => self.converting_stage(ui, p, body, footer),
            Stage::Review { .. } => self.review_stage(ui, p, body, footer),
        }
    }

    fn display_name(&self) -> String {
        let n = self.name.trim();
        if n.is_empty() { self.abbrev.clone() } else { n.to_string() }
    }

    /// Frame 13: the drop zone, names and unlock key.
    fn choose_stage(&mut self, ui: &mut Ui, p: &Palette, body: Rect, footer: Rect) -> Outcome {
        let painter = ui.painter().clone();
        let mut hovered = ui.input(|i| i.raw.hovered_files.first().map(|f| f.path.clone()));
        if self.demo_drag {
            hovered = Some(self.path.clone());
        }
        if let Some(path) = ui.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())) {
            self.choose(path);
        }
        let dragging = hovered.is_some();
        let zone = Rect::from_min_size(body.min, vec2(body.width(), 199.0));
        let (fill, border, disc, title_ink) = if dragging {
            (p.accent_soft, p.accent, p.raised, p.accent)
        } else {
            (p.fill, p.border, p.surface, p.ink)
        };
        painter.rect_filled(zone, 12.0, fill);
        dashed_rect(&painter, zone.shrink(0.75), 12.0, Stroke::new(1.5, border));
        let mut y = zone.top() + 29.5;
        let cx = zone.center().x;
        painter.circle_filled(pos2(cx, y + 28.0), 28.0, disc);
        icons::download(&painter, Rect::from_center_size(pos2(cx, y + 28.0), vec2(28.0, 28.0)), p.accent);
        y += 56.0 + 8.0;
        let file_name = |path: &std::path::Path| path.file_name().map(|n| n.to_string_lossy().into_owned());
        let title = match (&hovered, &self.path) {
            (Some(h), _) => {
                format!("Drop to import “{}”", h.as_deref().and_then(file_name).unwrap_or_default())
            }
            (None, Some(path)) => format!("“{}”", file_name(path).unwrap_or_default()),
            (None, None) => "Drop a Bible file here".into(),
        };
        let g = widgets::line(&painter, &title, theme::ui_semibold(15.0), title_ink, 0.0);
        let gx = cx - g.size().x / 2.0;
        widgets::paint_on_baseline(&painter, gx, widgets::css_baseline(y, 20.0, 15.0, false), g, title_ink);
        y += 20.0 + 8.0;
        let (line, ink) = match (&hovered, &self.peek) {
            (None, Some(Ok(peek))) => (summary(peek), p.ink_2),
            (None, Some(Err(e))) => (e.clone(), p.danger),
            _ => (FORMATS.to_string(), p.ink_2),
        };
        let line = crate::pages::elide(&painter, &line, &theme::ui_font(12.0), zone.width() - 48.0);
        let g = widgets::line(&painter, &line, theme::ui_font(12.0), ink, 0.0);
        let gx = cx - g.size().x / 2.0;
        widgets::paint_on_baseline(&painter, gx, widgets::css_baseline(y, 16.0, 12.0, false), g, ink);
        y += 16.0 + 8.0;
        let label = if self.path.is_some() { "Choose Another…" } else { "Choose File…" };
        let b = Button::new(Kind::Secondary, label).small();
        let w = b.width(&painter);
        let r = Rect::from_min_size(pos2(cx - w / 2.0, y), vec2(w, 24.0));
        if b.show(ui, p, r, ui.id().with("choose")).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Import a translation (USFM, OSIS, Zefania, SWORD)")
                .add_filter("Bible files", &["xml", "osis", "usfm", "sfm", "zip", "conf", "txt"])
                .pick_file()
        {
            self.choose(path);
        }

        // Abbreviation and name.
        let y = zone.bottom() + 20.0;
        let label = |s: &str, x: f32| text(&painter, x, y, 16.0, s, theme::ui_semibold(12.0), p.ink_2, false);
        label("Abbreviation", body.left());
        label("Name", body.left() + 140.0 + 12.0);
        let abbrev = Rect::from_min_size(pos2(body.left(), y + 22.0), vec2(140.0, 32.0));
        widgets::text_field(
            ui,
            p,
            abbrev,
            &mut self.abbrev,
            "ABBR",
            None,
            false,
            false,
            egui::Id::new("import-abbrev"),
        );
        let name = Rect::from_min_max(
            pos2(abbrev.right() + 12.0, abbrev.top()),
            pos2(body.right(), abbrev.bottom()),
        );
        widgets::text_field(
            ui,
            p,
            name,
            &mut self.name,
            "Translation name",
            None,
            false,
            false,
            egui::Id::new("import-name"),
        );

        // The unlock key.
        let y = abbrev.bottom() + 20.0;
        let w = text(
            &painter,
            body.left(),
            y,
            16.0,
            "SWORD unlock key",
            theme::ui_semibold(12.0),
            p.ink_2,
            false,
        );
        let locked = matches!(&self.peek, Some(Ok(peek)) if peek.locked);
        let hint = if locked {
            "this module is locked — enter the key you received"
        } else {
            "optional — for locked modules you purchased"
        };
        text(
            &painter,
            body.left() + w + 8.0,
            y,
            16.0,
            hint,
            theme::ui_font(12.0),
            if locked { p.accent } else { p.ink_3 },
            false,
        );
        let key = Rect::from_min_size(pos2(body.left(), y + 22.0), vec2(body.width(), 32.0));
        widgets::text_field(
            ui,
            p,
            key,
            &mut self.key,
            "XXXX-XXXX-XXXX-XXXX",
            Some(icons::lock),
            true,
            false,
            egui::Id::new("import-key"),
        );

        // Privacy note.
        let y = key.bottom() + 20.0;
        let icon = Rect::from_min_size(pos2(body.left(), y + 9.0), vec2(14.0, 14.0));
        icons::heavy(|| icons::info(&painter, icon, p.ink_3));
        let note = concat!(
            "The text is converted and stored only on ",
            crate::platform::this_computer!(),
            ". Copyrighted translations are never uploaded."
        );
        let font = theme::ui_font(12.0);
        let mut job = egui::text::LayoutJob::simple(note.into(), font, p.ink_3, body.width() - 22.0);
        job.sections[0].format.line_height = Some(16.0);
        let g = painter.layout_job(job);
        let gy = if g.rows.len() > 1 { y } else { y + 8.0 };
        painter.galley(pos2(body.left() + 22.0, gy), g, p.ink_3);

        // Footer: Cancel, Import.
        let cy = footer.center().y;
        let ready = matches!(self.peek, Some(Ok(_))) && !self.abbrev.trim().is_empty();
        let (import, resp) = Button::new(Kind::Primary, "Import").enabled(ready).show_right(
            ui,
            p,
            footer.right() - PAD,
            cy,
            ui.id().with("import"),
        );
        let go = resp.clicked() || (ready && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        let (_, cancel) = Button::new(Kind::Plain, "Cancel").show_right(
            ui,
            p,
            import.left() - 8.0,
            cy,
            ui.id().with("cancel"),
        );
        if go {
            self.start(ui.ctx());
        }
        if cancel.clicked() { Outcome::Close } else { Outcome::Open }
    }

    /// Frame 14: the file, a progress bar and the steps.
    fn converting_stage(&mut self, ui: &mut Ui, p: &Palette, body: Rect, footer: Rect) -> Outcome {
        let painter = ui.painter().clone();
        let Stage::Converting { progress, steps, .. } = &mut self.stage else { return Outcome::Open };
        let step = progress.step() as usize;
        while steps.started.len() <= step {
            steps.started.push(Instant::now());
        }

        // The file card.
        let card = Rect::from_min_size(body.min, vec2(body.width(), 64.0));
        painter.rect_filled(card, 12.0, p.fill);
        let tile = Rect::from_min_size(card.min + vec2(12.0, 12.0), vec2(40.0, 40.0));
        painter.rect_filled(tile, 10.0, p.accent_soft);
        icons::doc(&painter, Rect::from_center_size(tile.center(), vec2(20.0, 20.0)), p.accent);
        let peek = self.peek.as_ref().and_then(|p| p.as_ref().ok());
        let label = peek.map(|k| k.label.clone()).unwrap_or_default();
        let tx = tile.right() + 12.0;
        let badge_w = widgets::badge_width(&painter, BadgeKind::Accent, &self.abbrev);
        let width = card.right() - 12.0 - badge_w - 12.0 - tx;
        let label = crate::pages::elide(&painter, &label, &theme::ui_semibold(13.0), width);
        text(&painter, tx, card.top() + 14.0, 20.0, &label, theme::ui_semibold(13.0), p.ink, false);
        let meta = peek.map(summary).unwrap_or_default();
        let meta = crate::pages::elide(&painter, &meta, &theme::ui_font(12.0), width);
        text(&painter, tx, card.top() + 34.0, 16.0, &meta, theme::ui_font(12.0), p.ink_2, false);
        widgets::badge(
            &painter,
            p,
            card.right() - 12.0 - badge_w,
            card.center().y,
            BadgeKind::Accent,
            &self.abbrev,
        );

        // Progress.
        let y = card.bottom() + 20.0;
        let books = progress.books().min(66);
        let current = progress.book().map(|b| b.name().to_string()).unwrap_or_default();
        let status = match step {
            0 => "Reading the file…",
            1 => "Detecting the format…",
            2 => "Converting books…",
            _ => "Checking the text…",
        };
        text(&painter, body.left(), y, 20.0, status, theme::ui_semibold(13.0), p.ink, false);
        if step >= 2 {
            let count = if current.is_empty() {
                format!("{books} of 66")
            } else {
                format!("{books} of 66 · {current}")
            };
            text(&painter, body.right(), y + 2.0, 16.0, &count, theme::ui_medium(12.0), p.ink_2, true);
        }
        let track = Rect::from_min_size(pos2(body.left(), y + 28.0), vec2(body.width(), 6.0));
        painter.rect_filled(track, 3.0, p.fill_strong);
        let frac = if step < 2 { 0.0 } else { books as f32 / 66.0 };
        let fill = Rect::from_min_size(track.min, vec2((track.width() * frac).floor(), 6.0));
        painter.rect_filled(fill, 3.0, p.accent);
        // Time left from the pace so far.
        let left_text = match steps.started.get(2) {
            Some(t) if (3..66).contains(&books) => {
                let per = t.elapsed().as_secs_f32() / books as f32;
                let secs = (per * (66 - books) as f32).ceil().max(1.0) as u32;
                if secs < 60 {
                    format!("About {secs} {} left", if secs == 1 { "second" } else { "seconds" })
                } else {
                    format!("About {} min left", secs.div_ceil(60))
                }
            }
            _ => "Estimating…".into(),
        };
        text(&painter, body.left(), y + 42.0, 16.0, &left_text, theme::ui_font(12.0), p.ink_3, false);

        // Steps.
        let sword = peek.is_some_and(|k| k.format == bible_core::import::Format::Sword);
        let detected = peek
            .map(|k| {
                k.details
                    .iter()
                    .find_map(|d| d.strip_suffix(" versification").map(str::to_string))
                    .unwrap_or_else(|| k.format.name().to_string())
            })
            .unwrap_or_default();
        let took = |i: usize| -> String {
            match (steps.started.get(i), steps.started.get(i + 1)) {
                (Some(a), Some(b)) => format!("{:.1} s", (*b - *a).as_secs_f32().max(0.1)),
                _ => String::new(),
            }
        };
        let rows: [(&str, String); 5] = [
            (if sword { "Read module" } else { "Read file" }, took(0)),
            (if sword { "Detect versification" } else { "Detect format" }, detected),
            ("Convert books", format!("{books} / 66")),
            ("Check the text", String::new()),
            ("Add to the library", String::new()),
        ];
        let mut y = y + 58.0 + 20.0;
        for (i, (label, value)) in rows.into_iter().enumerate() {
            let cy = y + 16.0;
            let dot = pos2(body.left() + 10.0, cy);
            let state = i.cmp(&step);
            match state {
                std::cmp::Ordering::Less => {
                    painter.circle_filled(dot, 10.0, p.success);
                    icons::check(&painter, Rect::from_center_size(dot, vec2(10.0, 10.0)), p.on_accent);
                }
                std::cmp::Ordering::Equal => {
                    painter.circle_stroke(dot, 9.0, Stroke::new(2.0, p.accent));
                    painter.circle_filled(dot, 4.0, p.accent);
                }
                std::cmp::Ordering::Greater => {
                    painter.circle_stroke(dot, 9.25, Stroke::new(1.5, p.border));
                }
            }
            let (font, ink) = match state {
                std::cmp::Ordering::Equal => (theme::ui_semibold(13.0), p.ink),
                std::cmp::Ordering::Less => (theme::ui_font(13.0), p.ink),
                std::cmp::Ordering::Greater => (theme::ui_font(13.0), p.ink_3),
            };
            text(&painter, body.left() + 32.0, y + 6.0, 20.0, label, font, ink, false);
            let show = state == std::cmp::Ordering::Less || (state == std::cmp::Ordering::Equal && i == 2);
            if show && !value.is_empty() {
                text(&painter, body.right(), y + 8.0, 16.0, &value, theme::ui_font(12.0), p.ink_3, true);
            }
            y += 36.0;
        }

        // Footer.
        let cy = footer.center().y;
        text(
            &painter,
            footer.left() + PAD,
            cy - 8.0,
            16.0,
            "You can keep reading while this finishes.",
            theme::ui_font(12.0),
            p.ink_3,
            false,
        );
        let (_, resp) = Button::new(Kind::Plain, "Cancel Import").show_right(
            ui,
            p,
            footer.right() - PAD,
            cy,
            ui.id().with("cancel-import"),
        );
        if resp.clicked() {
            progress.cancel.store(true, Ordering::Relaxed);
            return Outcome::Close;
        }
        Outcome::Open
    }

    fn review_cards(result: &Result<Box<Imported>, String>) -> Vec<Issue> {
        let mut out = Vec::new();
        match result {
            Err(e) if is_lock_error(e) => out.push(Issue::Locked),
            Err(e) => out.push(Issue::Failed(e.clone())),
            Ok(imp) => {
                let missing: Vec<BookId> =
                    (0..66).map(BookId).filter(|b| !imp.translation.has_book(*b)).collect();
                if !missing.is_empty() {
                    out.push(Issue::Missing(missing));
                }
                if !imp.skipped_books.is_empty() {
                    out.push(Issue::Skipped(imp.skipped_books.clone()));
                }
                if !imp.warnings.is_empty() {
                    out.push(Issue::Warnings(imp.warnings.clone()));
                }
            }
        }
        out
    }

    fn review_height(&self, ui: &Ui, result: &Result<Box<Imported>, String>) -> f32 {
        let cards = Self::review_cards(result);
        let w = WIDTH - 2.0 * PAD;
        let h: f32 =
            cards.iter().map(|c| c.height(ui.painter(), w)).sum::<f32>() + 8.0 * (cards.len() as f32 - 1.0);
        44.0 + 20.0 + h
    }

    /// Frame 15: what happened, issue by issue.
    fn review_stage(&mut self, ui: &mut Ui, p: &Palette, body: Rect, footer: Rect) -> Outcome {
        let painter = ui.painter().clone();
        let Stage::Review { result, key_tried } = &self.stage else { return Outcome::Open };
        let cards = Self::review_cards(result);
        let label =
            self.peek.as_ref().and_then(|p| p.as_ref().ok()).map(|k| k.label.clone()).unwrap_or_default();

        // Summary.
        let disc = pos2(body.left() + 22.0, body.top() + 22.0);
        painter.circle_filled(disc, 22.0, p.danger.gamma_multiply(0.12));
        icons::warning(&painter, Rect::from_center_size(disc, vec2(24.0, 24.0)), p.danger);
        let tx = body.left() + 44.0 + 14.0;
        let (headline, sub) = match result {
            Ok(imp) => {
                let n = (0..66).filter(|b| imp.translation.has_book(BookId(*b))).count();
                let k = cards.len();
                (
                    format!("{n} of 66 books imported"),
                    format!(
                        "{label} · {k} {} a look before {} is ready to read.",
                        if k == 1 { "issue needs" } else { "issues need" },
                        imp.translation.meta.abbrev
                    ),
                )
            }
            Err(_) => (
                "Nothing was imported".to_string(),
                format!("{label} · fix the problem below and try again."),
            ),
        };
        text(&painter, tx, body.top() + 3.0, 20.0, &headline, theme::ui_semibold(15.0), p.ink, false);
        let sub = crate::pages::elide(&painter, &sub, &theme::ui_font(12.0), body.right() - tx);
        text(&painter, tx, body.top() + 25.0, 16.0, &sub, theme::ui_font(12.0), p.ink_2, false);

        // Issues.
        let mut y = body.top() + 44.0 + 20.0;
        let failed_key = key_tried.is_some();
        for (i, card) in cards.iter().enumerate() {
            let h = card.height(&painter, body.width());
            let r = Rect::from_min_size(pos2(body.left(), y), vec2(body.width(), h));
            card.paint(ui, p, r, i, &mut self.key, failed_key);
            y += h + 8.0;
        }

        // Footer: Copy Report at the left; the way forward at the right.
        let cy = footer.center().y;
        let (_, copy) = Button::new(Kind::Ghost, "Copy Report").icon(icons::copy).show_left(
            ui,
            p,
            footer.left() + PAD,
            cy,
            ui.id().with("copy-report"),
        );
        if copy.clicked() {
            let report = cards.iter().map(|c| c.report()).collect::<Vec<_>>().join("\n");
            ui.ctx().copy_text(format!("{label}\n{headline}\n{report}"));
        }
        let mut outcome = Outcome::Open;
        match result {
            Ok(_) => {
                let (add, resp) = Button::new(Kind::Primary, "Add to Library").show_right(
                    ui,
                    p,
                    footer.right() - PAD,
                    cy,
                    ui.id().with("add"),
                );
                let add_clicked = resp.clicked();
                let (_, discard) = Button::new(Kind::Secondary, "Discard").show_right(
                    ui,
                    p,
                    add.left() - 8.0,
                    cy,
                    ui.id().with("discard"),
                );
                if discard.clicked() {
                    outcome = Outcome::Close;
                }
                if add_clicked
                    && let Stage::Review { result: Ok(imp), .. } =
                        std::mem::replace(&mut self.stage, Stage::Choose)
                {
                    return Outcome::Install(imp);
                }
            }
            Err(e) => {
                let locked = is_lock_error(e);
                let ready = !locked || !self.key.trim().is_empty();
                let (again, resp) = Button::new(Kind::Primary, "Try Again")
                    .icon(icons::retry)
                    .enabled(ready)
                    .show_right(ui, p, footer.right() - PAD, cy, ui.id().with("again"));
                let again_clicked = resp.clicked();
                let other = if locked { "Cancel" } else { "Choose Another File" };
                let (_, resp) = Button::new(Kind::Secondary, other).show_right(
                    ui,
                    p,
                    again.left() - 8.0,
                    cy,
                    ui.id().with("other"),
                );
                if resp.clicked() {
                    if locked {
                        outcome = Outcome::Close;
                    } else {
                        self.stage = Stage::Choose;
                    }
                }
                if again_clicked {
                    self.start(ui.ctx());
                }
            }
        }
        outcome
    }
}

fn is_lock_error(e: &str) -> bool {
    e.contains("locked") || e.contains("unlock key")
}

/// "SWORD module · zText · 4.1 MB · KJV versification".
fn summary(peek: &Peek) -> String {
    let mut parts = vec![peek.format.name().to_string()];
    parts.extend(peek.details.iter().filter(|d| !d.ends_with("versification")).cloned());
    parts.push(format!("{:.1} MB", peek.bytes as f64 / (1024.0 * 1024.0)));
    parts.extend(peek.details.iter().filter(|d| d.ends_with("versification")).cloned());
    parts.join(" · ")
}

/// How many things a finished import needs looked at.
fn issues(imp: &Imported) -> usize {
    let missing = (0..66).any(|b| !imp.translation.has_book(BookId(b)));
    usize::from(missing) + usize::from(!imp.skipped_books.is_empty()) + usize::from(!imp.warnings.is_empty())
}

/// Books outside the canon by their usual names (USFM codes, OSIS ids).
fn book_name(id: &str) -> String {
    let name = match id.to_ascii_uppercase().as_str() {
        "TOB" => "Tobit",
        "JDT" | "JDTH" => "Judith",
        "WIS" => "Wisdom",
        "SIR" => "Sirach",
        "BAR" => "Baruch",
        "1MA" | "1MACC" => "1 Maccabees",
        "2MA" | "2MACC" => "2 Maccabees",
        "ESG" | "ADDESTH" => "Esther (Greek)",
        "DAG" => "Daniel (Greek)",
        "1ES" | "1ESD" => "1 Esdras",
        "2ES" | "2ESD" => "2 Esdras",
        "MAN" | "PRMAN" => "Prayer of Manasseh",
        "LJE" | "EPJER" => "Letter of Jeremiah",
        "S3Y" | "PRAZAR" => "Prayer of Azariah",
        "SUS" => "Susanna",
        "BEL" => "Bel and the Dragon",
        _ => return id.to_string(),
    };
    name.to_string()
}

/// "A, B and C".
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

enum Issue {
    Locked,
    Failed(String),
    Missing(Vec<BookId>),
    Skipped(Vec<String>),
    Warnings(Vec<String>),
}

impl Issue {
    fn danger(&self) -> bool {
        matches!(self, Issue::Locked | Issue::Failed(_))
    }

    fn title(&self) -> String {
        match self {
            Issue::Locked => "Module is locked".into(),
            Issue::Failed(_) => "Couldn’t read this file".into(),
            Issue::Missing(b) if b.len() == 39 => "No Old Testament in this file".into(),
            Issue::Missing(b) if b.len() == 27 => "No New Testament in this file".into(),
            Issue::Missing(b) => {
                format!("{} {} not in this file", b.len(), if b.len() == 1 { "book" } else { "books" })
            }
            Issue::Skipped(b) => {
                let deutero = b.iter().all(|id| book_name(id) != *id);
                let what = if deutero { "deuterocanonical " } else { "" };
                format!("{} {what}{} skipped", b.len(), if b.len() == 1 { "book" } else { "books" })
            }
            Issue::Warnings(w) => {
                format!("{} {} adjusted", w.len(), if w.len() == 1 { "verse" } else { "verses" })
            }
        }
    }

    fn badge(&self) -> (BadgeKind, &'static str) {
        match self {
            Issue::Locked => (BadgeKind::Danger, "Needs action"),
            Issue::Failed(_) => (BadgeKind::Danger, "Failed"),
            Issue::Missing(_) => (BadgeKind::Outline, "Partial"),
            Issue::Skipped(_) => (BadgeKind::Neutral, "Skipped"),
            Issue::Warnings(_) => (BadgeKind::Outline, "Adjusted"),
        }
    }

    fn body(&self) -> String {
        match self {
            Issue::Locked => "The text is encrypted. Enter the unlock key from your purchase receipt.".into(),
            Issue::Failed(e) => format!("{}.", e.trim_end_matches('.')),
            Issue::Missing(b) => {
                let names: Vec<String> = b.iter().take(4).map(|b| b.name().to_string()).collect();
                let more = b.len().saturating_sub(4);
                if more > 0 {
                    format!("{} and {more} more will be missing.", names.join(", "))
                } else {
                    format!("{} will be missing.", and_list(&names))
                }
            }
            Issue::Skipped(b) => {
                let names: Vec<String> = b.iter().map(|id| book_name(id)).collect();
                if names.len() == 1 {
                    format!("{} isn’t supported and was left out.", names[0])
                } else {
                    format!("{} aren’t supported and were left out.", and_list(&names))
                }
            }
            Issue::Warnings(w) => match w.len() {
                1 => w[0].clone(),
                n => format!("{} — and {} more.", w[0], n - 1),
            },
        }
    }

    fn report(&self) -> String {
        format!("- {}: {}", self.title(), self.body())
    }

    fn body_galley(&self, painter: &egui::Painter, width: f32, ink: Color32) -> Arc<egui::Galley> {
        let mut job =
            egui::text::LayoutJob::simple(self.body(), theme::ui_font(12.0), ink, width - 28.0 - 26.0);
        job.sections[0].format.line_height = Some(16.0);
        painter.layout_job(job)
    }

    fn height(&self, painter: &egui::Painter, width: f32) -> f32 {
        let body = self.body_galley(painter, width, Color32::PLACEHOLDER).size().y;
        let key = if matches!(self, Issue::Locked) { 6.0 + 32.0 + 6.0 + 16.0 } else { 0.0 };
        12.0 + 20.0 + 6.0 + body + key + 12.0
    }

    fn paint(&self, ui: &mut Ui, p: &Palette, r: Rect, i: usize, key: &mut String, failed_key: bool) {
        let painter = ui.painter().clone();
        if self.danger() {
            painter.rect_filled(r, 12.0, p.danger.gamma_multiply(0.07));
            painter.rect_stroke(r, 12.0, Stroke::new(1.0, p.danger.gamma_multiply(0.3)), StrokeKind::Inside);
        } else {
            painter.rect_filled(r, 12.0, p.fill);
        }
        let x = r.left() + 14.0;
        let icon = Rect::from_min_size(pos2(x, r.top() + 14.0), vec2(16.0, 16.0));
        icons::heavy(|| {
            if self.danger() {
                icons::warning(&painter, icon, p.danger);
            } else {
                icons::info(&painter, icon, if i == 1 { p.ink_2 } else { p.ink_3 });
            }
        });
        let tx = x + 26.0;
        let (kind, badge) = self.badge();
        let bw = widgets::badge_width(&painter, kind, badge);
        widgets::badge(&painter, p, r.right() - 14.0 - bw, r.top() + 22.0, kind, badge);
        let title = crate::pages::elide(
            &painter,
            &self.title(),
            &theme::ui_semibold(13.0),
            r.right() - 14.0 - bw - 10.0 - tx,
        );
        text(&painter, tx, r.top() + 12.0, 20.0, &title, theme::ui_semibold(13.0), p.ink, false);
        let g = self.body_galley(&painter, r.width(), p.ink_2);
        let gh = g.size().y;
        painter.galley(pos2(tx, r.top() + 12.0 + 20.0 + 6.0), g, p.ink_2);
        if matches!(self, Issue::Locked) {
            let field = Rect::from_min_size(pos2(tx, r.top() + 38.0 + gh + 6.0), vec2(300.0, 32.0));
            let error = failed_key;
            widgets::text_field(
                ui,
                p,
                field,
                key,
                "XXXX-XXXX-XXXX-XXXX",
                Some(icons::lock),
                true,
                error,
                egui::Id::new("import-key"),
            );
            if error {
                let y = field.bottom() + 6.0;
                let w = Rect::from_min_size(pos2(tx, y + 2.0), vec2(12.0, 12.0));
                icons::warning(&painter, w, p.danger);
                text(
                    &painter,
                    tx + 18.0,
                    y,
                    16.0,
                    "That key didn’t unlock the module",
                    theme::ui_font(12.0),
                    p.danger,
                    false,
                );
            }
        }
    }
}

/// A rounded rectangle outlined in 3 pt dashes with 1.5 pt gaps.
fn dashed_rect(painter: &egui::Painter, r: Rect, radius: f32, stroke: Stroke) {
    let pts = rounded_points(r, radius);
    let mut path: Vec<egui::Pos2> = pts.clone();
    path.push(pts[0]);
    let shapes = egui::Shape::dashed_line(&path, stroke, 3.0, 1.5);
    painter.extend(shapes);
}

fn rounded_points(r: Rect, radius: f32) -> Vec<egui::Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let corners = [
        (pos2(r.right() - radius, r.top() + radius), -FRAC_PI_2),
        (pos2(r.right() - radius, r.bottom() - radius), 0.0),
        (pos2(r.left() + radius, r.bottom() - radius), FRAC_PI_2),
        (pos2(r.left() + radius, r.top() + radius), PI),
    ];
    let mut pts = Vec::new();
    for (c, a0) in corners {
        for k in 0..=6 {
            let a = a0 + FRAC_PI_2 * k as f32 / 6.0;
            pts.push(c + vec2(a.cos(), a.sin()) * radius);
        }
    }
    pts
}
