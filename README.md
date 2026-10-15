# bible-rs

A very fast, native Bible reader for the Mac, written in Rust. Just the Bible — beautifully
typeset, searchable as you type, with highlights, notes and bookmarks kept on your computer.

![Reading John 1 in the King James Version](docs/screenshots/reader-light.png)

- **Instant.** First text on screen in about a tenth of a second; full-text search across the whole
  Bible in well under a millisecond for most queries; scrolling costs ~0.2 ms of CPU per frame.
- **Typeset for reading.** Literata serif, a comfortable measure, superscript verse numbers,
  chapter and section headings, poetry with hanging indents, LORD in small capitals, optional red
  letters, paragraphs or verse-per-line, adjustable size, light / sepia / dark.
- **Seven public-domain translations built in** (KJV, WEB, BSB, ASV, YLT, Darby, Webster), side by
  side in parallel columns or one verse across all of them. Bring your own licensed translations
  (NLT, NASB, LSB, ESV, …) by importing a file you own or fetching with your own API key.
- **Keyboard-first.** ⌘K jumps anywhere (`jn 3:16`, `Rom 8`, `1co13`, even `revalation 21`),
  ← → turn chapters, ⌘F searches, ⌘[ ⌘] go back and forward, 1–5 highlight.
- **Yours.** Highlights, notes and bookmarks live in a local SQLite file and export to JSON or
  Markdown. No accounts, no feeds, no network unless you add an online translation.

<table>
<tr>
<td><img src="docs/screenshots/psalm-sepia.png" alt="Psalm 23 in the Berean Standard Bible, sepia theme"></td>
<td><img src="docs/screenshots/dark-verse-per-line.png" alt="Matthew 5 in dark theme, verse per line, red letters"></td>
</tr>
<tr>
<td><img src="docs/screenshots/search.png" alt="Instant search with highlighted hits"></td>
<td><img src="docs/screenshots/jump-palette.png" alt="⌘K jump palette"></td>
</tr>
<tr>
<td><img src="docs/screenshots/compare.png" alt="Compare one verse across every translation"></td>
<td><img src="docs/screenshots/study-notes.png" alt="Notes, highlights and bookmarks"></td>
</tr>
<tr>
<td><img src="docs/screenshots/books.png" alt="Book and chapter picker"></td>
<td><img src="docs/screenshots/settings-translations.png" alt="Translations: bundled, imported and online"></td>
</tr>
</table>

![Psalm 1 in three parallel translations](docs/screenshots/parallel.png)

## Features

**Reading**
- Continuous scrolling through all 1,189 chapters — only what is on screen is ever typeset.
- Paragraph mode (verses flow inside paragraphs, first-line indents) or verse-per-line (numbers
  in the margin); poetry keeps its line structure in both.
- Book titles, chapter headings, section headings and Psalm superscriptions where the translation
  has them; words of Christ in red (⌘R); KJV translator additions in italics.
- Text size (⌘+ ⌘− ⌘0), line spacing, line length, justification, verse numbers and headings are
  all adjustable; light, sepia and dark themes (⌘⇧L).
- Remembers where you were.

**Navigation**
- ⌘K palette: references with fuzzy book names and typos, recent places, translations, commands.
- Book/chapter picker in the toolbar and the sidebar (⌘B).
- Back/forward history (⌘[ ⌘], ⌥← ⌥→), previous/next chapter (← →).

**Search** (⌘F)
- Results update as you type; every word must appear (word prefixes by default, or whole words).
- `"exact phrase"`, `-excluded` words, whole Bible / Old Testament / New Testament / one book.
- Matches highlighted in the results; ↑ ↓ and ⏎ jump to the verse, which flashes.

**Study**
- Click a verse (⇧-click for a range, J/K to move) to select it, then: highlight in five colours
  (1–5, 0 to clear), write a note (N), bookmark (B / ⌘D), copy with reference (⌘C) or compare
  translations (C). Right-click offers the same.
