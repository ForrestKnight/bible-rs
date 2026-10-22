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

## Highlights: whole verses, words, and stretches across verses

A highlight can cover a whole verse, a few words of one, or a stretch across verses. The store
keeps highlights per verse, and a verse can hold several. Each records the chars it covers in the
verse's text (`start`, and `end`, which is empty when it runs to the verse's end) and, for part of a
verse, the words themselves (`quote`). A stretch across verses is stored as the tail of the first
verse, the middle verses whole, and the head of the last. Keeping everything per verse leaves
lookups as they were (one map read per verse while painting) and lets each verse's highlights be
merged on their own. The range logic is in `bible-core` (`highlight.rs`), so the Mac, web and phone
apps can't disagree about it.

**Offsets and translations.** Highlights are keyed by canonical reference and show in every
translation, but char offsets only mean something in the text they were counted in. A highlight of
part of a verse is placed in another text where its words are (ignoring spacing, the occurrence
nearest its offset). John 3:16's "God so loved the world" lands on the same words in the KJV and
the WEB. When the other translation words it differently, the highlight **falls back to the whole
verse**, so it never disappears. Hiding it was the other choice; a highlight that vanishes on
switching translation looks like lost data. When several fall back at once, the one coloured most
recently is on top.

**Overlaps.** Colouring over a highlight replaces it where they overlap, like a highlighter over a
highlighter: the newest colour is on top, and the rest of the older one stays. Neighbours of one
colour with only spaces between them join, keeping the earlier date. Each highlight keeps when it
was made (what lists show) apart from when it was last coloured (what decides the top colour), so
recolouring an old highlight puts it on top without moving it in the lists. Clearing part of a highlight
leaves the rest. Edits are made against the text being read, so a verse that's edited is stored
again in that text's offsets, exactly as it was shown. A highlight that had fallen back to the whole
verse there becomes whole-verse.

**Choosing words.** On the Mac and the web, clicking a verse selects it and shift-click extends to
whole verses, as before. Dragging across text chooses words within one chapter, and shift-click
carries that choice on. Keys 1 to 6, 0 and the verse menu act on the chosen words when there are
some, and copying copies just them. Chosen words belong to the translation they were counted in:
switching translation lets go of them and leaves their verses selected. The web uses the browser's
own selection, so a touch screen gets
a long press and its handles, plus a bar of swatches under the selection, since it has no number
keys or right-click.

