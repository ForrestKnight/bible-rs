# Bible on the App Store

How to get the iPhone and iPad app into the App Store, and what to paste into App Store Connect.
The repository holds everything that doesn't need an Apple account: the privacy manifest, the
release script, the screenshots, the privacy and support pages and the listing text below.
Creating the app's record, signing, uploading and submitting need the account, so they're steps
for its holder.

## Decide first

The App Store name is **Bible: Fast & Offline**. The name under the icon, `CFBundleDisplayName`,
stays "Bible". No live app in the US, UK, Canadian or Australian store used the name on 20
October 2026; App Store Connect confirms it's free when the record is created.

1. **The bundle ID.** It's `dev.bible-rs.ios`, set as `BIBLE_BUNDLE_ID` in
   `Config/Bible.xcconfig`, and it can't change once the record exists. The Mac app is
   `dev.bible-rs.app` and ships outside the store, so the two don't need to match.
2. **The version.** The phone app takes its version from `Cargo.toml`, like the Mac, so it would
   launch as 0.2.0. The store accepts that; set 1.0.0 there first if you'd rather launch at 1.0.
3. **The UK store.** The KJV is public domain everywhere except the UK, where the Crown's letters
   patent cover it. Either leave the UK out under Pricing and Availability, or keep it in, as the
   Mac download already does.
4. **A support email.** The support page sends people to GitHub issues, which needs a GitHub
   account. An email address there would reach everyone else.

## One-time setup

1. **The app's record.** In App Store Connect, Apps, +, New App: platform iOS, name "Bible: Fast & Offline", primary
   language English (U.S.), bundle ID `dev.bible-rs.ios`, SKU `bible-ios`. If the bundle ID isn't
   in the list yet, register it first under Certificates, Identifiers & Profiles, Identifiers, as
   an explicit App ID with no capabilities.
2. **Signing.** The team is Fabrika LLC, 5S3KCRYBD2, as on the Mac. The simplest route is to sign
   in to Xcode with the Apple ID under Xcode, Settings, Accounts. `scripts/release-ios.sh` then
   lets xcodebuild create the Apple Distribution certificate and the App Store profile on first
   use. Without Xcode signed in, make an App Store Connect API key with the Admin role under Users
   and Access, Integrations, Team Keys, and set `ASC_KEY_PATH`, `ASC_KEY_ID` and `ASC_ISSUER_ID`
   for the script. Keep the `.p8` file out of the repository.
3. **The website.** The listing links to `https://simplymybible.com/privacy` and `/support`. They
   go live when the `site/` changes reach `main`, the site deploys and the domain is attached to
   the Vercel project; check both open before submitting.

## Each release

1. Set the version in `Cargo.toml`, which the Mac app shares
   ([RELEASING.md](../../RELEASING.md)), and check out a clean `main`.
2. Run `scripts/release-ios.sh --upload`. It builds the Rust core for the device, archives the app
   with its debug symbols, signs it for App Store distribution and uploads it. The build number is
   the commit count, so every upload from a newer commit is higher; set `BUILD_NUMBER` to upload
   twice from one commit. Without `--upload` it leaves `dist/ios/Bible.ipa` for Transporter, and
   `--unsigned` only checks that the release build archives.
3. The build appears under TestFlight once Apple has processed it, usually within half an hour.
   Install it through TestFlight on an iPhone and an iPad and read for a while: rotate, change
   the text size, search, take notes.
4. On the App Store tab, add the version, pick the build, fill in What's New and submit for
   review.

## Listing

| Field | Text |
|---|---|
| Name | Bible: Fast & Offline |
| Subtitle | Seven translations, built in |
| Category | Reference (primary), Books (secondary) |
| Price | Free |
| Support URL | https://simplymybible.com/support |
| Marketing URL | https://simplymybible.com |
| Privacy Policy URL | https://simplymybible.com/privacy |
| Copyright | 2026 Micky |
| Keywords | `kjv,scripture,holy,verse,study,psalms,gospel,web,bsb,asv,devotional,notes,highlight,daily,christian` |

