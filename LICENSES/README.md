# Licences and provenance

bible-rs's own code is MIT-licensed (see `../LICENSE`). Everything else that ships inside the
binary is listed here with where it came from and why it may be redistributed, along with the
object specs, which are CC BY 4.0.

## Bundled Bible translations — all public domain

Each translation is converted from the eBible.org USFM edition by `cargo run -p xtask -- build-data`.
The download URL and SHA-256 of every source archive are pinned in `xtask/src/main.rs` and recorded
in each translation's metadata (shown in Settings → Translations). Footnotes, cross-references,
Strong's numbers and introductions are dropped; the verse text itself is unchanged.

| Id | Translation | Source (eBible.org id) | Status | Notes |
|---|---|---|---|---|
| `kjv` | King James Version (1769 Oxford standard text) | `eng-kjv2006` | Public domain | Outside the UK. In the United Kingdom the KJV is under perpetual Crown letters patent (printing/importing printed copies needs permission from the Queen's/King's Printers: Cambridge UP, Oxford UP, Collins). eBible.org: "This royal decree has no effect outside of the UK, where this work is firmly in the Public Domain." |
| `web` | World English Bible (2020 stable text, 66 books) | `engwebp` | Public domain | Dedicated to the public domain by its translators. "World English Bible" is a trademark of eBible.org; the name may only be used for unmodified text. bible-rs does not alter the words (it only omits footnotes), so the name is kept. |
| `bsb` | Berean Standard Bible | `engbsb` | Public domain | Dedicated to the public domain by BSB Publishing / the Berean Bible Translation Committee on April 30, 2023 ("This text of God's Word has been dedicated to the public domain"). |
| `asv` | American Standard Version (1901) | `eng-asv` | Public domain | Published 1901; copyright long expired. |
| `ylt` | Young's Literal Translation (1898) | `engylt` | Public domain | Robert Young; published 1862/1898. |
| `dby` | Darby Translation (1890) | `engDBY` | Public domain | J. N. Darby; published 1867/1890. |
| `wbs` | Webster Bible (1833) | `engwebster` | Public domain | Noah Webster's revision of the KJV. |

eBible.org's catalogue (`translations.csv`) and each archive's `copr.htm` state "Public Domain" for
every source above; copies of those statements are reproduced in [`bibles.md`](bibles.md).

### Considered and not bundled

* **Bible in Basic English** — eBible.org lists it as public domain, but its status outside the
  United States is disputed (Cambridge University Press published the 1965 edition), so it is
  left out.
* **Translations with Apocrypha/Deuterocanon or non-KJV versification** (Douay–Rheims, Brenton
  Septuagint, …) — public domain, but they don't fit the 66-book KJV verse grid used for parallel
  and compare views. Users can import them; deuterocanonical books are skipped.
* **NLT, NASB, LSB, ESV, NIV, NKJV and any other copyrighted translation** — never bundled: no
  text, no derived index, no test fixtures. They can be imported from a file the user owns or
  fetched with the user's own API key (see the README).

## Fonts — SIL Open Font License 1.1

| Font | Use | Source | Licence |
|---|---|---|---|
| Literata (variable, roman + italic) | Scripture text | github.com/google/fonts `ofl/literata` | [OFL-Literata.txt](../crates/app/assets/fonts/OFL-Literata.txt) |
| Inter (variable) | Interface | github.com/google/fonts `ofl/inter` | [OFL-Inter.txt](../crates/app/assets/fonts/OFL-Inter.txt) |
| Inter Medium, Inter SemiBold (static instances of the variable Inter, made with `fonttools varLib.instancer`, wght 500/600, opsz 14) | Interface, verse numbers | derived from the above | [OFL-Inter.txt](../crates/app/assets/fonts/OFL-Inter.txt) |

egui's default fonts (Ubuntu-Light, Hack, Noto Emoji, emoji-icon-font; OFL/Apache/MIT) remain as
fallbacks for symbols.

## Object specs and model images: CC BY 4.0

The object specs in `objects/` (`ark.json`, `cubits.json`, their schemas and docs) and the images
rendered from their 3D models (`site/static/ark-of-the-covenant/ark.jpg` and `og.jpg`; the app
carries `ark.jpg` and the specs) are licensed CC BY 4.0, unlike the MIT code that draws them. The
notice is [`objects/LICENSE`](../objects/LICENSE) and the legal code is
[`CC-BY-4.0.txt`](CC-BY-4.0.txt). Credit them as:

> Ark of the Covenant specs and 3D models © 2026 Micky (github.com/michaelshimeles) / bible-rs contributors, licensed CC BY 4.0, https://github.com/michaelshimeles/bible-rs

## Test fixtures

Every importer and online-provider test uses public-domain wording (mostly KJV) in the *format*
of the source being tested (OSIS, Zefania, SWORD, ESV API text, API.Bible JSON). No copyrighted
Bible text appears anywhere in this repository.