**Migration.** The SQLite file's `user_version` goes from 0 to 1. The highlights table gains
`char_start`, `char_end`, `quote` and `edited`, and `char_start` joins its key. The old rows are
copied across as whole-verse highlights with their colour and date. JSON exports add `start`,
`end` and `quote` only for parts of verses, and `edited` only when it differs from `created`, so older exports (and the web app's saved snapshots) import as
whole verses, and an older app reading a newer export sees whole verses. An older app opening a
migrated file still reads its highlights but can't save new ones.

**The phone.** The iPhone app shares the store, so it migrates the same way and keeps every
highlight, but it still draws and edits whole verses: part of a verse shows there as the whole
verse in its colour. Its Highlights list has a row per highlight, and removing a row removes only
that one. Choosing words needs hand-drawn selection handles over its custom text
drawing, so it's left for later.

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
- `scripts/bundle-macos.sh` builds `Bible.app` (icon, Info.plist, Sparkle, ad-hoc or Developer ID
  signature, hardened runtime). `scripts/release-macos.sh` wraps it for releases (RELEASING.md):
  universal binary, notarization, DMG, zip, checksums. CI runs it ad-hoc signed on `macos-latest`
  and uploads the DMG.
- The minimum is macOS 11 because the window code calls `NSWindow.toolbarStyle`, new in 11. The
  build targets 11.0 for both slices; the x86_64 target would otherwise default to 10.12.
- No entitlements. API keys use the legacy login Keychain (`SecKeychain`), which needs none under
  the hardened runtime; each item trusts the designated requirement (bundle ID and team), so
  releases signed by the same team read existing keys without a prompt.
- dmgbuild lays out the DMG. It writes Finder's `.DS_Store` itself, so it needs no Finder
  scripting and runs headless in CI. lzma compression (ULMO) makes the DMG 16 % smaller than zlib.
- Releases carry the DMG twice: `bible-rs-<version>-universal.dmg`, and `bible-rs-universal.dmg`
  for the website, whose Download button points at `releases/latest/download/` with a fixed name.
- The app is called Bible: the bundle (`Bible.app`), its executable, the menu bar, the window and
  the disk image's volume. The bundle ID (`dev.bible-rs.app`), the data folder and the keychain
  service keep bible-rs, so an installed copy's notes, settings and API keys carry over, as do
  the repository, the Cargo package, the command-line binary and the release file names (the
  website links to `bible-rs-universal.dmg`).
- Installed copies update with Sparkle 2, the updater most Mac apps outside the App Store use. A
  self-written updater would have to redo what it already does: signature checks, replacing a
  bundle the user may not own, relaunching. `scripts/fetch-sparkle.sh` downloads a pinned release
  (checked by SHA-256); `scripts/bundle-macos.sh` embeds the framework without its XPC services
  (only sandboxed apps need them). The app loads it with `NSBundle` after the first frame rather
  than linking it, so it adds nothing to launch, and builds without it fall back to the releases
  page.
- The feed is `appcast.xml` on the latest GitHub release, so publishing a release is what ships it
  to installed copies, as it already did for the website. Each feed lists one version, the newest;
  Sparkle needs no more to update. Both the zip and the feed are signed with an EdDSA key
  (`SURequireSignedFeed`), and the zip's signature is checked before it is unpacked
  (`SUVerifyUpdateBeforeExtraction`, without which Sparkle refuses to start when the feed must be
  signed). The cost: the key can't be replaced by leaning on the Developer ID signature, so losing
  it means installed copies stop updating.
- Sparkle checks once a day without asking first (`SUEnableAutomaticChecks`), so every installed
  copy hears about a new release. Left to itself, Sparkle asks on the second launch, and anyone
  who declined or never answered would stay on an old version. The check downloads the feed and
  sends nothing but the app's version.
- 0.1.0 had no updater. Its users drag `Bible.app` from the new DMG into Applications, so the new
  app moves a `bible-rs.app` with the same bundle ID in the same Applications folder to the Trash.

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

## iPhone: SwiftUI and UIKit over the Rust core

The phone app lives in the same repository (`apps/ios`) and shares `bible-core` with the Mac:
the `.bbl` model and bundled texts, search, references, the study store and copy formatting.
What differs is the toolkit, and here the Mac's choice is overruled.

| Option | Why not (on a phone) |
|---|---|
| **egui on iOS** (eframe via winit) | Text fields get no native keyboard behaviour: no autocorrect, dictation, selection handles or hardware-keyboard shortcuts. AccessKit has no UIKit adapter, so VoiceOver would see an empty view. Scrolling would be egui's own physics, not `UIScrollView`'s deceleration, rubber-banding and scroll-to-top. No Dynamic Type. On the Mac each of these was a small cost; on a phone they are most of the experience. |
| Flutter, React Native | A second runtime and bridge in front of the same Rust core, with text drawn by their engines rather than Core Text. |
| Tauri mobile | A web view: the start-up time and memory the Mac app avoided. |
| **SwiftUI + UIKit over UniFFI** ✔ | Native text input, VoiceOver, Dynamic Type, scrolling and sheets. The Rust interface is described once, and Swift (and later Kotlin) bindings are generated from it. |

- **The Rust boundary is `crates/ffi` (`bible-ffi`), exported with UniFFI.** It returns plain
  records: a chapter as blocks (prose, poetry with levels, headings, Psalm titles) of styled runs
  (verse numbers, words of Christ, additions, small capitals), search hits with their match
  ranges, references, and study data. Everything that isn't drawing stays in Rust, so an
  Android app would draw the same structure. Match ranges are UTF-16 offsets, which Swift's
  `NSString` and Kotlin's `String` index directly. Calls are coarse (a chapter, a page of hits),
  so UniFFI's per-call cost of copying records through a buffer is small next to the work.
  A hand-written C ABI (cbindgen) would avoid that copy, but every string, list and error would
  need glue code written twice, once per platform.
- **Packaging:** `scripts/build-ios.sh` builds the crate as a static library for the device
  (`aarch64-apple-ios`) and the simulator (`aarch64-apple-ios-sim`), generates the Swift
  bindings, wraps both libraries in `BibleFFI.xcframework`, generates the Xcode project from the
  checked-in `apps/ios/project.yml` (XcodeGen) and builds the app. The library is a static one,
  so nothing extra is loaded at launch. The bundled translations come with it, embedded in the
  library as on the Mac.
- **Chrome is SwiftUI; the reader is UIKit.** SwiftUI's `Text` can't report which character
  was tapped or where a verse's glyphs are, and lazy stacks with rows of varying height jump
  when scrolled to an item far away. Both are central to a reader that selects verses and jumps
  to references. The reader is a `UIScrollView` whose content is typeset with TextKit, so
  scrolling keeps the system's physics.
- **The scroll holds a window of chapters, like the Mac's.** Chapters are typeset as they come
  within a screen and a half and dropped beyond four; adding or dropping one above moves the
  content and the offset together, so the text on screen stays still and the scroll runs from
  Genesis 1 to Revelation 22. The chapter just past each end of the window is typeset ahead on
  a background queue (TextKit 1 is safe off the main thread while no view owns the layout), so
  a fling never waits for it; a jump typesets its chapter on the spot. The text is drawn into
  256-pt tiles, so only what is on screen is ever drawn.
- **TextKit 1, not 2.** `NSLayoutManager` gives the glyph rectangles of any verse directly,
  which highlights, selection, taps and the note markers all need. A layout-manager delegate
  centres each line's face in its line box, as CSS and the Mac app do, so line spacing matches.
  A phone column has no margin, so verse-per-line numbers and poetry numbers sit in a narrow
  gutter at the start of the column; in paragraphs they are superscripts, as on the Mac.
- **One set of design tokens.** Colours come from `crates/app/assets/design-tokens.json`, compiled
  into `bible-ffi` and read through `palette(theme:)`, and the fonts are the Mac's files.
  Literata's optical size is pinned at its default (12) rather than following the point size,
  which would change widths and line breaks. The reading size follows Dynamic Type.
- **Settings and the reading position** are one JSON value in the study store's settings table
  (`ios`), written 1.5 s after a change and when the app goes to the background.
- **No network, no encryption.** The phone build turns off `bible-core`'s `tls` feature, so the
  app contains no TLS or other encryption code. Its Info.plist sets
  `ITSAppUsesNonExemptEncryption` to `NO`, and the App Store's export questions don't apply.
  `scripts/build-ios.sh` fails if TLS comes back. Online translations on the phone will need the
  feature back and a new answer. The privacy manifest, `PrivacyInfo.xcprivacy`, declares no data
  collected and gives reasons for the two kinds of file API that SQLite calls.
- **Not on the phone yet:** online translations, import, parallel and compare. Online
  translations need their keys in the iOS Keychain (the `keyring` crate the Mac uses has no iOS
  store) and the same caching limits as on the Mac. Import needs the document picker.
- **Measured** in an iPhone 17 Pro simulator on an M4 Pro, Release build:
  - Launch: the first page is drawn about 340 ms after the process starts (median of 7 warm
    launches). Almost all of it is iOS starting the process and the scene; the app's own work
    (opening the core, typesetting and drawing John 1) is about 25 ms.
  - Search: `love one another` returns 19 verses in about 1 ms.
  - Scrolling: a tile draws in 0.2 ms on average (1.1 ms at most). Chapters are typeset in
    1–7 ms each on the background queue, never during a frame. The first chapter takes 13–19 ms,
    because it includes decoding the translation.

## Web: the core in WebAssembly, the reader in the DOM

The web reader (`/read` on the site, `site/src/routes/read`) is the Mac app in a browser tab: the
same typesetting, themes, palette, search, study tools, pages and shortcuts. Every chapter is
also a static page that search engines can index.

| Option | Why not |
|---|---|
| **egui on the web** (eframe to a canvas) | One canvas: no text selection, find in page, screen readers, translation tools or reader mode, and nothing for a search engine to index. A few megabytes of wasm before the first word. |
| A JavaScript port of the core | Two implementations of search, references, the study store and the chapter layout, which would drift apart. |
| **bible-core in wasm, a Svelte reader drawing DOM text** ✔ | One core for Mac, iPhone and web. The browser lays out the text, so selection, accessibility and find in page work, and each chapter can be served as plain HTML. |

- **The boundary is `crates/web` (`bible-web`), through wasm-bindgen.** It is the same shape as
  `bible-ffi`: a chapter as blocks of styled runs (`bible_core::content`, shared by both), search
  hits with match ranges, references, copy formats, study data, history and plans. The study
  store keeps everything in memory and writes to SQLite only with the `native` feature, so in
  the browser it runs in memory and the page saves its JSON export in `localStorage` (debounced,
  and when the tab is hidden). `scripts/build-web.sh` builds it (with SIMD, which makes search's
  scans several times faster) into `site/src/lib/read/wasm/`. The output is committed, so the site
  builds without Rust: 612 KB, 196 KB with brotli.
- **Every chapter of every bundled translation is a prerendered page** (7 × 1,189 = 8,323), at
  `/read/<translation>/<book>/<chapter>`, with its title, a description taken from its first
  verses, a canonical URL and a sitemap entry. The server runs the same wasm module in Node to typeset
  the chapter with the default settings, so the page is readable before any script runs.
  `/read` itself restores the saved place, and names John 1 as its canonical URL.
- **The reader's places are real links that it follows itself.** Its controls were buttons, so a
  chapter page had no `<a href>` at all. The sidebar's books and chapters (rendered on the server
  for the book being read), the book picker and the translation menu are now links
  (`ui/follow.ts`). A plain click still moves the reader in place; a modified click opens a new
  tab. Previous and next chapter, the trail up to the book and translation, and the chapter in the
  other translations had no control to turn into a link. They are a navigation that stays out of
  sight until it has keyboard focus (`chrome/PageLinks.svelte`). The window looks and behaves as
  before.
