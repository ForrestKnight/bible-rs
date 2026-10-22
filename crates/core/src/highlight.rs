//! Highlights of any length: a whole verse, a few words of one, or a stretch across several.
//!
//! The store keeps highlights per verse (see [`crate::study`]), each as the characters it covers in
//! that verse's text: `start..end` in chars (Unicode scalar values), `end` left open to run to the
//! verse's end. A stretch across verses is kept as one piece per verse: the tail of the first, the
//! middle verses whole, the head of the last.
//!
//! Offsets only mean something in the text they were counted in, and highlights are shared by every
//! translation. So a highlight of part of a verse also keeps the words it covered (`quote`). In
//! another text it is placed where those words are, the occurrence nearest its offset; when the
//! text doesn't have them (another translation's wording), it falls back to the whole verse, so a
//! highlight never disappears. Whole-verse highlights need neither and fit every translation.
//!
//! Colouring over a highlight replaces it where they overlap (the most recently coloured is on top,
//! as with a highlighter), neighbours of one colour join, and clearing part of a highlight leaves
//! the rest.
//! Edits are made against the text being read: a verse that is edited is stored again in that
//! text's offsets, as it was shown.

use std::ops::Range;

use crate::books::VerseRef;
use crate::study::HighlightColor;

/// One coloured stretch of a verse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Highlight {
    pub color: HighlightColor,
    /// The first character covered, in chars of the verse's text.
    pub start: u32,
    /// The character after the last one covered; `None` runs to the verse's end.
    pub end: Option<u32>,
    /// For part of a verse: the words it covered when made, to find them again in another text.
    pub quote: Option<String>,
    /// When it was first made (Unix seconds): what lists show and sort by.
    pub created: i64,
    /// When it was last coloured: where highlights overlap (placed in another translation), the
    /// most recently coloured is on top.
    pub edited: i64,
}

impl Highlight {
    /// The whole verse in `color`.
    pub fn whole(color: HighlightColor, created: i64) -> Self {
        Highlight { color, start: 0, end: None, quote: None, created, edited: created }
    }

    pub fn is_whole(&self) -> bool {
        self.start == 0 && self.end.is_none()
    }

    /// Where the highlight lies in a verse whose text is `text`, in chars (see the module docs).
    pub fn range_in(&self, text: &str) -> Range<usize> {
        let chars: Vec<char> = text.chars().collect();
        self.place(&chars)
    }

    /// The words it covers in `text`, with an ellipsis where it cuts the verse short.
    pub fn excerpt(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let r = self.place(&chars);
        let full = trim(&chars, 0..chars.len());
        let mut out = String::new();
        if r.start > full.start {
            out.push('…');
        }
        out.extend(&chars[r.clone()]);
        if r.end < full.end {
            out.push('…');
        }
        out
    }

    fn place(&self, chars: &[char]) -> Range<usize> {
        let full = trim(chars, 0..chars.len());
        if self.is_whole() {
            return full;
        }
        let start = self.start as usize;
        let end = self.end.map_or(full.end, |e| e as usize);
        let Some(quote) = &self.quote else {
            // Offsets alone (an import without words): trusted, within the text.
            let r = start.min(chars.len())..end.min(chars.len());
            return if r.is_empty() { full } else { r };
        };
        if end <= chars.len() && start < end && chars[start..end].iter().copied().eq(quote.chars()) {
            return start..end;
        }
        // Elsewhere, or spaced differently (a verse's text has no space at a poetry line break;
        // the plain text shown in lists has one): matched letter for letter, ignoring whitespace.
        let q: Vec<char> = quote.chars().filter(|c| !c.is_whitespace()).collect();
        let letters: Vec<usize> = (0..chars.len()).filter(|&i| !chars[i].is_whitespace()).collect();
        if q.is_empty() || q.len() > letters.len() {
            return full;
        }
        let nearest = (0..=letters.len() - q.len())
            .filter(|&i| letters[i..i + q.len()].iter().map(|&k| chars[k]).eq(q.iter().copied()))
            .min_by_key(|&i| letters[i].abs_diff(start));
        match nearest {
            Some(i) => letters[i]..letters[i + q.len() - 1] + 1,
            None => full,
        }
    }
}

/// A stretch of text in one chapter: from char `start` of verse `first` to just before char `end`
/// of verse `last`, counted in the verse texts of the translation it was chosen in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub first: VerseRef,
    pub start: usize,
    pub last: VerseRef,
    pub end: usize,
}

