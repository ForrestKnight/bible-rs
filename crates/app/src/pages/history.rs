//! Study › History (frame 19): reading sessions by day, newest first, beside a summary of the
//! week with minutes per day.

use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};

use bible_core::study::ReadingSession;

use super::{Empty, PageEnv, Tone, group_label, text};
use crate::actions::Action;
use crate::icons;
use crate::theme::{self, Palette};
use crate::time;
use crate::widgets::{self, BadgeKind, Button, Kind, css_baseline, flow, paint_runs};

const TIMELINE: f32 = 584.0;
const ROW: f32 = 84.0;
const CARD: f32 = 320.0;
const DAY: i64 = 86_400;

pub struct State {
    /// 0 today, 1 the last seven days, 2 everything.
    range: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { range: 1 }
    }
}

fn chapter_title(s: &ReadingSession, env: &PageEnv<'_>) -> String {
    let name = env
        .library
        .get(&s.translation)
        .map(|e| e.meta.ref_name(s.book).to_string())
        .unwrap_or_else(|| s.book.ref_name().to_string());
    format!("{name} {}", s.chapter)
}

impl State {
    pub fn show(&mut self, ui: &mut Ui, content: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) -> f32 {
        let p = env.settings.theme.palette();
        let painter = ui.painter().clone();
        let (left, right, top) = (content.left(), content.right(), content.top());
        let now = bible_core::study::now();
        let today = time::day_start(now);
        super::title(&painter, p, left, top, "History", "Everything you’ve read, newest first");
        let week = env.study.reading_since(today - 6 * DAY);
        if env.study.reading_since(0).is_empty() {
            let empty = Empty {
                glyph: icons::history,
                tone: Tone::Accent,
                title: "Your reading will show up here",
                body: "Each chapter you read is kept here — when, for how long and how far you got. It stays \
                       on this Mac.",
                buttons: &[Button::new(Kind::Primary, "Open John 1")],
                hints: &[],
                width: 360.0,
            };
            return super::bookmarks::empty_page(ui, p, content, &empty, actions);
        }

        // Header actions: the range, and Clear….
        let cy = top + 26.0 + 16.0;
        let (clear, resp) =
            Button::new(Kind::Plain, "Clear…").show_right(ui, p, right, cy, ui.id().with("clear-history"));
        crate::menu::popup(&resp, p).show(|ui| {
            ui.set_min_width(200.0);
            if ui.button("Clear Today").clicked() {
                actions.push(Action::ClearReading(today));
            }
            if ui.button("Clear the Last 7 Days").clicked() {
                actions.push(Action::ClearReading(today - 6 * DAY));
            }
            crate::menu::separator(ui);
            if ui.button("Clear All History").clicked() {
                actions.push(Action::ClearReading(0));
            }
        });
        let (_, pick) = widgets::segmented_tight(
            ui,
            p,
            pos2(clear.left() - 8.0, cy),
            ui.id().with("history-range"),
            &["Today", "Week", "All"],
            self.range,
        );
        if let Some(i) = pick {
            self.range = i;
        }

        // The timeline.
        let since = match self.range {
            0 => today,
            1 => today - 6 * DAY,
            _ => 0,
        };
        let q = env.query.trim().to_lowercase();
        let sessions: Vec<ReadingSession> = env
            .study
            .reading_since(since)
            .into_iter()
            .filter(|s| q.is_empty() || chapter_title(s, env).to_lowercase().contains(&q))
            .collect();
        let body = top + 58.0 + 20.0;
        let card_left = right - CARD;
        let timeline_w = TIMELINE.min(card_left - 32.0 - left);
        let mut y = body;
        let mut day = None;
        let n = sessions.len();
        for (i, s) in sessions.iter().enumerate() {
            let d = time::day_start(s.started);
            if day != Some(d) {
                day = Some(d);
                let l = time::local(s.started);
                let label = if d == today {
                    format!("Today · {}", time::weekday(l.weekday))
                } else if d >= today - 6 * DAY {
                    time::weekday(l.weekday).to_string()
                } else {
                    format!("{} · {}", time::weekday(l.weekday), time::short_date(s.started))
                };
                group_label(&painter, p, left + 60.0, y + 4.0, &label);
                y += 32.0;
            }
            let row = Rect::from_min_size(pos2(left, y), vec2(timeline_w, ROW));
            self.session(ui, p, row, s, env, i + 1 == n, actions);
            y += ROW;
        }
        if sessions.is_empty() {
            text(
                &painter,
                left + 60.0,
                y + 4.0,
                20.0,
                "Nothing read in this range.",
                theme::ui_font(13.0),
                p.ink_3,
                false,
            );
            y += 40.0;
        }

        // The week.
        let card_bottom = self.week_card(ui, p, pos2(card_left, body), &week, today, env);
        y.max(card_bottom)
    }