- **Translations, books and verses have plain pages** (`/read/kjv`, `/read/kjv/john`,
  `/read/kjv/john/3/16`). They are text and links in the reader's type and theme, prerendered
  without the app or any client-side JavaScript. Every reader page carries JSON-LD (a WebPage of
  the site, with its breadcrumb trail), and `/sitemap.xml` is an index of a file per kind of page.
  Verse pages exist for the KJV, BSB and WEB only, and the other four translations' verses link to
  their place in the chapter (`#v16`). All seven would nearly double the output and triple the
  build time:

  | Verse pages for | Pages | Files | Static output | Local build (M-series) |
  |---|---|---|---|---|
  | none (chapters only, before) | 8,327 | 16,712 | 0.6 GB | 18 s |
  | **KJV, BSB, WEB** ✔ | 102,082 | 110,499 | 2.2 GB | about 2 min |
  | all seven | 226,471 | 234,892 | 4.2 GB | about 6 min |
- **Topic pages keep references, not text.** Marketing's 50 topics (`site/src/lib/topics/`) list
  each verse by book, chapter and verse; the page takes the KJV and BSB text from the core as it's
  prerendered, so a topic can never quote a verse differently from its verse page. A reference
  the core doesn't have, or a title that doesn't end in the site's name, stops the build. Each verse
  links to its verse pages, and each verse page links back to the topics that list it.
