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

Renderer: **wgpu/Metal on macOS, glow (OpenGL) elsewhere** — chosen by measurement, not
assumption. The macOS CI job runs the startup and scroll probes with both renderers: OpenGL
(deprecated on macOS and layered over Metal) held only ~15 fps while scrolling there (67 ms
between frames) where Metal held the display's 60, and time to first frame was within that
runner's noise. Under Xvfb on Linux, glow creates its context about twice as fast as wgpu
(≈90 ms vs ≈170 ms with software rendering) and scrolls fine. wgpu asks for the **low-power GPU**:
a reader has no use for a discrete GPU, and waking one costs start-up time and battery.
`BIBLE_RS_RENDERER=glow|wgpu` overrides the choice.

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
   module** (zText/RawText, KJV/KJVA/NRSV/NRSVA versification, OSIS/ThML/GBF markup, and Sapphire-II-locked
   modules with the user's unlock key — the cipher is verified against SWORD's own C++
   implementation). The converted `.bbl` lives only in the user's data folder.
2. **Fetch from a publisher's API.** The NLT API (Tyndale) and the NET Bible API (bible.org)
   without a key, the ESV API (Crossway) with a free key, and API.Bible (NASB, NKJV, CSB, … where
   the user's plan and the publisher allow) with the user's key. Keys are stored in the macOS
   Keychain (the `keyring` crate; a 0600 file elsewhere). A per-translation background worker
   fetches chapters on demand, rate-limited, and the cache follows each API's terms:
   - NLT: 50 verses a request and 500 requests a day anonymously, 500 and 5,000 with a free key.
     Long chapters are fetched as consecutive ranges, every request is spaced ≥ 1 s, and the day's
     count persists across launches so the limit is never passed. Tyndale's permission statement
     caps quotation at 500 verses and never a complete book; the cache keeps to both.
   - NET: no storage limit in its terms; requests spaced ≥ 1 s, chapters refreshed after 30 days.
     The NET isn't bundled even though it is free to copy: "free means free" forbids bundling it
     with anything sold, and this repository's licence allows selling builds.
   - ESV: never more than 500 verses stored and never more than half of any book (a one-chapter
     book is displayed but not stored); requests spaced ≥ 1 s (limit is 60/min).
   - API.Bible: fewer than 500 consecutive verses cached, refreshed within 14 days; every
     displayed chapter is reported through **FUMS** (Fair Use Management System) with an
     anonymous device id.

   Each online chapter ends with the publisher's notice (the copyright API.Bible returns, plus its
   citation), and the notice is a link to the publisher's site: the ESV, NET and API.Bible terms
   ask for a link as well as the notice. [TRANSLATIONS.md](TRANSLATIONS.md) records which modern
   translations can be had free this way and why the rest can't.

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
- Full-size content view: the toolbar is the title bar. An empty `NSToolbar` in the *unified*
  style makes AppKit lay the title bar out 52 pt tall with the traffic lights centred in it — the
  same geometry as Safari or Notes — and keeps them there through resizing, full screen and theme
  changes. The egui toolbar reads the buttons' real frames back every frame (a handful of
  Objective-C calls, ~5 µs): its height is twice their centre line and its first item starts 16 pt
  after them, so the two can't drift apart. Measured at 2×: traffic lights and every toolbar icon
  centre on the same pixel row. The alternative — moving the standard buttons by hand, as Tauri's
  `trafficLightPosition` does — costs nothing at launch but has to be re-applied after every
  AppKit layout pass and can flicker during live resize; the toolbar costs ~2 ms at launch
  (`setToolbar:` building its views).
- In full screen the (empty) toolbar is hidden, since AppKit would otherwise show it as an opaque
  strip over the app's own.
- A 52 pt title bar makes the toolbar's buttons part of the window-drag area: a double-click on
  Back would zoom the window. macOS collects that area from views' `mouseDownCanMoveWindow` ahead
  of time and hands it to the window server, so answering per click doesn't work; instead
  invisible, click-through `NSView`s sit over the button groups (and any egui popup reaching into
  the bar) and answer "no".
- The sidebar runs the window's full height like a source list, with the traffic lights and the
  sidebar/back/forward buttons in its top strip; the toolbar over the reading area takes the page
  colour, as in Books.
