# Object specs

Each file here describes one object the Bible gives measurements for, built only from the text.
The site makes the 3D model, the verse panel and the object's page from it. The first is
[`ark.json`](ark.json), the Ark of the Covenant from Exodus 25:10–22 and 37:1–9, shown at
[/ark-of-the-covenant](https://simplymybible.com/ark-of-the-covenant) and in the Mac app.

| File | What it is |
|---|---|
| `<id>.json` | An object: its parts, every number with its verse, and what is interpretive |
| `object.schema.json` | The JSON Schema every object follows |
| `cubits.json` | The cubits an object can be shown in, and the person drawn for scale |
| `cubits.schema.json` | Its schema |
| `LICENSE` | CC BY 4.0, for everything here |

To propose a correction, see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The specs here and the images rendered from their 3D models
(`site/static/ark-of-the-covenant/`) are licensed CC BY 4.0
([LICENSE](LICENSE), legal code in [`LICENSES/CC-BY-4.0.txt`](../LICENSES/CC-BY-4.0.txt)). The code
that reads them and generates the models' geometry is MIT, like the rest of the repository. Credit
them as:

> Ark of the Covenant specs and 3D models © 2026 Micky (github.com/michaelshimeles) / bible-rs contributors, licensed CC BY 4.0, https://github.com/michaelshimeles/bible-rs

## The rules

- **Every number names its basis.** A number the text gives has `"interpretive": false`, the
  verse it comes from (`ref`) and the words that give it in the KJV and the BSB (`quote`). A
  number the model has to choose has `"interpretive": true` and a `note` saying why.
- **Every part says whether the text gives its form.** `"interpretive": false` means the text
  gives its shape and size, as for the chest and the mercy seat. `true` means the model adds
  something the text leaves open, as with the cherubim's figures, and the page labels the part
  "Interpretive choice".
- **Debated readings are listed, not chosen silently.** A part's `options` names every reading,
  marks one as the default, and the viewer lets the reader switch between them. The Ark's rings
  sit low (the "feet", as the BSB reads it) or at the top corners.
- **References are OSIS**: `Exod.25.10`, `Heb.9.4`, `1Kgs.8.9`, or a run of verses in one
  chapter, `Exod.25.10-22`.

## Fields

An object:

| Field | Example | Meaning |
|---|---|---|
| `id` | `ark` | The file's name |
| `slug` | `ark-of-the-covenant` | The page's address, `/<slug>` |
| `title`, `summary` | | Its name, and one sentence on what it is |
| `license` | `CC-BY-4.0` | The spec's license, always CC BY 4.0 ([LICENSE](LICENSE)) |
| `sources` | `["Exod.25.10-22", "Exod.37.1-9"]` | The passages that describe it. The readers show the object's glyph after the first verse of each |
| `unit` | `cubit` | The unit of every length |
| `parts` | | See below |
| `assembly` | `{ "part": "rings", "ref": "Exod.25.12" }` | The order "Build from the text" brings the parts in, each with its verse as the caption. `"effect": "gild"` turns a part already built from wood to gold |

A part:

| Field | Example | Meaning |
|---|---|---|
| `id`, `label` | `rings`, `Four rings of gold` | |
| `interpretive` | `true` | Whether the model's form goes beyond the text |
| `note` | | What the text gives and what the model adds, shown with the part |
| `facts` | `{ "text": "Four rings of cast gold", "ref": "Exod.25.12", "quote": { "kjv": "cast four rings of gold", "bsb": "Cast four gold rings" } }` | What the text says about the part, verse by verse. The panel quotes these verses |
| `dims` | `"length": { "value": 2.5, "ref": "Exod.25.10", "interpretive": false, "quote": { ... } }` | The part's numbers by name, in cubits unless `"unit": "count"` |
| `options` | `{ "id": "position", "default": "feet", "choices": [...] }` | A debated reading; each choice has a label, a note and the numbers it sets (`values`) |
| `model` | `procedural:rings` | How the part is drawn: a generator in `site/src/lib/objects/scene.ts`, or `{ "file": "models/<id>/<part>.glb", "author": "...", "license": "..." }` for a glTF model |

## The check

`node site/src/lib/objects/check.mjs` validates each file against its schema, then checks what a
schema can't: that every reference is a verse the repository's KJV and BSB both have, that every
quote is in its verse in that translation, that a stated number is the one its quoted words give
("two cubits and a half" is 2.5), that ids are unique, that the assembly and the options name
real parts and choices, and that the parts and numbers `scene.ts` draws are there. It prints what it checked, or every problem and exits 1.
CI runs it on every pull request, and the site runs the same check while it builds the page, so
the site can't publish a wrong number.

Code draws the cherubim too (`procedural:cherub`), as fuller artistic figures that kneel, wear
robes and have feathered wings. The text gives their material, place, wings and gaze, not their
bodies or faces, so the part is interpretive.
