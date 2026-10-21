//! Reading history: how long each chapter was read and how far, saved as the reader moves on.
//!
//! Time counts while the window has focus, between frames up to a couple of minutes apart (egui
//! only draws when something changes, and reading a screenful changes nothing). A session shorter
//! than a few seconds — paging past a chapter — isn't kept.

use bible_core::books::ChapterIdx;
use bible_core::library::Library;
use bible_core::study::{ReadingSession, StudyStore, now};

/// Sessions shorter than this aren't history.
const MIN_SECONDS: f64 = 8.0;
/// Gaps between frames longer than this aren't counted as reading.
const MAX_GAP: f64 = 120.0;
/// Save an open session this often.
const SAVE_EVERY: f64 = 20.0;

#[derive(Default)]
pub struct Tracker {
    session: Option<ReadingSession>,
    seconds: f64,
    last: Option<f64>,
    saved_at: f64,
}

impl Tracker {
    /// Called once a frame with where the reader is (`furthest`: the furthest verse in view).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        time: f64,
        focused: bool,
        chapter: ChapterIdx,
        furthest: u8,
        translation: &str,
        library: &Library,
        study: &mut StudyStore,
    ) {
        let same = self
            .session
            .as_ref()
            .is_some_and(|s| s.chapter_idx() == Some(chapter) && s.translation == translation);
        if !same {
            self.finish(study);
            let verses = library
                .load(translation)
                .ok()
                .and_then(|t| t.verses_in(chapter).map(|v| v.number_end.max(v.number)).max())
                .unwrap_or(0);
            self.session = Some(ReadingSession {
                started: now(),
                book: chapter.book(),
                chapter: chapter.chapter(),
                translation: translation.to_string(),
                seconds: 0,
                furthest: 0,
                verses,
            });
            self.seconds = 0.0;
            self.saved_at = time;
        }
        if let Some(last) = self.last
            && focused
        {
            let gap = time - last;
            if gap > 0.0 && gap < MAX_GAP {
                self.seconds += gap;
            }
        }
        self.last = Some(time);
        if let Some(s) = &mut self.session {
            s.furthest = s.furthest.max(furthest);
            s.seconds = self.seconds as u32;
        }
        if time - self.saved_at > SAVE_EVERY {
            self.save(study);
            self.saved_at = time;
        }
    }

    fn save(&self, study: &mut StudyStore) {
        if let Some(s) = &self.session
            && self.seconds >= MIN_SECONDS
        {
            let _ = study.record_reading(s);
        }
    }

    /// Saves the open session (on leaving a chapter, and on quit).
    pub fn finish(&mut self, study: &mut StudyStore) {
        self.save(study);
        self.session = None;
    }
}