- Notes and bookmarks show in the margin; the Study sidebar lists everything.
- Export to Markdown (⌘E) or JSON (⌘⇧E); JSON imports back.

**Translations**
- Instant switching (⌘1…⌘9 or the toolbar), parallel columns (⌘⇧P) aligned verse by verse, and a
  compare view showing one verse in every installed translation.

**Mac**
- Native menu bar, unified title bar, app icon, `scripts/bundle-macos.sh` → `.app` + `.dmg`.

## Translations

### Bundled — all public domain

| | Translation | Notes |
|---|---|---|
| KJV | King James Version (1769) | Red letters, italics for translator additions, small-caps LORD. Public domain except in the UK (Crown letters patent). |
| WEB | World English Bible | Modern English, red letters. Public domain; "World English Bible" is a trademark of eBible.org (text unaltered). |
| BSB | Berean Standard Bible | Modern English with section headings. Dedicated to the public domain in 2023. |
| ASV | American Standard Version (1901) | |
| YLT | Young's Literal Translation (1898) | |
| DBY | Darby Translation (1890) | |
| WBS | Webster Bible (1833) | |

All seven come from eBible.org's USFM editions; download URLs and SHA-256 digests are pinned in
[`xtask/src/main.rs`](xtask/src/main.rs) and the data is rebuilt with `cargo run -p xtask --
build-data`. Provenance, licence statements and what was deliberately left out are in
[`LICENSES/`](LICENSES/README.md).

**Copyrighted translations (NLT, NASB, LSB, ESV, NIV, …) are never part of this repository** —
no text, no derived index, no test fixtures. You can add them yourself in either of two ways.

### Import a translation you own

Settings (⌘,) → Translations → **Import file…** / **Import folder…** (or ⌘I), or from a terminal:

```sh
bible-rs import ~/Downloads/MyBible.zip --abbrev NASB --name "New American Standard Bible"
```

Supported formats:

| Format | What to point at |
|---|---|
| **USFM** | a `.usfm`/`.sfm` file, a folder of them, or a `.zip` |
| **OSIS XML** | the `.xml`/`.osis` file |
| **Zefania XML** | the `.xml` file |
| **SWORD module** | the module `.zip`, its unpacked folder, or its `mods.d/*.conf` file. zText/RawText drivers in KJV, KJVA, NRSV or NRSVA versification; OSIS, ThML and GBF markup. **Locked modules** (e.g. purchased from a publisher) work with your unlock key: enter it in the *SWORD unlock key* field (or `--key`). |

The text is converted to bible-rs's format and stored only in your data folder
(`~/Library/Application Support/bible-rs/translations/` on macOS). Deuterocanonical books are
skipped. Remove an import from the same settings page or with `bible-rs remove <id>`.

### Online translations with your own API key

| Service | Translations | Get a key |
|---|---|---|
| **ESV API** (Crossway) | English Standard Version | <https://api.esv.org/> (free for non-commercial use) |
| **API.Bible** (American Bible Society) | Whatever your plan and the publishers allow — e.g. NASB, LSB, NLT, NKJV, AMP | <https://scripture.api.bible/> |

1. Settings (⌘,) → Translations → paste the key → **Save**. Keys are stored in the **macOS
   Keychain** (on Linux, a `secrets.json` readable only by you).
2. ESV: click **Add ESV**. API.Bible: click **List my Bibles**, filter, and **Add** the ones you want.
3. Pick the translation like any other. Chapters are fetched as you read and shown with the
   publisher's copyright notice.

What is stored locally follows each API's terms:

- **ESV API** — at most 500 verses and never more than half of any book are kept on disk (oldest
  chapters are evicted; a one-chapter book is shown but not stored); requests are spaced at least a
  second apart (the limit is 60 a minute); Crossway's copyright notice accompanies the text.