- **The site and the apps have different names.** The apps are "Bible" (`NAME`). The website is
  "Simply My Bible" (`SITE_NAME`, at simplymybible.com). It ends every title and is `og:site_name`
  and the WebSite's name in the JSON-LD.
- **Hydration never shows a wrong layout.** The first client render matches the server's.
  Saved settings are applied after mount. An inline script in `app.html` sets the theme and
  accent before the first paint. When saved settings change the typesetting (verse per line,
  parallel columns, no headings, a different place), the same script hides the text until the core
  has typeset it again, with a 5 s fallback.
- **Typesetting is CSS with the Mac's metrics.** The same Literata and Inter files, cut to the
  axis ranges and characters the reader uses (`site/scripts/fonts.sh`), with the same pinned
  optical sizes. CSS centres each line's glyphs in the line box, as the Mac app does. Highlights,
  the selection and the flash are stacked background layers sized to the 28 pt line box. With
  `box-decoration-break: clone` they wrap exactly as egui paints them. Colours come from
  `design-tokens.json` through `site/scripts/tokens.js`. Line breaks agree with the Mac's except
  for the odd word at the column edge, because the browser's shaping and harfrust's differ by a
  fraction of a pixel per line.
- **Continuous scrolling keeps a window of chapters and an anchor.** The anchor is the verse at
  the top of the reading area and how far into it (or how far above it, in a chapter head). Any
  relayout puts it back: fonts arriving, a resize, a settings change, chapters added above.
  Scrolling up or down never shifts the text on screen.