impl TextSpan {
    /// Whole verses, `first` through `last`.
    pub fn verses(first: VerseRef, last: VerseRef) -> Self {
        TextSpan { first, start: 0, last, end: usize::MAX }
    }

    /// The verses it touches, in order. Spans stay in one chapter: one that doesn't touches only its
    /// first verse.
    pub fn refs(&self) -> Vec<VerseRef> {
        let (a, b) = (self.first, self.last);
        if a.book != b.book || a.chapter != b.chapter || b.verse < a.verse {
            return vec![a];
        }
        (a.verse..=b.verse).map(|n| VerseRef::new(a.book, a.chapter, n)).collect()
    }

    /// The chars it covers of verse `v` (to `usize::MAX` where it runs to the verse's end).
    pub fn range_of(&self, v: VerseRef) -> Range<usize> {
        let start = if v == self.first { self.start } else { 0 };
        let end = if v == self.last { self.end } else { usize::MAX };
        start..end
    }
}

/// A placed highlight: where it lies in a text, and what it was.
#[derive(Debug, Clone, PartialEq)]
struct Piece {
    range: Range<usize>,
    color: HighlightColor,
    created: i64,
    edited: i64,
}

/// `range` without the whitespace at either end.
fn trim(chars: &[char], range: Range<usize>) -> Range<usize> {
    let (mut s, mut e) = (range.start.min(chars.len()), range.end.min(chars.len()));
    while s < e && chars[s].is_whitespace() {
        s += 1;
    }
    while e > s && chars[e - 1].is_whitespace() {
        e -= 1;
    }
    s..e
}

/// The highlights placed in `chars`, layered (the most recently coloured on top where they
/// overlap) and joined.
fn layer(parts: &[Highlight], chars: &[char]) -> Vec<Piece> {
    let mut order: Vec<&Highlight> = parts.iter().collect();
    order.sort_by_key(|h| h.edited);
    let mut pieces: Vec<Piece> = Vec::new();
    for h in order {
        let range = trim(chars, h.place(chars));
        if range.is_empty() {
            continue;
        }
        pieces = cut(pieces, chars, &range);
        pieces.push(Piece { range, color: h.color, created: h.created, edited: h.edited });
    }
    join(&mut pieces, chars);
    pieces
}

/// The pieces with `range` taken out of them.
fn cut(pieces: Vec<Piece>, chars: &[char], range: &Range<usize>) -> Vec<Piece> {
    let mut out = Vec::with_capacity(pieces.len() + 1);
    for p in pieces {
        if p.range.end <= range.start || p.range.start >= range.end {
            out.push(p);
            continue;
        }
        for r in [p.range.start..range.start, range.end..p.range.end] {
            let r = trim(chars, r);
            if !r.is_empty() {
                out.push(Piece { range: r, ..p.clone() });
            }
        }
    }
    out
}

/// Sorts the pieces and joins neighbours of one colour with only spaces between them.
fn join(pieces: &mut Vec<Piece>, chars: &[char]) {
    pieces.sort_by_key(|p| p.range.start);
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for p in pieces.drain(..) {
        if let Some(last) = out.last_mut()
            && last.color == p.color
            && chars[last.range.end.min(p.range.start)..p.range.start].iter().all(|c| c.is_whitespace())
        {
            last.range.end = last.range.end.max(p.range.end);
            last.created = last.created.min(p.created);
            last.edited = last.edited.max(p.edited);
            continue;
        }
        out.push(p);
    }
    *pieces = out;
}

/// What a verse whose text is `text` shows: its coloured ranges in chars, in order and not
/// overlapping.
pub fn ranges(parts: &[Highlight], text: &str) -> Vec<(Range<usize>, HighlightColor)> {
    let chars: Vec<char> = text.chars().collect();
    layer(parts, &chars).into_iter().map(|p| (p.range, p.color)).collect()
}

