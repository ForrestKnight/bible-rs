//! `BIBLE_RS_SIDEBAR_PROBE=<toggles>`: opens and closes the sidebar on a timer, records how the
//! frames around each change were paced and what they cost, then prints the numbers as JSON and
//! quits (`scripts/bench-sidebar.sh`).
//!
//! The toggle goes through the same action as the toolbar button and ⌘B. A frame belongs to a
//! toggle while it starts within [`WINDOW`] of it; frames outside every window are counted as
//! idle frames, which shows whether the app stops drawing once a change is over.

use std::time::{Duration, Instant};

/// Longer than any sidebar transition.
const WINDOW: Duration = Duration::from_millis(450);
/// From one toggle to the next.
const PERIOD: Duration = Duration::from_millis(900);
/// Before the first toggle: startup and the first chapters' layout are done by then.
const SETTLE: Duration = Duration::from_millis(1500);

pub struct SidebarProbe {
    toggles_left: u32,
    next: Instant,
    /// When the current window's toggle happened.
    toggled: Option<Instant>,
    /// When the previous frame inside the current window started.
    last: Option<Instant>,
    /// Chapter layouts counted at the previous frame's end.
    layouts_seen: u64,
    ui_ms: Vec<f64>,
    intervals: Vec<f64>,
    frames_per_toggle: Vec<u32>,
    layouts_per_toggle: Vec<u64>,
    idle_frames: u32,
}

impl SidebarProbe {
    pub fn from_env() -> Option<Self> {
        let toggles = std::env::var("BIBLE_RS_SIDEBAR_PROBE").ok()?.parse().unwrap_or(8);
        Some(Self {
            toggles_left: toggles,
            next: Instant::now() + SETTLE,
            toggled: None,
            last: None,
            layouts_seen: 0,
            ui_ms: Vec::new(),
            intervals: Vec::new(),
            frames_per_toggle: Vec::new(),
            layouts_per_toggle: Vec::new(),
            idle_frames: 0,
        })
    }

    /// At the start of a frame: whether to toggle the sidebar now. Schedules the next wake-up
    /// itself, so it never keeps the app drawing.
    pub fn begin(&mut self, ctx: &eframe::egui::Context, now: Instant) -> bool {
        let toggle = self.toggles_left > 0 && now >= self.next;
        if toggle {
            self.toggles_left -= 1;
            self.next = now + PERIOD;
            self.toggled = Some(now);
            self.last = None;
            self.frames_per_toggle.push(0);
            self.layouts_per_toggle.push(0);
        }
        ctx.request_repaint_after(self.next.saturating_duration_since(now));
        toggle
    }

    /// At the end of the frame that started at `start`, with the reader's running count of
    /// chapter layouts. Returns the report once every toggle's window is over.
    pub fn end(&mut self, start: Instant, layouts: u64) -> Option<String> {
        let new_layouts = layouts - self.layouts_seen.min(layouts);
        self.layouts_seen = layouts;
        match self.toggled.filter(|t| start.duration_since(*t) < WINDOW) {
            Some(_) => {
                self.ui_ms.push(start.elapsed().as_secs_f64() * 1000.0);
                if let Some(last) = self.last {
                    self.intervals.push(start.duration_since(last).as_secs_f64() * 1000.0);
                }
                self.last = Some(start);
                if let (Some(f), Some(l)) =
                    (self.frames_per_toggle.last_mut(), self.layouts_per_toggle.last_mut())
                {
                    *f += 1;
                    *l += new_layouts;
                }
            }
            None if self.toggled.is_some() => self.idle_frames += 1,
            None => {}
        }
        let done = self.toggles_left == 0 && self.toggled.is_some_and(|t| t.elapsed() >= WINDOW);
        done.then(|| self.report())
    }

    fn report(&mut self) -> String {
        let (mean, p50, p95, p99, max) = stats(&mut self.ui_ms);
        let (imean, ip50, ip95, _, imax) = stats(&mut self.intervals);
        // A frame that took over one and a half refresh periods missed at least one.
        let long = self.intervals.iter().filter(|i| **i > ip50 * 1.5).count();
        format!(
            "{{\"toggles\":{},\"frames_per_toggle\":{:?},\"chapter_layouts_per_toggle\":{:?},\"ui_ms\":{{\"mean\":{mean:.3},\"p50\":{p50:.3},\"p95\":{p95:.3},\"p99\":{p99:.3},\"max\":{max:.3}}},\"frame_interval_ms\":{{\"mean\":{imean:.3},\"p50\":{ip50:.3},\"p95\":{ip95:.3},\"max\":{imax:.3}}},\"long_intervals\":{long},\"idle_frames\":{}}}",
            self.frames_per_toggle.len(),
            self.frames_per_toggle,
            self.layouts_per_toggle,
            self.idle_frames,
        )
    }
}

/// Mean, median, 95th and 99th percentiles and maximum (zeros when empty).
fn stats(v: &mut [f64]) -> (f64, f64, f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    v.sort_by(f64::total_cmp);
    let pct = |p: f64| v[((v.len() as f64 - 1.0) * p).round() as usize];
    (v.iter().sum::<f64>() / v.len() as f64, pct(0.5), pct(0.95), pct(0.99), v[v.len() - 1])
}