- **Translations are static files.** The bundled `.bbl` files are served from `/translations/`
  and fetched when first used. A service worker scoped to `/read` precaches the app, its wasm and
  fonts, keeps translations cache-first and pages network-first, so the reader works offline after
  one visit.
- **Importing runs in a worker, with its own wasm module.** The Mac's importers (USFM, OSIS,
  Zefania, SWORD, locked modules included) take their files from memory as well as from disk
  (`import::peek_files`, `import::import_files`). They sit behind bible-core's `import` feature,
  which brings the zip and gzip readers but not SQLite or the HTTP client. `scripts/build-web.sh`
  builds `bible-web` twice: the reader's module without the importers (710 KB) and
  `bible_import` with them (964 KB). Only `import.worker.ts` loads the second module, the first
  time someone imports, so the reader doesn't pay for it and a long conversion never blocks the
  page. On wasm the `.bbl` is compressed with ruzstd's fastest level, since zstd's C library
  isn't available there. The converted bytes go to IndexedDB, and the list of imports to
  `localStorage` next to the settings. An imported translation has no static page, so while it
  is the reading translation the address stays `/read`.
- **The NLT is the web's one online translation, through the site's own proxy.** ESV, API.Bible
  and NET still aren't offered: they need the reader's own key, which a page would expose, and
  their APIs aren't all open to browsers (CORS). The NLT has a site key (`NLT_API_KEY`, set in
  Vercel), so `/api/nlt` (`site/src/routes/api/nlt`) asks Tyndale for a chapter or less at a
  time and returns the verses' HTML with Tyndale's credit line. It refuses whole books and
  passages across chapters, and allows 20 chapters a minute and 300 a day for each address. Each
  function instance counts these on its own, so they slow one client down rather than cap the site.
  Browsers get `Cache-Control: no-store`, and Vercel's edge keeps a chapter for a day
  (`Vercel-CDN-Cache-Control`), but nothing from a one-chapter book, which could be the whole book. The page
  parses the HTML with the core's NLT parser (`bible_core::nlt_html`, shared with the Mac's
  `online::nlt`), and the core holds that one chapter until the reader moves on. So the reader
  shows one NLT chapter at a time, ending in Tyndale's credit line and links to the chapters
  either side, instead of scrolling on. Nothing of it is prerendered, stored or cached offline.
  Search, the study lists and exports use the translation read before the NLT, and a highlight
  of some words in the NLT colours their verses whole, since a partial highlight keeps the words
  it covers. `/read/nlt/<book>/<chapter>` shows the KJV page for that chapter (a `reroute`
  hook), with its canonical address and description. Vercel adds `X-Robots-Tag: noindex`, and the
  reader adds the robots meta tag while it shows the NLT. Offline, rate-limited or failing, the
  reader goes back to the translation read before, with a toast to retry.

## Windows

The Windows app is the same egui app with the macOS-only pieces given Windows counterparts
(`cfg(windows)`); on macOS nothing changes. [WINDOWS.md](WINDOWS.md) is the user-facing side.

- **The native Win32 menu bar**, through muda as on the Mac, rather than menus drawn in the
  window. Alt and access keys, screen readers and the look of other Windows programs come with it.
  The app installs no accelerator table, so the keys still reach egui, a shortcut changed in
  Settings works, and nothing fires twice. It costs two things. The drop-down menus stay light in
  the dark theme, because Windows has no public dark menu API and muda draws only the bar dark.
  And the menus show the default shortcuts.
