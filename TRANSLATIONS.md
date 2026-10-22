# Getting translations for free, legally

bible-rs only uses sources the publishers sanction: their own APIs, API.Bible (which licenses
texts on the publishers' behalf), or files under an open licence. It never scrapes websites. Text
that is still under copyright is never part of this repository. Checked October 2026; terms
change, so the publishers' pages are linked below.

| Translation | How to get it free in bible-rs | Key, limits and terms |
|---|---|---|
| KJV, WEB, BSB, ASV, YLT, Darby, Webster | Built in | Public domain. |
| **NLT** (Tyndale) | Settings → Translations → **Add NLT**. On the web reader, pick **NLT** in the translation menu (see below) | No key needed: 50 verses a request, 500 requests a day. An optional free key from [api.nlt.to](https://api.nlt.to/) raises that to 500 verses and 5,000 requests. Non-commercial use only, and consistent with [Tyndale's purpose](https://www.tyndale.com/purpose). At most 500 verses are kept on disk, and never a whole book. Tyndale's credit line appears under each chapter. |
| **NET Bible** (bible.org) | Settings → Translations → **Add NET** | No key needed ([labs.bible.org](https://labs.bible.org/api/)). Non-commercial use may quote the text in any form. "(NET)" and the acknowledgment appear under each chapter, linked to netbible.org. Requests are spaced 1 s apart, and chapters are kept for 30 days. |
| **ESV** (Crossway) | Settings → Translations → **Get Free Key…** opens [api.esv.org](https://api.esv.org/account/create-application/). Create an "API application" there, paste the key, then **Add ESV** | Non-commercial use only. 5,000 requests a day, 1,000 an hour, 60 a minute. At most 500 verses are stored, and never more than half a book. The notice under each chapter links to esv.org. |
| NASB 2020, NASB 1995, AMP (Lockman) | Your own [API.Bible](https://api.bible/) key → **List My Bibles…** → **Add** | Lockman has no API of its own. These Bibles are on API.Bible under its "Standard License". See the note on API.Bible below. |
| NKJV (HarperCollins) | API.Bible, as above | Standard License. HarperCollins has no public API. |
| CSB (Lifeway) | API.Bible, as above | Standard License. Lifeway has no public API. |
| GNT, CEV, The Message | API.Bible, as above | Standard License. |
| LSB (Three Sixteen / Lockman) | Not available free: import a copy you've bought | No official API, and it isn't on API.Bible. Its permission terms cap free use at 1,000 verses, including verses stored electronically. |
| NIV (Biblica) | Not available free: import a copy you've bought | API.Bible lists the NIV under a "Unique License", which means a separate agreement with Biblica rather than API.Bible's plans. |
| NRSVue | Not available free | Its fair-use guidelines exclude phone apps and other new media. App licences go through Petradi Rights Management. |
| CEB | Not available free | No API, and it isn't on API.Bible. Beyond 500 verses (or a whole book) it needs written permission and a fee. |
| LSV, Free Bible Version, Translation for Translators | Download the USFM zip from [eBible.org](https://ebible.org/), then **Import File…** | CC BY-SA 4.0. They could also be bundled later, with attribution. |
| Open English Bible | The same, from eBible.org or [openenglishbible.org](https://openenglishbible.org/) | CC0 (public domain). Not finished yet: the New Testament, the Psalms and some Old Testament books. |
| unfoldingWord ULT / UST | The same | CC BY-SA 4.0. These are translators' aids, not reading Bibles. |

**The NLT on the web.** The web reader (`/read`) fetches the NLT live through the site's
`/api/nlt`, which holds the site's own key. Each request asks Tyndale for one chapter or less. The
site refuses whole books and passages that cross chapters, and each IP address can ask for 20
chapters a minute and 300 a day. Tyndale receives only the passage, from the site's server.
Vercel's servers keep a chapter for a day, but nothing from a one-chapter book (Jude, say). The browser keeps only the
chapter on screen, in memory. The reader stores none of it, the offline cache skips it, and the
site's pages, sitemaps and link previews never contain it. Tyndale's credit line is under the chapter, linked to
[newlivingtranslation.com](https://www.newlivingtranslation.com). Highlights, notes and bookmarks
work as in other translations, but a highlight of some words colours the whole verse, so that no
NLT text is saved with it.

**API.Bible.** API.Bible's free Starter plan is for non-commercial use and covers up to three
licensed Bibles and 5,000 calls a month. Its Bibles page also says Standard License Bibles need a
paid plan, so check what your key lists. bible-rs keeps fewer than 500 consecutive verses and
refreshes them within 14 days. It reports each chapter you read through API.Bible's fair-use
system (FUMS), and shows the copyright API.Bible returns, followed by the citation and link its
Starter plan requires.

**Why the NET isn't built in.** The NET's terms let it be copied only if "free means free": it
"cannot be bundled with anything sold". bible-rs's licence lets anyone sell builds of the app, and
the repository never holds copyrighted text. So the NET is fetched instead, and eBible.org's NET
download is there for personal use. bible.org's API documentation is behind a bot check that
couldn't be read, so this follows the NET Bible copyright page, which the API defers to.

Sources: [NLT API](https://api.nlt.to/) and [Tyndale permissions](https://www.tyndale.com/permissions),
[ESV API terms](https://api.esv.org/), [NET Bible copyright](https://netbible.com/copyright),
[API.Bible](https://api.bible/) and its [terms](https://api.bible/terms-and-conditions),
[Lockman](https://www.lockman.org/permission-to-quote-copyright-trademark-information/),
[LSB](https://lsbible.org/permission-to-quote-the-lsb/), [CSB](https://csbible.com/permissions/),
[CEB](https://www.commonenglishbible.com/permissions),
[NRSVue FAQ](https://www.friendshippress.org/pages/nrsvue-quick-faq),
[eBible.org copyright list](https://ebible.org/Scriptures/copyright.php).