- The sidebar slides on a critically damped spring: 95 % of the way in 135 ms, at rest by
  265 ms. The spring is evaluated in closed form from a frame clock, so a frame's timing can't
  bend its path, and a second ⌘B mid-slide keeps the current speed and turns back instead of
  jumping. A late frame advances that clock by at most one and a half refresh periods: when a
  ProMotion panel wakes from its idle refresh rate the first frames come late, and the slide
  then starts a frame later instead of jumping ahead. The sidebar moves at its full width
  rather than narrowing, so nothing in it is laid out again.
- Pacing the slide with a display link that asks for 120 Hz was tried and measured against
  this: under load macOS still ran some slides at 60 Hz, and over four alternating rounds the
  display link had more late frames, not fewer. It was left out.
- The reading column only moves while the sidebar slides as long as its measure doesn't change,
  which is the case from a ~1,064 pt window up (640 pt of text plus 80 pt margins beside the
  264 pt sidebar). Narrower windows reflow the chapter on screen every frame: one chapter layout
  (0.1–2.5 ms, Psalm 119 the worst) per frame, with the reader's idle typesetting of
  neighbouring chapters paused until the slide ends. Sliding a stale layout and reflowing once at
  the end was the alternative; it trades the reflow cost for a visible rewrap when the slide
  stops, and the measured frame times didn't call for it.
- While the sidebar moves, the drag carve-out views stay where they were and catch up when it
  stops. Moving `NSView`s every frame made AppKit ask for redraws at 60 Hz on top of the
  display's 120 Hz frames: half again as many frames, unevenly spaced.
- Reduce Motion, the app's setting or the system's (read when the sidebar is toggled, never at
  launch), shows and hides the sidebar at once.
- `scripts/bundle-macos.sh` builds `bible-rs.app` (icon, Info.plist, ad-hoc or Developer ID
  signature) and a DMG; CI does this on `macos-latest` and uploads the DMG.

## Visual design: the Paper designs

The interface follows the high-fidelity designs in the Paper file "Bible" (Screens 01–09, Flows
10–34 and the Design System page). How they map onto the app:

- **Tokens are compiled in.** `crates/app/assets/design-tokens.json` is the design's token file,
  copied verbatim. `build.rs` turns it into Rust constants: one `Palette` per theme whose fields
  are the design's colour roles (`paper`, `ink`, `ink_2`, `accent_soft`, `fill`, …), the six
  highlight fills and swatches, the shadows as ring + drop `Elevation`s, the type scale
  (`text::BODY`, `text::LABEL`, …) and the metric groups (`space`, `radius`, `titlebar`,
  `reading`, …). Nothing is parsed at startup; a changed token is a rebuild.
- **CSS baselines.** egui puts a baseline at the font's ascent from the top of its row (and
  centres a font against the tallest fallback in its family), where CSS centres the font's
  content area in the line box. Text blocks and headings are moved by the difference — measured
  from egui and computed from the fonts' hhea metrics — so lines sit within half a point of the
  design. Inline verse numbers get a line height that puts their baseline where the design's
  raised numbers are.
- **The reader runs under the toolbar.** The toolbar is a row of glass capsules over the top of
  the reader; at rest it is transparent over the page, and once text scrolls beneath it takes the
  translucent toolbar material and a hairline. Each chapter ends with 80 pt of space, more than
  the toolbar plus the page's top margin, so a chapter scrolled to its start shows nothing of the
  previous one above it.
- **Floating source list.** The sidebar is a 256 pt panel inset 8 pt from the window's edges,
  holding the traffic lights and its own toggle in its header, then Study (bookmarks, highlights,
  notes, expandable in place) and Books (testaments → books → a chapter grid with activity dots).
  Search moved to the toolbar's field; while it has a query the sidebar shows the results.
- **First launch** opens a 960 × 680 window with the onboarding steps (translation, then theme and
  reading size); finishing them grows it to the reader's 1280 × 800. Settings saved before
  onboarding existed count as onboarded, and the startup/scroll probes skip it. The design's
  third step ("Ready") isn't drawn, so the second step's Continue opens the reader.