/// Colours chars `range` of a verse whose text is `text` (`None` clears them), over the highlights
/// it has. Returns the verse's highlights after, in `text`'s offsets.
pub fn paint(
    parts: &[Highlight],
    text: &str,
    range: Range<usize>,
    color: Option<HighlightColor>,
    now: i64,
) -> Vec<Highlight> {
    let chars: Vec<char> = text.chars().collect();
    let range = trim(&chars, range);
    if range.is_empty() {
        return parts.to_vec();
    }
    let before = layer(parts, &chars);
    // Colouring exactly what a highlight covers recolours it: it keeps when it was made.
    let created = before.iter().find(|p| p.range == range).map_or(now, |p| p.created);
    let mut pieces = cut(before, &chars, &range);
    if let Some(color) = color {
        pieces.push(Piece { range, color, created, edited: now });
    }
    join(&mut pieces, &chars);
    let full = trim(&chars, 0..chars.len());
    pieces
        .into_iter()
        .map(|p| {
            if p.range.start <= full.start && p.range.end >= full.end {
                return Highlight { edited: p.edited, ..Highlight::whole(p.color, p.created) };
            }
            Highlight {
                color: p.color,
                start: p.range.start as u32,
                end: (p.range.end < full.end).then_some(p.range.end as u32),
                quote: Some(chars[p.range.clone()].iter().collect()),
                created: p.created,
                edited: p.edited,
            }
        })
        .collect()
}

/// The colour that covers every letter of chars `range` in `text`, if one does.
pub fn color_of(parts: &[Highlight], text: &str, range: Range<usize>) -> Option<HighlightColor> {
    let chars: Vec<char> = text.chars().collect();
    let range = trim(&chars, range);
    if range.is_empty() {
        return None;
    }
    let pieces = layer(parts, &chars);
    let mut color = None;
    for i in range.filter(|&i| !chars[i].is_whitespace()) {
        let c = pieces.iter().find(|p| p.range.contains(&i))?.color;
        if color.is_some_and(|k| k != c) {
            return None;
        }
        color = Some(c);
    }
    color
}