The keywords and subtitle leave out "bible", "fast" and "offline", because the store already
indexes the words in the name.

**Promotional text**, which you can change without a new version:

> The Bible, typeset to be read and searchable as you type. Seven translations built in, it works
> offline and keeps your notes on your device.

**Description:**

```text
Bible is the Bible and nothing else, typeset to be read. It opens where you left off and finds any word in the whole Bible as you type.

READ
• Set in Literata, a serif made for long reading, with poetry in its lines and the words of Jesus in red.
• Scroll from Genesis to Revelation without page breaks or waiting.
• Jump anywhere by typing a reference such as "jn 3:16" or "ps 23", or pick a book and chapter.
• Light, sepia and dark themes. The text follows your device's text size, and you can make it larger or smaller.

SEARCH
• Results appear as you type, with the matching words highlighted.
• Search the whole Bible, the Old or New Testament, or the book you're reading.
• Misspell a word and Bible suggests the one you meant.

STUDY
• Highlight verses in six colours, add notes and bookmarks, and find them all in Study.
• Copy or share a verse with its reference.
• Export your notes and highlights as Markdown or JSON.

SEVEN TRANSLATIONS, BUILT IN
King James Version, World English Bible, Berean Standard Bible, American Standard Version, Young's Literal Translation, Darby and Webster. All are in the public domain and work offline.

PRIVATE
No account, no ads, no analytics and no tracking. Bible never goes online, and your notes stay on your device.

Bible is also on the Mac: simplymybible.com
```

**What's New** for the first version: `The first release of Bible for iPhone and iPad.`

## Screenshots

`scripts/ios-screenshots.sh --app-store` takes them in simulators, in listing order, into
`target/ios/app-store-screenshots/`:

- `iphone-6.9-inch/`: 1320 × 2868 from an iPhone 17 Pro Max, for the 6.9-inch display set
- `ipad-13-inch/`: 2064 × 2752 from an iPad Pro 13-inch, for the 13-inch display set

App Store Connect scales these down for the smaller iPhones and iPads. It takes up to ten per set
and the script makes twelve, so upload `01` to `10`; the first three show in search results.

## App Store Connect answers

**App Privacy.** Answer "No, we do not collect data from this app", and the listing shows Data Not
Collected. The app has no account, analytics, ads or network access. `Bible/PrivacyInfo.xcprivacy`
tells Apple's tools the same. It also gives reasons for the two kinds of file API that SQLite
calls, file timestamps (C617.1) and disk space (E174.1).

**Encryption.** The Info.plist sets `ITSAppUsesNonExemptEncryption` to `NO`, so App Store Connect
doesn't ask on each upload. That holds because the phone build leaves out `bible-core`'s `tls`
feature, and the app contains no encryption code at all. `scripts/build-ios.sh` stops if TLS comes
back, because the answer would then change.

**Age rating.** Answer "None" or "No" to every question, which gives 4+. The app shares nothing
users write and has no web access, messaging or ads.

**Content rights.** Yes, the app shows third-party content: the seven translations. You have the
rights to it because they're all in the public domain. Point 3 above covers the KJV in the UK.

**App Review information.** No sign-in is needed. Notes for the reviewer:

```text
Bible is a reader for seven public-domain translations, all inside the app. It needs no account and makes no network connections.

To try it: tap the book name at the top left and type "jn 3:16" to jump there. Tap a verse to highlight it, add a note or bookmark, copy or share it. The magnifying glass searches the whole Bible as you type. The AA button changes the theme and text size. The bookmark button lists your highlights, notes and bookmarks.
```

Add your name, phone number and email as the review contact.

## Before submitting

- [ ] `/privacy` and `/support` open on simplymybible.com
- [ ] The TestFlight build works on a real iPhone and iPad, in both orientations
- [ ] Screenshots, description, keywords, URLs and the review notes are filled in for the version
- [ ] App Privacy, age rating, content rights and Pricing and Availability are answered
