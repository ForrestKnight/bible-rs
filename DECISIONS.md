# Design decisions

Why bible-rs is built the way it is, and what was traded away. The brief: a very snappy, extremely
fast native Bible reader for the Mac — nicely formatted text, search, highlights, notes — built on
a headless Linux VPS.

## GUI toolkit: egui (eframe)

| Option | Typography | Speed / footprint | Why not (for this app) |
|---|---|---|---|
| **gpui** (Zed) | Excellent on macOS (Core Text) | Excellent | Pre-1.0 API that moves with Zed, thin documentation outside Zed, and its Linux backend needs Vulkan — nothing could be built, run and screenshotted reliably on the headless VPS this was developed on. |
| **iced** | Good (cosmic-text shaping, font fallback) | Good | Retained Elm architecture makes a virtualized, continuously scrolling 31,000-verse document awkward: rich text spans exist, but positioning individual glyph runs, hit-testing verses and painting under-layers need custom widgets either way. |
| **AppKit via objc2 / SwiftUI** | Native Core Text | Excellent | Mac-only: impossible to build, test or screenshot on the Linux build machine, and several times the code per view. |
| Tauri / Electron | Browser-grade | Web view start-up and memory | A web view costs 100+ MB and hundreds of milliseconds before first paint — the opposite of the brief. |
| **egui 0.36** ✔ | HarfRust (HarfBuzz) shaping with kerning and ligatures, skrifa outlines rasterized by vello_cpu, variable-font axes per text section | ~0.2 ms per frame while scrolling, first frame ≈ 15 ms after the window exists | Widgets don't look like AppKit out of the box — addressed with custom drawing. |

egui 0.36 is what tipped it: text is now shaped by **HarfRust** (a Rust port of HarfBuzz) and
rasterized from **skrifa** outlines, so serif body text gets proper kerning and ligatures, and
**variable-font coordinates** can be set per text section — Literata's optical-size and weight
axes are used for display headings vs. body text. Immediate mode makes the hard parts simple:

- **Continuous scroll across the whole Bible.** The scroll position is an *anchor* (chapter +
  offset) rather than a position in one huge document. Each frame lays out only the chapters on
  screen (cached per chapter), so scrolling Genesis → Revelation costs the same per frame as one
  psalm, and a window resize or font change re-anchors to the verse you were reading.
- **Underlays from glyph geometry.** Highlights, selection, search flashes and verse hit-testing are
  computed from the shaped glyph positions, so selecting or highlighting a verse never re-lays out
  text.
- **One code path on macOS and Linux.** The same app runs on the Mac (window, menu bar, Keychain)
  and under Xvfb on the VPS, where the screenshots, the startup probe and the scroll probe run.

Renderer: **glow** (OpenGL) by default on every platform. It creates its context several times
faster than wgpu (≈90 ms vs ≈200 ms under Xvfb with software rendering), and time-to-first-text is
the headline metric. `BIBLE_RS_RENDERER=wgpu` switches to wgpu/Metal.

## Typography

- **Literata** (TypeTogether for Google Play Books; SIL OFL) for Scripture: designed for long-form
  screen reading, with roman + italic variable fonts. Weight and optical size are pinned per role
  (body 400 @ 14 pt, headings 600, book titles 300 @ 60 pt optical size).
- **Inter** (OFL) for the interface and for verse numbers, which contrast with the serif text.
- Measure capped at ~34 em (≈ 65–75 characters per line), line height 1.62, both adjustable.
- Verse numbers are superscripts followed by a no-break space, so a number never ends a line apart
  from its verse.
- Paragraphs get a first-line indent except after headings (book convention); poetry lines get a
  level indent plus a hanging indent for wrapped lines; stanza breaks add space; `Selah` and
  similar lines are right-aligned.
- The divine name (`\nd`, `<divineName>`) is set in small capitals: egui has no OpenType `smcp`
  toggle, so the remaining letters are set in capitals at 78 % size with their line height adjusted
  so their baseline matches the text (egui otherwise aligns mixed sizes by ascent).
- Words of Christ in red (toggle), KJV translator additions in italics, Psalm superscriptions in
  muted italics, section headings in semibold.
- Light, sepia and dark palettes chosen for long reading (warm paper, not pure white; not pure black).

## Data: one verse-indexed model, many sources

Every translation — bundled, imported or fetched — becomes the same structure: one UTF-8 string of
verse text plus flat arrays of verses, structural *marks* (paragraph, poetry line + level, stanza
break, heading, Psalm title) and character *spans* (words of Christ, additions, divine name, Selah),
all keyed to the canonical **66-book / 1,189-chapter KJV grid**. Using one grid is what makes
verse-aligned parallel columns, the compare view, and translation-independent highlights possible.