- **The standard title bar.** A title bar drawn by the app, as on the Mac, would lose Snap
  layouts on the maximize button unless the app answered Windows' hit tests itself. egui already
  gives it the app's theme (dark for Dark). winit puts it back to the system's theme whenever a
  system setting changes, so the app sets it again when the window gets the focus back. The menu
  bar follows the theme too.
- **Direct3D 12 through wgpu**, not glow. Windows ships no OpenGL 3 driver, so glow depends on the
  GPU vendor's driver and fails in virtual machines and remote sessions; Direct3D 12 falls back to
  Windows' software renderer (WARP). Only the DX12 backend is enabled, as only Metal is on macOS,
  so wgpu doesn't spend start-up time probing Vulkan.
- **Shortcuts are the Mac's with Ctrl for ⌘**, and the UI writes them that way ("Ctrl+Shift+P").
  egui reports Ctrl as the command key on Windows, so ⌃⌘F (focus mode) would also be Ctrl+F
  (search); focus mode is F11 there, the usual full-screen key.
- **NSIS, a per-user installer.** No administrator prompt, an entry in Installed apps, a Start
  menu shortcut. NSIS also runs on macOS and Linux, so the installer is built and inspected off
  Windows too; MSI (WiX) and Inno Setup need Windows to build, and MSIX can't be installed
  unsigned. The C runtime is linked statically (`.cargo/config.toml`), so no Visual C++
  Redistributable is needed.
- **Updates check GitHub's latest release**, daily and on demand, and open the new installer in
  the browser. Sparkle is Mac-only; WinSparkle or Velopack would install by
  themselves, but both want signed builds and an update feed, which can wait for signing.
- **Share Verse copies the image** to the clipboard (eframe's clipboard does images) instead of
  calling Windows' share sheet, which needs WinRT and an asynchronous handshake for one button.
- **API keys in the Credential Manager** through the same `keyring` crate as the Keychain on
  macOS.

## Objects in 3D: open specs, three.js, and the same page in the Mac app

The first object is the Ark of the Covenant (`/ark-of-the-covenant`), from Marketing's spec and
prototype.

- **The spec is data, checked against the text.** `objects/ark.json` gives every part and every
  number with the verse it comes from and the KJV and BSB words that state it, or marks it
  interpretive with a note ([objects/README.md](objects/README.md)). The check
  (`site/src/lib/objects/check.mjs`) finds each quote in its verse in the repository's own text,
  so the spec can't cite a verse for a number the verse doesn't hold. CI runs it, and so does the
  page's build.
- **Plain three.js, not Threlte.** Marketing's spec suggested Threlte. The model is procedural
  geometry with a timeline (the assembly), camera moves and picking, all imperative, which is
  what their prototype already did in three.js. Threlte would add a declarative layer and tie the
  scene to Svelte's version for none of that. `scene.ts` has no Svelte in it, and the page and
  the app's embed load the same module.
- **three.js loads only on the object's page, after it paints.** It is a 156 KB (gzipped) dynamic
  import. The reader's service worker precaches every immutable file, so a build plugin
  (`viewerFiles` in `site/vite.config.ts`) names the viewer's chunks for the worker to skip, and
  reading offline never downloads them. A chapter page grew by 1.7 KB of gzipped script for the
  glyph.
- **Hotspots read in the reader's translation without the core.** The 25 cited verses in each
  bundled translation are prerendered into `verses.json` (26 KB, 4 KB gzipped); only a
  translation imported into the browser loads the core. The page's HTML carries the KJV, the
  text search engines index.

## What was traded away

- Highlights of part of a verse fall back to the whole verse in a translation that words it
  differently, and the iPhone app shows them whole-verse for now.
- No footnotes or cross-references from the source texts (dropped at import).
- SWORD modules in versifications other than KJV(A)/NRSV(A) (e.g. Vulgate, Synodal) aren't supported yet;
  deuterocanonical books are skipped everywhere to keep the 66-book grid.
- Search covers locally available text; online translations are searched via their own sites.
