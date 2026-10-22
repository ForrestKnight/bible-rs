# Releasing Bible for macOS

The iPhone and iPad app goes through the App Store instead; see
[apps/ios/APP_STORE.md](apps/ios/APP_STORE.md).

`scripts/release-macos.sh` builds everything on a Mac in one command. It writes to `dist/`:

| File | What it is |
|---|---|
| `bible-rs-<version>-universal.dmg` | Disk image: the app, an Applications link and the install background |
| `bible-rs-universal.dmg` | The same disk image under the name the website links to |
| `bible-rs-<version>-universal.zip` | The app alone; installed copies update from it |
| `bible-rs-<version>-SHA256SUMS.txt` | SHA-256 of the three files above |
| `appcast.xml` | The update feed: this version, its zip and the zip's signature, signed itself |
| `Bible.app` | The app itself (universal, signed, stapled) |

The app runs on macOS 11 or later, on Apple silicon and Intel. The script signs it with the
certificate "Developer ID Application: Fabrika LLC (5S3KCRYBD2)" and turns on the hardened runtime.
The app needs no entitlements; `packaging/macos/entitlements.plist` says why. The repository, the
Cargo package, the command-line binary and the release file names keep the name bible-rs.

## One-time setup

1. **Signing certificate.** `security find-identity -v -p codesigning` must list
   `Developer ID Application: Fabrika LLC (5S3KCRYBD2)`. To sign with another identity, set
   `SIGN_IDENTITY="…"`.