    #[allow(clippy::too_many_arguments)]
    fn session(
        &self,
        ui: &mut Ui,
        p: &Palette,
        row: Rect,
        s: &ReadingSession,
        env: &PageEnv<'_>,
        last: bool,
        actions: &mut Vec<Action>,
    ) {
        let painter = ui.painter().clone();
        let (x0, y) = (row.left(), row.top());
        let resp = ui.interact(row, ui.id().with(("session", s.started)), Sense::click());
        text(&painter, x0 + 48.0, y, 20.0, &time::clock(s.started), theme::ui_medium(12.0), p.ink_3, true);
        // The rail: a dot, and a line down to the next session.
        painter.circle_filled(pos2(x0 + 68.0, y + 10.5), 4.5, p.accent);
        if !last {
            painter.vline(x0 + 68.0, (y + 19.0)..=(y + ROW), Stroke::new(1.0, p.separator));
        }
        let x = x0 + 88.0;
        let w = row.right() - x;
        let title = chapter_title(s, env);
        let tw = text(&painter, x, y, 20.0, &title, theme::ui_semibold(15.0), p.ink, false);
        let abbrev = env.library.get(&s.translation).map(|e| e.meta.abbrev.clone()).unwrap_or_default();
        if !abbrev.is_empty() {
            widgets::badge(&painter, p, x + tw + 8.0, y + 10.0, BadgeKind::Neutral, &abbrev);
        }
        let mins = time::duration(s.seconds);
        text(&painter, row.right(), y + 2.0, 16.0, &mins, theme::ui_font(12.0), p.ink_2, true);
        // The chapter's opening, in the translation it was read in.
        let snippet = s
            .chapter_idx()
            .and_then(|c| env.library.load(&s.translation).ok().map(|t| (c, t)))
            .and_then(|(c, t)| t.verses_in(c).next().map(|v| v.plain_text()))
            .unwrap_or_default();
        if !snippet.is_empty() {
            let serif = FontId::new(14.0, theme::family(theme::SERIF));
            if let Some(l) = flow(&painter, &snippet, &serif, 10.0, w - 8.0, 1).first() {
                paint_runs(&painter, x, css_baseline(y + 24.0, 20.0, 14.0, true), l, p.ink_2);
            }
        }
        let track = Rect::from_min_size(pos2(x, y + 54.0), vec2(120.0, 4.0));
        painter.rect_filled(track, 2.0, p.fill_strong);
        let progress = if s.finished() { 1.0 } else { s.progress().clamp(0.0, 1.0) };
        painter.rect_filled(Rect::from_min_size(track.min, vec2(120.0 * progress, 4.0)), 2.0, p.accent);
        let status = if s.finished() {
            "Finished".to_string()
        } else {
            format!("{} % of chapter", (progress * 100.0).round())
        };
        text(&painter, x + 128.0, y + 48.0, 16.0, &status, theme::ui_medium(11.0), p.ink_3, false);
        if resp.clicked()
            && let Some(c) = s.chapter_idx()
        {
            actions.push(Action::SetTranslation(s.translation.clone()));
            actions.push(Action::Jump { chapter: c, verse: None, flash: false });
        }
    }