/// The char offset in `text` of UTF-16 offset `utf16` (how JavaScript and Swift count).
pub fn chars_from_utf16(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.chars().enumerate() {
        if units >= utf16 {
            return i;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

/// The UTF-16 offset in `text` of char offset `chars`.
pub fn utf16_from_chars(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::BookId;
    use HighlightColor::{Blue, Green, Yellow};

    const KJV: &str = "For God so loved the world, that he gave his only begotten Son, that whosoever \
                       believeth in him should not perish, but have everlasting life.";
    const WEB: &str = "For God so loved the world, that he gave his one and only Son, that whoever \
                       believes in him should not perish, but have eternal life.";

    fn at(text: &str, words: &str) -> Range<usize> {
        let b = text.find(words).unwrap();
        let s = text[..b].chars().count();
        s..s + words.chars().count()
    }

    fn shown(parts: &[Highlight], text: &str) -> Vec<(String, HighlightColor)> {
        ranges(parts, text)
            .into_iter()
            .map(|(r, c)| (text.chars().skip(r.start).take(r.len()).collect(), c))
            .collect()
    }

    #[test]
    fn part_of_a_verse_keeps_its_words() {
        let parts = paint(&[], KJV, at(KJV, "God so loved the world"), Some(Yellow), 10);
        assert_eq!(parts.len(), 1);
        let h = &parts[0];
        assert_eq!((h.start, h.end), (4, Some(26)));
        assert_eq!(h.quote.as_deref(), Some("God so loved the world"));
        assert_eq!(shown(&parts, KJV), vec![("God so loved the world".into(), Yellow)]);
        assert_eq!(h.excerpt(KJV), "…God so loved the world…");
    }

    #[test]
    fn the_whole_verse_is_stored_as_a_whole_verse() {
        let parts = paint(&[], KJV, 0..usize::MAX, Some(Green), 10);
        assert_eq!(parts, vec![Highlight::whole(Green, 10)]);
        // Selections that only leave out surrounding spaces are whole too.
        let parts = paint(&[], " a b ", 0..4, Some(Green), 10);
        assert!(parts[0].is_whole());
        // The rest of a verse from a point is open-ended.
        let parts = paint(&[], KJV, at(KJV, "but have everlasting life.").start..usize::MAX, Some(Blue), 10);
        assert_eq!(parts[0].end, None);
        assert_eq!(parts[0].excerpt(KJV), "…but have everlasting life.");
    }

    #[test]
    fn spaces_at_the_edges_of_a_selection_are_left_out() {
        let r = at(KJV, " so loved ");
        let parts = paint(&[], KJV, r, Some(Yellow), 10);
        assert_eq!(parts[0].quote.as_deref(), Some("so loved"));
        assert_eq!(paint(&[], KJV, 3..4, Some(Yellow), 10), Vec::new(), "only a space: nothing to colour");
    }

    #[test]
    fn same_colour_neighbours_and_overlaps_join() {
        let a = paint(&[], KJV, at(KJV, "God so"), Some(Yellow), 10);
        let b = paint(&a, KJV, at(KJV, "loved the"), Some(Yellow), 20);
        assert_eq!(shown(&b, KJV), vec![("God so loved the".into(), Yellow)]);
        assert_eq!(b[0].created, 10, "joined highlights keep the earliest date");
        let c = paint(&b, KJV, at(KJV, "the world"), Some(Yellow), 30);
        assert_eq!(shown(&c, KJV), vec![("God so loved the world".into(), Yellow)]);
        // A comma between two highlights keeps them apart.
        let d = paint(&c, KJV, at(KJV, "that he"), Some(Yellow), 40);
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn a_new_colour_goes_on_top_and_splits_what_it_covers() {
        let a = paint(&[], KJV, at(KJV, "God so loved the world"), Some(Yellow), 10);
        let b = paint(&a, KJV, at(KJV, "loved"), Some(Blue), 20);
        assert_eq!(
            shown(&b, KJV),
            vec![("God so".into(), Yellow), ("loved".into(), Blue), ("the world".into(), Yellow)]
        );
        // The pieces left of the first highlight keep its date.
        assert!(b.iter().filter(|h| h.color == Yellow).all(|h| h.created == 10));
        // Clearing part of it leaves the rest.
        let c = paint(&b, KJV, at(KJV, "loved the"), None, 30);
        assert_eq!(shown(&c, KJV), vec![("God so".into(), Yellow), ("world".into(), Yellow)]);
        // A whole-verse colour replaces everything.
        let d = paint(&c, KJV, 0..usize::MAX, Some(Green), 40);
        assert_eq!(d, vec![Highlight::whole(Green, 40)]);
        // Recolouring exactly what's covered keeps the date it was made.
        let e = paint(&d, KJV, 0..usize::MAX, Some(Blue), 50);
        assert_eq!(e, vec![Highlight { edited: 50, ..Highlight::whole(Blue, 40) }]);
        assert_eq!(paint(&e, KJV, 0..usize::MAX, None, 60), Vec::new());
    }

    #[test]
    fn colour_of_a_selection() {
        let a = paint(&[], KJV, at(KJV, "God so loved the world"), Some(Yellow), 10);
        assert_eq!(color_of(&a, KJV, at(KJV, "so loved")), Some(Yellow));
        assert_eq!(color_of(&a, KJV, at(KJV, " so loved ")), Some(Yellow), "spaces don't count");
        assert_eq!(color_of(&a, KJV, at(KJV, "world, that")), None, "partly uncoloured");
        let b = paint(&a, KJV, at(KJV, "loved"), Some(Blue), 20);
        assert_eq!(color_of(&b, KJV, at(KJV, "so loved")), None, "two colours");
        assert_eq!(color_of(&[Highlight::whole(Green, 1)], KJV, 0..usize::MAX), Some(Green));
    }

    #[test]
    fn another_translation_finds_the_words_or_shows_the_whole_verse() {
        let a = paint(&[], KJV, at(KJV, "God so loved the world"), Some(Yellow), 10);
        // The WEB has the same words at the same place.
        assert_eq!(shown(&a, WEB), vec![("God so loved the world".into(), Yellow)]);
        // Words found elsewhere are found there, wherever the offsets point.
        let b = paint(&[], KJV, at(KJV, "should not perish"), Some(Blue), 10);
        assert_eq!(b[0].range_in(WEB), at(WEB, "should not perish"));
        // Words the text hasn't: the whole verse.
        let c = paint(&[], KJV, at(KJV, "everlasting"), Some(Green), 10);
        assert_eq!(c[0].range_in(WEB), 0..WEB.chars().count());
        assert_eq!(c[0].excerpt(WEB), WEB);
        // Of several placed pieces that overlap there, the newest is on top.
        let mut both = b.clone();
        both.extend(paint(&[], KJV, at(KJV, "only begotten"), Some(Green), 20));
        assert_eq!(shown(&both, WEB), vec![(WEB.into(), Green)]);
    }

    #[test]
    fn the_last_colouring_is_on_top_in_another_translation() {
        // Both fall back to the whole verse in the WEB; recolouring the older one puts it on top
        // there, though it keeps the date it was made.
        let a = paint(&[], KJV, at(KJV, "only begotten"), Some(Yellow), 10);
        let b = paint(&a, KJV, at(KJV, "everlasting"), Some(Green), 20);
        assert_eq!(shown(&b, WEB), vec![(WEB.into(), Green)]);
        let c = paint(&b, KJV, at(KJV, "only begotten"), Some(Blue), 30);
        assert_eq!(shown(&c, WEB), vec![(WEB.into(), Blue)]);
        let blue = c.iter().find(|h| h.color == Blue).unwrap();
        assert_eq!((blue.created, blue.edited), (10, 30));
    }

    #[test]
    fn editing_in_another_translation_keeps_what_was_shown() {
        // A KJV highlight the WEB shows whole; colouring a word in the WEB keeps the rest shown.
        let kjv = paint(&[], KJV, at(KJV, "everlasting"), Some(Green), 10);
        let web = paint(&kjv, WEB, at(WEB, "eternal"), Some(Blue), 20);
        let shown_web = shown(&web, WEB);
        assert_eq!(shown_web.len(), 3);
        assert_eq!(shown_web[1], ("eternal".into(), Blue));
        assert!(web.iter().filter(|h| h.color == Green).all(|h| h.created == 10));
    }

    #[test]
    fn a_quote_found_twice_takes_the_nearest() {
        let text = "the one, the two, the three";
        // "the" is at 0, 9 and 18; the offsets point at 9.
        let h =
            Highlight { start: 9, end: Some(12), quote: Some("the".into()), ..Highlight::whole(Yellow, 1) };
        assert_eq!(h.range_in(text), 9..12);
        let moved = Highlight { start: 15, end: Some(18), ..h };
        assert_eq!(moved.range_in(text), 18..21);
    }

    #[test]
    fn words_are_found_however_they_are_spaced() {
        // A verse's text runs poetry lines together; its plain text puts a space between them.
        let text = "The LORD is my shepherd;I shall not want.";
        let plain = "The LORD is my shepherd; I shall not want.";
        let parts = paint(&[], text, at(text, "shepherd;I shall"), Some(Yellow), 1);
        assert_eq!(parts[0].excerpt(plain), "…shepherd; I shall…");
        assert_eq!(parts[0].range_in(plain), at(plain, "shepherd; I shall"));
    }

    #[test]
    fn offsets_without_words_are_trusted_within_the_text() {
        let h = Highlight { start: 4, end: Some(400), ..Highlight::whole(Yellow, 1) };
        assert_eq!(h.range_in("For God"), 4..7);
        let past = Highlight { start: 40, ..h };
        assert_eq!(past.range_in("For God"), 0..7, "nothing left: the whole verse");
    }

    #[test]
    fn spans_cover_their_verses() {
        let v = |n| VerseRef::new(BookId(42), 3, n);
        let s = TextSpan { first: v(16), start: 4, last: v(18), end: 9 };
        assert_eq!(s.refs(), vec![v(16), v(17), v(18)]);
        assert_eq!(s.range_of(v(16)), 4..usize::MAX);
        assert_eq!(s.range_of(v(17)), 0..usize::MAX);
        assert_eq!(s.range_of(v(18)), 0..9);
        let one = TextSpan { first: v(16), start: 4, last: v(16), end: 9 };
        assert_eq!(one.range_of(v(16)), 4..9);
        assert_eq!(TextSpan::verses(v(1), v(2)).range_of(v(2)), 0..usize::MAX);
        let across = TextSpan { last: VerseRef::new(BookId(42), 4, 1), ..s };
        assert_eq!(across.refs(), vec![v(16)]);
    }

    #[test]
    fn utf16_offsets() {
        let text = "a😀b"; // the emoji is two UTF-16 units
        assert_eq!(utf16_from_chars(text, 2), 3);
        assert_eq!(chars_from_utf16(text, 3), 2);
        assert_eq!(chars_from_utf16(text, 1), 1);
        assert_eq!(chars_from_utf16(text, 99), 3);
        assert_eq!(utf16_from_chars("Ἰησοῦς", 6), 6);
    }
}
