# Proposing a correction

If a number, a part or a note in one of these specs looks wrong, say so with the verse.

1. **Open an issue** on [GitHub](https://github.com/michaelshimeles/bible-rs/issues) with the
   object, the part and the reference, or **open a pull request** that edits the JSON file.
2. **Give the basis.** A number stated in the text needs its verse (`ref`) and the exact words
   that give it in the KJV and the BSB (`quote`). A number the text doesn't give needs
   `"interpretive": true` and a `note` saying so. If scholars read a passage more than one way,
   add the readings as an option rather than replacing one with another. [README.md](README.md)
   describes every field.
3. **Run the check**: `node site/src/lib/objects/check.mjs` (after `npm install` in `site/`). It
   finds each quote in its verse and every reference in the repository's KJV and BSB. CI runs it
   too.
4. **Preview it** with `npm run dev` in `site/` and open
   <http://localhost:5173/ark-of-the-covenant>. The model and the page are built from the file.

The specs follow the text of the translations in `data/translations/`. Please don't add text
from copyrighted translations, even as a quote.