    /// The week card: minutes per day for the last seven days, and totals.
    fn week_card(
        &self,
        ui: &mut Ui,
        p: &Palette,
        at: egui::Pos2,
        week: &[ReadingSession],
        today: i64,
        env: &PageEnv<'_>,
    ) -> f32 {
        let painter = ui.painter().clone();
        let card = Rect::from_min_size(at, vec2(CARD, 326.0));
        p.shadows.panel.paint(&painter, card, 16.0);
        painter.rect_filled(card, 16.0, p.sidebar);
        let x = card.left() + 20.0;
        let y = card.top() + 20.0;
        text(&painter, x, y, 20.0, "This week", theme::ui_semibold(15.0), p.ink, false);
        text(&painter, x, y + 22.0, 16.0, "Minutes read per day", theme::ui_font(12.0), p.ink_2, false);

        // Seven days, oldest first, ending today.
        let days: Vec<(i64, Vec<&ReadingSession>)> = (0..7)
            .rev()
            .map(|k| {
                let d = today - k * DAY;
                (d, week.iter().filter(|s| time::day_start(s.started) == d).collect())
            })
            .collect();
        let minutes: Vec<u32> =
            days.iter().map(|(_, s)| (s.iter().map(|s| s.seconds).sum::<u32>() + 30) / 60).collect();
        let max = minutes.iter().copied().max().unwrap_or(0).max(1) as f32;
        let chart = Rect::from_min_size(pos2(x, y + 54.0), vec2(272.0, 160.0));
        let bottom = chart.top() + 136.0;
        let mut hovered = None;
        for (i, ((d, sessions), m)) in days.iter().zip(&minutes).enumerate() {
            let col =
                Rect::from_min_size(pos2(chart.left() + 40.0 * i as f32, chart.top()), vec2(32.0, 160.0));
            let is_today = *d == today;
            let hover = ui.rect_contains_pointer(col) && *m > 0;
            if hover {
                hovered = Some((i, *d, sessions, *m));
            }
            if *m == 0 {
                painter.rect_filled(
                    Rect::from_min_max(pos2(col.left(), bottom - 2.0), pos2(col.right(), bottom)),
                    1.0,
                    p.fill_strong,
                );
            } else {
                let h = (*m as f32 / max * 106.0).max(4.0);
                let bar = Rect::from_min_max(pos2(col.left(), bottom - h), pos2(col.right(), bottom));
                let alpha = if is_today || hover { 1.0 } else { 0.55 };
                painter.rect_filled(
                    bar,
                    egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 },
                    p.accent.gamma_multiply(alpha),
                );
                if is_today {
                    let label = format!("{m} min");
                    let g = widgets::line(&painter, &label, theme::ui_semibold(11.0), p.ink, 0.0);
                    let gx = col.center().x - g.size().x / 2.0;
                    widgets::paint_on_baseline(
                        &painter,
                        gx,
                        css_baseline(bar.top() - 18.0, 16.0, 11.0, false),
                        g,
                        p.ink,
                    );
                }
            }
            let l = time::local(*d);
            let name = time::weekday_short(l.weekday);
            let (font, ink) =
                if is_today { (theme::ui_semibold(11.0), p.ink) } else { (theme::ui_font(11.0), p.ink_3) };
            let g = widgets::line(&painter, name, font, ink, 0.0);
            let gx = col.center().x - g.size().x / 2.0;
            widgets::paint_on_baseline(
                &painter,
                gx,
                css_baseline(chart.top() + 144.0, 16.0, 11.0, false),
                g,
                ink,
            );
        }
        painter.hline(chart.x_range(), chart.top() + 138.5, Stroke::new(1.0, p.separator));