- **API.Bible** — fewer than 500 consecutive verses are cached and nothing older than 14 days is
  used; the copyright notice returned with each chapter is shown; each displayed chapter is
  reported through API.Bible's Fair Use Management System (FUMS) with an anonymous device id.

Full-text search covers local translations (bundled and imported); online translations are read
chapter by chapter and appear in parallel and compare views as their chapters load.

## Keyboard shortcuts

| | |
|---|---|
| ⌘K | Jump to a reference, translation or command |
| ⌘F or / | Search |
| ← / → | Previous / next chapter |
| ↑ ↓, Space, ⇧Space, PgUp/PgDn | Scroll |
| ⌘[ / ⌘] (⌥← / ⌥→) | Back / forward |
| Click, ⇧-click, J / K | Select a verse, a range, next / previous verse |
| 1–5, 0 | Highlight colour, remove highlight |
| N · B or ⌘D · ⌘C · C | Note · bookmark · copy with reference · compare |
| ⌘1…⌘9 | Switch translation |
| ⌘⇧P | Parallel translations |
| ⌘L · ⌘R · ⌘⇧L | Verse-per-line · red letters · theme |
| ⌘+ ⌘− ⌘0 | Text size |
| ⌘B · ⌘, · ⌘/ | Sidebar · settings · shortcut help |
| ⌘E · ⌘⇧E · ⌘I | Export Markdown · export JSON · import translation |

## Performance

PERFORMANCE_TABLE

Reproduce with:

```sh
cargo bench -p bible-core          # search, decoding, references, SQLite
cargo bench -p bible-rs            # chapter typesetting
scripts/bench-startup.sh           # launch → first frame, and a 900-frame scroll run
bible-rs search "love one another" # prints the query time
```

## Building

Requires Rust 1.85+ (edition 2024).

```sh
# Linux needs: libxkbcommon-dev libgl1-mesa-dev libwayland-dev libx11-dev
cargo build --release
./target/release/bible-rs
```

On macOS, `scripts/bundle-macos.sh` builds `dist/bible-rs.app` and `dist/bible-rs-<version>.dmg`
(`UNIVERSAL=1` for arm64 + x86_64, `SIGN_IDENTITY=…` to sign). CI builds the DMG on every push
and uploads it as an artifact.

Other tasks:

| | |
|---|---|
| `cargo test --workspace` | core (model, format, importers, search, references, study, online with a mock server) and typesetting tests |
| `cargo run -p xtask -- build-data` | re-download the pinned public-domain sources and regenerate `data/translations/*.bbl` |
| `scripts/screenshots.sh` | regenerate `docs/screenshots/` under Xvfb |
| `BIBLE_RS_HOME=/some/dir bible-rs` | use another data folder |

The binary doubles as a small CLI:

```sh
bible-rs read "jn 3:16-18" --translation web
bible-rs search '"in the beginning"' --scope nt
bible-rs list
bible-rs import path/to/module.zip [--abbrev X --name "…" --key UNLOCK]
bible-rs export --markdown -o notes.md
bible-rs import-notes notes.json
```

## How it's built

```
crates/core   bible-core: canonical book grid, verse-indexed model, .bbl format, USFM/OSIS/
              Zefania/SWORD importers, search, reference parsing, SQLite study store,
              ESV/API.Bible providers, Keychain secrets, translation library
crates/app    bible-rs: egui app — reader (typesetting, continuous scroll), sidebar, palette,
              panels, settings, macOS menu bar, CLI
xtask         reproducible bundled-data build from pinned public-domain sources
data          generated .bbl files (public domain only)
```

See [DECISIONS.md](DECISIONS.md) for why egui, how typesetting and continuous scrolling work, the
data format, search, and the licensing approach.

## License

Code: MIT ([LICENSE](LICENSE)). Bundled translations: public domain ([LICENSES/](LICENSES/README.md)).
Fonts: Literata and Inter, SIL Open Font License 1.1.