2. **Tools.** Xcode or its command line tools, rustup, and [uv](https://docs.astral.sh/uv/) or
   Python 3.10+. The script adds the two Rust targets. It installs dmgbuild, the tool that lays out
   the DMG, into `target/dmgbuild`, pinned by hash in `packaging/macos/dmgbuild-requirements.txt`.
3. **Notarization credentials.** Apple must notarize the app, or other Macs refuse to open it. The
   script uses a notarytool profile named `bible-notary` from the login keychain. Create it once;
   notarytool prompts for the secret and stores it in the keychain, never in the repository.

   With an Apple ID and an app-specific password, which you create at https://account.apple.com
   under Sign-In and Security:

   ```sh
   xcrun notarytool store-credentials bible-notary --apple-id <apple-id-email> --team-id 5S3KCRYBD2
   ```

   Or with an App Store Connect API key with the Developer role, made under Users and Access,
   Integrations, Team Keys:

   ```sh
   xcrun notarytool store-credentials bible-notary --key AuthKey_<KEYID>.p8 --key-id <KEYID> --issuer <issuer-id>
   ```

   Without the profile, `scripts/release-macos.sh` stops before building and prints these commands.
4. **Update signing key.** Installed copies update themselves with
   [Sparkle](https://sparkle-project.org) and install only zips signed with the private half of
   the `SUPublicEDKey` in `scripts/bundle-macos.sh`. That key lives in the login keychain under the
   account `bible-rs` (item "Private key for signing Sparkle updates"); the release script reads
   it there and stops if it is missing or different. Keep a copy somewhere safe, and move it to a
   new Mac the same way:

   ```sh
   S="$(scripts/fetch-sparkle.sh)"
   "$S/bin/generate_keys" --account bible-rs -x bible-rs-sparkle.key   # export
   "$S/bin/generate_keys" --account bible-rs -f bible-rs-sparkle.key   # import on another Mac
   ```

   Never generate a new key for this app: copies already installed would refuse updates signed
   with it.

## Making a release

1. Set the new `version` under `[workspace.package]` in `Cargo.toml`, merge it to `main`, and
   check out a clean `main`.
2. Run `scripts/release-macos.sh`. It builds the universal app, checks its architectures, minimum
   macOS version, Info.plist and signature, runs the signed binary, notarizes and staples the app,
   builds and signs the DMG, notarizes and staples it, zips the app, writes the checksums and
   writes and signs the update feed. It ends by asking Gatekeeper about the app and the DMG; both
   must say
   `source=Notarized Developer ID`. Expect about two minutes, plus a few minutes for each of
   Apple's two notarization checks.
3. Tag the commit and push the tag:

   ```sh
   git tag -a v<version> -m "bible-rs <version>"
   git push origin v<version>
   ```

4. Create the release as a draft, check it, then publish it on GitHub:

   ```sh
   gh release create v<version> --draft --verify-tag --generate-notes --title "Bible <version>" \
     dist/bible-rs-<version>-universal.dmg dist/bible-rs-universal.dmg \
     dist/bible-rs-<version>-universal.zip dist/bible-rs-<version>-SHA256SUMS.txt dist/appcast.xml
   ```

The website's Download button (`simplymybible.com/download`) redirects to
`https://github.com/michaelshimeles/bible-rs/releases/latest/download/bible-rs-universal.dmg`, and
installed copies read
`https://github.com/michaelshimeles/bible-rs/releases/latest/download/appcast.xml`. GitHub serves
both from the newest published release, so every release must include `bible-rs-universal.dmg`
and `appcast.xml`. Publishing a release switches the site and every installed copy to it at once;
a draft or a pre-release reaches neither. The feed's enclosure points at
`releases/download/v<version>/…`, so the tag must be `v<version>`.

To replace a published release's files, build the same version, upload with `--clobber`, then
edit the release notes:

```sh
gh release upload v<version> dist/bible-rs-universal.dmg dist/bible-rs-<version>-universal.dmg \
  dist/bible-rs-<version>-universal.zip dist/bible-rs-<version>-SHA256SUMS.txt dist/appcast.xml --clobber
```

## Updates

Sparkle checks the feed once a day without asking first (`SUEnableAutomaticChecks`), and Bible ›
Check for Updates… (also in Settings › About) checks at once. When the feed has a newer version, it
shows the update window. Installing downloads the zip, checks its signature against `SUPublicEDKey`
and that it is signed by the same Developer ID team, replaces the app where it is installed and
relaunches it.

Up to 0.1.0 the app was `bible-rs.app` and had no updater; its "Check for Updates…" opens the latest
release's page. Someone on 0.1.0 downloads the DMG and drags `Bible.app` into Applications, which
leaves `bible-rs.app` beside it. The first time `Bible.app` runs from `/Applications` or
`~/Applications` it moves a `bible-rs.app` in the same folder with the same bundle ID to the Trash.
The bundle ID, the data folder (`~/Library/Application Support/bible-rs`) and the keychain items
are unchanged, so notes, settings and saved API keys carry over.

Other options:

- `scripts/release-macos.sh --skip-notarize` builds signed files without notarizing. They open on
  this Mac but Gatekeeper blocks them on others; use it to test the build.
- `scripts/release-macos.sh --ad-hoc` signs without a Developer ID. CI uses it on every push.
- `ALLOW_DIRTY=1` builds from a working tree with uncommitted changes.

## GitHub Actions

`.github/workflows/release.yml` runs the same script when someone pushes a `v*` tag, then
attaches the files to a draft release. Publishing stays manual. If a draft for the tag already
exists, the workflow replaces its files; it never touches a published release. Build locally
while GitHub Actions billing is broken. The workflow needs these repository secrets.
`gh secret set NAME` prompts for each value, so it stays out of shell history.

| Secret | Value |
|---|---|
| `DEVELOPER_ID_P12` | The Developer ID Application certificate and private key, exported from Keychain Access as a .p12, then base64: `base64 -i developer-id.p12 \| gh secret set DEVELOPER_ID_P12` |
| `DEVELOPER_ID_P12_PASSWORD` | The password chosen during that export |
| `NOTARY_API_KEY` | The contents of the App Store Connect API key file: `gh secret set NOTARY_API_KEY < AuthKey_<KEYID>.p8` |
| `NOTARY_API_KEY_ID` | That key's ID |
| `NOTARY_API_ISSUER` | The issuer ID shown above the list of keys |
| `SPARKLE_PRIVATE_KEY` | The update signing key: `"$(scripts/fetch-sparkle.sh)/bin/generate_keys" --account bible-rs -x key.txt && gh secret set SPARKLE_PRIVATE_KEY < key.txt && rm key.txt` |

Delete the exported .p12 once the secret is set. The workflow imports the certificate into a
temporary keychain and deletes that keychain when it finishes.

## Notes

- **Keychain.** API keys stay in the login Keychain. Each item trusts the app's designated
  requirement (bundle ID `dev.bible-rs.app`, team 5S3KCRYBD2), so every release signed this way
  reads them without a prompt. Changing the bundle ID or the signing team makes every user approve
  access once. A key saved by an ad-hoc signed development build asks for approval once, the first
  time a release build reads it.
- **Ad-hoc builds.** The hardened runtime loads only frameworks signed by the app's own team, so an
  ad-hoc signed build (CI, `--ad-hoc`) can't load Sparkle; its Check for Updates… opens the latest
  release's page.
- **Rejected notarization.** `scripts/notarize-macos.sh` prints Apple's log, which names each
  file that failed and why.
- **Intel testing.** On an Apple silicon Mac without Rosetta, the script checks the x86_64 slice's
  architecture and minimum macOS version but cannot run it. `softwareupdate --install-rosetta`
  lets it run that slice too.