`.bbl` files store that model as column arrays compressed with zstd (level 19 for bundled data):
the 4.1 MB KJV is 1.04 MB, all seven bundled translations ≈ 7 MB. The JSON metadata header is
uncompressed, so listing translations never decompresses anything. Decoding is a handful of linear
passes (~18 ms for a whole Bible) and happens on a background thread while the window opens; other
translations decode lazily (or are warmed after the first frame), so switching is instant.

Bundled data is generated by `cargo run -p xtask -- build-data` from eBible.org USFM archives whose
URLs and SHA-256 digests are pinned in the repository, using the same USFM importer users get.

## Translations and licensing

Only public-domain texts are bundled: KJV, WEB, BSB, ASV, YLT, Darby, Webster (provenance and
licence notes in `LICENSES/`). Bible in Basic English was left out because its status outside the
US is unclear. Copyrighted translations — NLT, NASB, LSB, ESV, NIV, … — never appear in the
repository: no text, no derived index, no test fixtures (tests use public-domain words in the
*format* of each source).

The pluggable provider interface (`Library` → bundled / imported / online entries, all answering
`chapter(id, chapter)` without blocking) gives users two routes to licensed texts:

1. **Import a file they own** — USFM (file/folder/zip), OSIS XML, Zefania XML, or a **SWORD
   module** (zText/RawText, KJV/KJVA versification, OSIS/ThML/GBF markup, and Sapphire-II-locked
   modules with the user's unlock key — the cipher is verified against SWORD's own C++
   implementation). The converted `.bbl` lives only in the user's data folder.
2. **Fetch with their own API key** — the ESV API (Crossway) and API.Bible (NASB, LSB, NLT, …
   where the user's plan and the publisher allow). Keys are stored in the macOS Keychain (the
   `keyring` crate; a 0600 file elsewhere). A per-translation background worker fetches chapters
   on demand, rate-limited, and the cache follows each API's terms:
   - ESV: never more than 500 verses stored and never more than half of any book (a one-chapter
     book is displayed but not stored); requests spaced ≥ 1 s (limit is 60/min); Crossway's
     copyright notice shown with the text.
   - API.Bible: fewer than 500 consecutive verses cached, refreshed within 14 days; the copyright
     returned with each chapter is shown; every displayed chapter is reported through **FUMS**
     (Fair Use Management System) with an anonymous device id.

Highlights, notes and bookmarks are keyed by canonical reference, not by translation, so they
follow the reader across translations.

## Search

No inverted index: each translation lazily builds a normalized corpus (lower-case, diacritics
folded, apostrophes joined, punctuation → spaces) laid out as `" w1 w2 … "` per verse. A term is a
SIMD substring scan (`memchr::memmem`) for `" term"`, a phrase is a longer needle, and because
verses are wrapped in spaces and needles never contain a double space, a match can't straddle two
verses. Mapping a hit to its verse is a binary search, after which the scan jumps to the next
verse. Multi-term queries scan the longest (rarest) term first, then filter candidates. Results:
0.15–1.8 ms on the KJV (see README), comfortably inside the 10 ms budget, with no index to
build at import time or store on disk.

## Storage

One SQLite file (`study.sqlite`, WAL mode) holds highlights, notes, bookmarks and settings; it is
mirrored in memory (hundreds of rows) so the reader can query it per verse every frame. Settings
(including the reading position) are written at most every 1.5 s, so scrolling never waits on
disk. Export to JSON (round-trips via import) and Markdown.

## macOS integration

- Native menu bar via `muda` (`cfg(target_os = "macos")`). Menu items with key equivalents are
  routed into the same actions as keyboard shortcuts; there is deliberately no Edit menu because
  AppKit's predefined copy/paste items would swallow ⌘C/⌘V before text fields see them.
- Full-size content view: the toolbar is the title bar.
- `scripts/bundle-macos.sh` builds `bible-rs.app` (icon, Info.plist, ad-hoc or Developer ID
  signature) and a DMG; CI does this on `macos-latest` and uploads the DMG.

## What was traded away

- No per-word highlighting (verse-level only) and no free text selection inside verses — verse
  selection with "copy with reference" covers the common case.
- No footnotes or cross-references from the source texts (dropped at import).
- SWORD modules in non-KJV versifications (e.g. Vulgate, Synodal) aren't supported yet;
  deuterocanonical books are skipped everywhere to keep the 66-book grid.
- Search covers locally available text; online translations are searched via their own sites.
