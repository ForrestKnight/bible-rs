# Releasing bible-rs for macOS

`scripts/release-macos.sh` builds everything on a Mac in one command. It writes to `dist/`:

| File | What it is |
|---|---|
| `bible-rs-<version>-universal.dmg` | Disk image: the app, an Applications link and the install background |
| `bible-rs-universal.dmg` | The same disk image under the name the website links to |
| `bible-rs-<version>-universal.zip` | The app alone |
| `bible-rs-<version>-SHA256SUMS.txt` | SHA-256 of the three files above |
| `bible-rs.app` | The app itself (universal, signed, stapled) |

The app runs on macOS 11 or later, on Apple silicon and Intel. The script signs it with the
certificate "Developer ID Application: Fabrika LLC (5S3KCRYBD2)" and turns on the hardened runtime.
The app needs no entitlements; `packaging/macos/entitlements.plist` says why.

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

## Making a release

1. Set the new `version` under `[workspace.package]` in `Cargo.toml`, merge it to `main`, and
   check out a clean `main`.
2. Run `scripts/release-macos.sh`. It builds the universal app, checks its architectures, minimum
   macOS version, Info.plist and signature, runs the signed binary, notarizes and staples the app,
   builds and signs the DMG, notarizes and staples it, zips the app and writes the checksums. It
   ends by asking Gatekeeper about the app and the DMG; both must say
   `source=Notarized Developer ID`. Expect about two minutes, plus a few minutes for each of
   Apple's two notarization checks.
3. Tag the commit and push the tag:

   ```sh
   git tag -a v<version> -m "bible-rs <version>"
   git push origin v<version>
   ```

4. Create the release as a draft, check it, then publish it on GitHub:

   ```sh
   gh release create v<version> --draft --verify-tag --generate-notes --title "bible-rs <version>" \
     dist/bible-rs-<version>-universal.dmg dist/bible-rs-universal.dmg \
     dist/bible-rs-<version>-universal.zip dist/bible-rs-<version>-SHA256SUMS.txt
   ```

The website's Download button (`bible-rs.vercel.app/download`) redirects to
`https://github.com/michaelshimeles/bible-rs/releases/latest/download/bible-rs-universal.dmg`, which
GitHub serves from the newest published release. Every release must therefore include
`bible-rs-universal.dmg`, and the site switches to a new version the moment you publish it.

v0.1.0 went out signed but not notarized. To replace a published release's files, build the same
version, upload with `--clobber`, then edit the release notes:

```sh
gh release upload v0.1.0 dist/bible-rs-universal.dmg dist/bible-rs-0.1.0-universal.dmg \
  dist/bible-rs-0.1.0-universal.zip dist/bible-rs-0.1.0-SHA256SUMS.txt --clobber
```

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

Delete the exported .p12 once the secret is set. The workflow imports the certificate into a
temporary keychain and deletes that keychain when it finishes.

## Notes

- **Keychain.** API keys stay in the login Keychain. Each item trusts the app's designated
  requirement (bundle ID `dev.bible-rs.app`, team 5S3KCRYBD2), so every release signed this way
  reads them without a prompt. Changing the bundle ID or the signing team makes every user approve
  access once. A key saved by an ad-hoc signed development build asks for approval once, the first
  time a release build reads it.
- **Updates.** Settings › About › Check for Updates… opens the latest release page in the browser.
  The app makes no network requests of its own to check for updates.
- **Rejected notarization.** `scripts/notarize-macos.sh` prints Apple's log, which names each
  file that failed and why.
- **Intel testing.** On an Apple silicon Mac without Rosetta, the script checks the x86_64 slice's
  architecture and minimum macOS version but cannot run it. `softwareupdate --install-rosetta`
  lets it run that slice too.