- **Pages take the reader's place.** The Flows frames add full pages (the translation library,
  then the study collections, history and plans). The sidebar's rows open them; the toolbar
  keeps back/forward (Back returns to the reader) and swaps the pickers and view tools for a
  200 pt field that filters the page. Rows show the real data: sizes are each text's decoded
  size from its file header, descriptions and years of the bundled texts are a small table in
  `bundled.rs`.
- **The reader's top stays at the reader screens' 40 pt.** Frames 01–03 pad the page 40 pt above
  the chapter heading; the later Flows frames (22, 23, 32, 34) draw the same reader with 24. The
  reader keeps 40 so the reader screens match, and those frames sit 16 pt lower than drawn.
- **A search that finds nothing suggests a spelling that does.** Each query word is compared
  with the translation's vocabulary (edit distance up to 1–3 by length); one word is swapped at
  a time and the nearest relative to the word's length that finds verses wins.
- **An online chapter that can't load gets a page, not a line.** When the reading translation is
  online and the chapter fails, the reader's place shows why (no connection, or the provider's
  own reason), Retry, a local translation instead (the KJV when installed) and the chapters the
  disk cache still holds, as the design's frame 33. Its error toast stays until clicked away or
  the chapter loads. Parallel columns keep their one-line message.
- **Toasts are the design's inverse capsules** at the bottom centre of the main area, 4 s for
  messages; errors stay.
- **Menus share one frame and one text column** (`menu.rs`, the `menu` group added to the token
  file). The design system's menu sheet (ds-12) gives the radius 12, 28 pt rows with radius 6
  highlights, 24 pt section headers and 9 pt separators; its 6 pt padding felt cramped, so row
  highlights sit 8 pt from a menu's sides and 10 pt from its top and bottom. Text, icons,
  checkboxes, section headers and separators all start 10 pt inside the highlight. egui's menus
  (the translation switcher, select fields, page menus) and the hand-painted verse and More
  menus use the same tokens; popovers with fields or controls (text settings, New Note) keep
  egui's popup frame. The book picker is a grid of menu cells in the menu frame. Its 28 pt cells
  sit edge to edge, as wide as the longest name (narrower in a 620 pt window), with names and
  the testament headers in the text column and one 16 pt gutter between the testaments; the book
  and chapter being read are selected. Arrow keys move the same highlight as the pointer, return
  chooses and backspace goes back to the books. The reader's single-key shortcuts wait while the
  picker is open, so escape closes it and the arrows stay in the grid; the copy chords still
  copy.
- **Empty and error states share one widget and one axis.** Notes, Bookmarks, Highlights,
  History, finished plans, the offline error and a search that found nothing all draw
  `pages::Empty`. Each sits optically centred in the space it fills, with 45 % of the leftover
  height above it, where frames 32 and 33 anchor theirs near the top. Buttons centre on what
  shows: a ghost button's padding is invisible at rest, and centring its box would pull the row
  7 pt left, as the design does. Hints that don't fit the width wrap into centred rows.
- **Compact windows fold the toolbar, not the sidebar.** Below a 760 pt toolbar the view tools
  and the search field become one Search + More capsule (frame 34), book names shorten ("Rom 8")
  and the reader keeps 80 pt margins (a 560 pt measure at 720). The sidebar stays where the user
  left it; More can show or hide it. While searching, the 136 pt field takes the capsule's place.
  ⌘T (Text & Layout) is a menu command rather than a row in Settings › Shortcuts, which keeps the
  designed list.
- Static Inter 500/600 instances (made with fonttools from the variable font) keep semibold UI
  text and verse numbers off egui's per-run variation path. Literata keeps its default optical
  size: at 20 pt its widths differ from opsz 20 by 0.3 %, so line breaks match the design anyway.

## What was traded away

- No per-word highlighting (verse-level only) and no free text selection inside verses — verse
  selection with "copy with reference" covers the common case.
- No footnotes or cross-references from the source texts (dropped at import).
- SWORD modules in versifications other than KJV(A)/NRSV(A) (e.g. Vulgate, Synodal) aren't supported yet;
  deuterocanonical books are skipped everywhere to keep the 66-book grid.
- Search covers locally available text; online translations are searched via their own sites.