        // Totals.
        let sy = chart.bottom() + 16.0;
        painter.hline(x..=card.right() - 20.0, sy + 0.5, Stroke::new(1.0, p.separator));
        let total: u32 = week.iter().map(|s| s.seconds).sum();
        let read_days = minutes.iter().filter(|m| **m > 0).count();
        let mut chapters: Vec<_> = week.iter().map(|s| (s.book, s.chapter)).collect();
        chapters.sort();
        chapters.dedup();
        let stats = [
            (time::duration(total), "total"),
            (format!("{read_days} of 7"), "days"),
            (chapters.len().to_string(), "chapters"),
        ];
        let serif = FontId::new(20.0, theme::family(theme::SERIF));
        let widths: Vec<f32> = stats
            .iter()
            .map(|(v, l)| {
                let a = painter.layout_no_wrap(v.clone(), serif.clone(), Color32::PLACEHOLDER).size().x;
                let b = painter
                    .layout_no_wrap((*l).into(), theme::ui_medium(11.0), Color32::PLACEHOLDER)
                    .size()
                    .x;
                a.max(b)
            })
            .collect();
        let spare = ((CARD - 40.0) - widths.iter().sum::<f32>() - 24.0).max(0.0) / 3.0;
        let mut sx = x;
        for ((v, l), w) in stats.iter().zip(widths) {
            let g = widgets::line(&painter, v, serif.clone(), p.ink, 0.0);
            widgets::paint_on_baseline(&painter, sx, css_baseline(sy + 12.0, 28.0, 20.0, true), g, p.ink);
            text(&painter, sx, sy + 40.0, 16.0, l, theme::ui_medium(11.0), p.ink_3, false);
            sx += w + spare + 12.0;
        }

        // The hovered day: when, how long and what.
        if let Some((i, d, sessions, m)) = hovered {
            let l = time::local(d);
            let line1 = format!("{} · {m} min", time::weekday(l.weekday));
            let mut books: Vec<_> = sessions.iter().map(|s| s.book).collect();
            books.dedup();
            let what = if books.len() == 1 {
                let mut cs: Vec<u8> = sessions.iter().map(|s| s.chapter).collect();
                cs.sort_unstable();
                let name = chapter_title(sessions[0], env);
                let book = name.rsplit_once(' ').map_or(name.as_str(), |(b, _)| b).to_string();
                match (cs.first(), cs.last()) {
                    (Some(a), Some(b)) if a != b => format!("{book} {a}–{b}"),
                    _ => name,
                }
            } else {
                format!("{} chapters", sessions.len())
            };
            let n = sessions.len();
            let line2 = format!("{what} · {n} {}", if n == 1 { "session" } else { "sessions" });
            let w1 = painter
                .layout_no_wrap(line1.clone(), theme::ui_semibold(12.0), Color32::PLACEHOLDER)
                .size()
                .x;
            let w2 =
                painter.layout_no_wrap(line2.clone(), theme::ui_font(11.0), Color32::PLACEHOLDER).size().x;
            let tw = w1.max(w2) + 20.0;
            let cx = chart.left() + 40.0 * i as f32 + 16.0;
            let tx = (cx - tw / 2.0).clamp(card.left() + 8.0, card.right() - 8.0 - tw);
            let tip = Rect::from_min_size(pos2(tx, chart.top() - 6.0), vec2(tw, 44.0));
            p.shadows.popover.paint(&painter, tip, 8.0);
            painter.rect_filled(tip, 8.0, p.raised);
            text(
                &painter,
                tip.left() + 10.0,
                tip.top() + 6.0,
                16.0,
                &line1,
                theme::ui_semibold(12.0),
                p.ink,
                false,
            );
            text(
                &painter,
                tip.left() + 10.0,
                tip.top() + 22.0,
                16.0,
                &line2,
                theme::ui_font(11.0),
                p.ink_2,
                false,
            );
        }
        card.bottom()
    }
}
