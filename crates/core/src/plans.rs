//! Reading plans: a catalogue of day-by-day readings, generated from the canon's chapter lists.
//! Progress (which plans are under way, which passages are read) lives in the study store.

use std::sync::OnceLock;

use crate::books::{BookId, ChapterIdx};

/// One reading: a whole chapter, or verses of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Passage {
    pub chapter: ChapterIdx,
    pub verses: Option<(u8, u8)>,
}

impl Passage {
    fn whole(chapter: ChapterIdx) -> Self {
        Self { chapter, verses: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanIcon {
    Calendar,
    Book,
    Quote,
    History,
    Flame,
    Note,
}

#[derive(Debug)]
pub struct Plan {
    pub id: &'static str,
    pub title: &'static str,
    /// Length and pace ("90 days · ~3 chapters a day").
    pub meta: &'static str,
    pub description: &'static str,
    pub icon: PlanIcon,
    pub days: Vec<Vec<Passage>>,
}

impl Plan {
    pub fn len(&self) -> usize {
        self.days.len()
    }

    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// The passages for day `n` (1-based).
    pub fn day(&self, n: usize) -> &[Passage] {
        n.checked_sub(1).and_then(|i| self.days.get(i)).map_or(&[], Vec::as_slice)
    }
}

fn book_chapters(books: impl IntoIterator<Item = u8>) -> Vec<ChapterIdx> {
    books
        .into_iter()
        .flat_map(|b| {
            let b = BookId(b);
            let first = b.first_chapter();
            (0..b.chapters() as u16).map(move |i| ChapterIdx(first.0 + i))
        })
        .collect()
}

/// Chapters spread evenly over `days` days, in order.
fn spread(chapters: &[ChapterIdx], days: usize) -> Vec<Vec<Passage>> {
    let n = chapters.len();
    (0..days)
        .map(|d| chapters[d * n / days..(d + 1) * n / days].iter().map(|c| Passage::whole(*c)).collect())
        .collect()
}

/// Several streams read side by side, each spread over the same days.
fn streams(parts: &[Vec<ChapterIdx>], days: usize) -> Vec<Vec<Passage>> {
    let spread: Vec<_> = parts.iter().map(|p| spread(p, days)).collect();
    (0..days).map(|d| spread.iter().flat_map(|s| s[d].iter().copied()).collect()).collect()
}

const PSALMS: u8 = 18;
const PROVERBS: u8 = 19;
const MATTHEW: u8 = 39;
const JOHN: u8 = 42;

fn build() -> Vec<Plan> {
    let gospels = book_chapters(MATTHEW..=JOHN);
    let nt = book_chapters(MATTHEW..66);
    let ot_without_psalms = book_chapters((0..MATTHEW).filter(|b| *b != PSALMS));
    let psalms = book_chapters([PSALMS]);
    let twice = |v: &Vec<ChapterIdx>| v.iter().chain(v.iter()).copied().collect::<Vec<_>>();
    let ps = |n: u16| Passage::whole(ChapterIdx(BookId(PSALMS).first_chapter().0 + n - 1));
    let mt = |c: u8, a: u8, b: u8| Passage {
        chapter: ChapterIdx::new(BookId(MATTHEW), c).expect("Matthew"),
        verses: Some((a, b)),
    };
    vec![
        Plan {
            id: "gospels-30",
            title: "Gospels in 30 Days",
            meta: "30 days · ~3 chapters a day",
            description: "Matthew, Mark, Luke and John — about 3 chapters a day.",
            icon: PlanIcon::Book,
            days: spread(&gospels, 30),
        },
        Plan {
            id: "year-streams",
            title: "Through the Bible in a Year",
            meta: "365 days · ~4 chapters a day",
            description: "Old Testament once, New Testament and Psalms twice.",
            icon: PlanIcon::Calendar,
            days: streams(&[ot_without_psalms, twice(&nt), twice(&psalms)], 365),
        },
        Plan {
            id: "nt-90",
            title: "New Testament in 90 Days",
            meta: "90 days · ~3 chapters a day",
            description: "Matthew to Revelation at a steady pace.",
            icon: PlanIcon::Book,
            days: spread(&nt, 90),
        },
        Plan {
            id: "psalms-proverbs",
            title: "Psalms & Proverbs",
            meta: "31 days · 5 psalms + 1 proverb",
            description: "A month of prayer and wisdom, every evening.",
            icon: PlanIcon::Quote,
            days: (1..=31u16)
                .map(|d| {
                    let mut day: Vec<Passage> =
                        (0..5).map(|k| d + 30 * k).filter(|n| *n <= 150).map(ps).collect();
                    day.push(Passage::whole(ChapterIdx::new(BookId(PROVERBS), d as u8).expect("Proverbs")));
                    day
                })
                .collect(),
        },
        Plan {
            id: "bible-year",
            title: "Bible in a Year",
            meta: "365 days · ~3 chapters a day",
            description: "Genesis to Revelation, in order.",
            icon: PlanIcon::History,
            days: spread(&ChapterIdx::all().collect::<Vec<_>>(), 365),
        },
        Plan {
            id: "sermon-mount",
            title: "Sermon on the Mount",
            meta: "7 days · Matthew 5–7",
            description: "A week with the longest teaching of Jesus.",
            icon: PlanIcon::Flame,
            days: vec![
                vec![mt(5, 1, 16)],
                vec![mt(5, 17, 32)],
                vec![mt(5, 33, 48)],
                vec![mt(6, 1, 18)],
                vec![mt(6, 19, 34)],
                vec![mt(7, 1, 14)],
                vec![mt(7, 15, 29)],
            ],
        },
        Plan {
            id: "john-21",
            title: "Gospel of John",
            meta: "21 days · a chapter a day",
            description: "Slow reading, one chapter at a time.",
            icon: PlanIcon::Note,
            days: spread(&book_chapters([JOHN]), 21),
        },
    ]
}

/// Every plan, in the order they're offered.
pub fn catalogue() -> &'static [Plan] {
    static PLANS: OnceLock<Vec<Plan>> = OnceLock::new();
    PLANS.get_or_init(build)
}

pub fn get(id: &str) -> Option<&'static Plan> {
    catalogue().iter().find(|p| p.id == id)
}

/// A day's passages as a short label: "Luke 5–6", "Psalms 1, 31", "Matthew 5:1–16".
pub fn label(passages: &[Passage], name: impl Fn(BookId) -> String) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < passages.len() {
        let p = passages[i];
        let book = p.chapter.book();
        if let Some((a, b)) = p.verses {
            parts.push(format!("{} {}:{a}–{b}", name(book), p.chapter.chapter()));
            i += 1;
            continue;
        }
        // A run of whole, consecutive chapters of one book.
        let mut j = i;
        while j + 1 < passages.len()
            && passages[j + 1].verses.is_none()
            && passages[j + 1].chapter.book() == book
            && passages[j + 1].chapter.0 == passages[j].chapter.0 + 1
        {
            j += 1;
        }
        let first = p.chapter.chapter();
        let last = passages[j].chapter.chapter();
        let range = if j > i { format!("{first}–{last}") } else { first.to_string() };
        match parts.last_mut() {
            // Further runs of the same book join the previous part: "Psalms 1, 31".
            Some(prev)
                if i > 0 && passages[i - 1].chapter.book() == book && passages[i - 1].verses.is_none() =>
            {
                prev.push_str(&format!(", {range}"));
            }
            _ => parts.push(format!("{} {range}", name(book))),
        }
        i = j + 1;
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_cover_what_they_say() {
        let gospels = get("gospels-30").unwrap();
        assert_eq!(gospels.len(), 30);
        assert_eq!(gospels.days.iter().map(Vec::len).sum::<usize>(), 89);
        let year = get("year-streams").unwrap();
        assert_eq!(year.len(), 365);
        assert_eq!(year.days.iter().map(Vec::len).sum::<usize>(), 779 + 2 * 260 + 2 * 150);
        assert_eq!(get("bible-year").unwrap().days.iter().map(Vec::len).sum::<usize>(), 1189);
        let pp = get("psalms-proverbs").unwrap();
        assert_eq!(pp.day(1).len(), 6);
        assert_eq!(pp.day(31).len(), 5);
        let name = |b: BookId| b.name().to_string();
        assert_eq!(label(pp.day(1), name), "Psalms 1, 31, 61, 91, 121 · Proverbs 1");
        assert_eq!(label(get("sermon-mount").unwrap().day(1), name), "Matthew 5:1–16");
        let luke5 = ChapterIdx::new(BookId(41), 5).unwrap();
        let two = [Passage::whole(luke5), Passage::whole(ChapterIdx(luke5.0 + 1))];
        assert_eq!(label(&two, name), "Luke 5–6");
    }
}
