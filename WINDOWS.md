# Bible for Windows

The Windows app is the Mac app built for Windows. It has the same reader, search, study tools,
themes and data format, from the same code. It runs on 64-bit Windows 10 and Windows 11.

Builds are **unsigned** for now. CI makes an installer on every push, as a workflow artifact;
nothing is attached to GitHub Releases yet. Signing needs a certificate that only Fabrika LLC can
get ([Signing](#signing)).

## Install

1. Open a recent run of the **CI** workflow on GitHub (Actions › CI), scroll to **Artifacts** and
   download **bible-rs-windows-x64-unsigned**. It holds
   `bible-rs-<version>-windows-x64-setup.exe` and its SHA-256.
2. Run the installer. Because it is unsigned, SmartScreen says "Windows protected your PC".
   Choose **More info**, then **Run anyway**.
3. The installer asks for no administrator rights. It installs for the current user into
   `%LOCALAPPDATA%\Programs\Bible`, adds **Bible** to the Start menu and to Settings › Apps ›
   Installed apps (publisher Fabrika LLC), and can open the app when it finishes.

Uninstall from Settings › Apps, or with `"%LOCALAPPDATA%\Programs\Bible\Uninstall Bible.exe"`.
Uninstalling keeps your notes, highlights, settings and API keys, as dragging the Mac app to the
Trash does.

The installer also runs unattended: `bible-rs-<version>-windows-x64-setup.exe /S` installs
silently, `/D=C:\Some\Folder` (last on the command line) picks the folder.

## Where things live

| What | Where |
|---|---|
| The app | `%LOCALAPPDATA%\Programs\Bible\bible-rs.exe` |
| Notes, highlights, bookmarks, settings (`study.sqlite`), imported translations, backups | `%APPDATA%\bible-rs` (or `BIBLE_RS_HOME`) |
| API keys (ESV, API.Bible) | Windows Credential Manager, generic credentials named `<key>.bible-rs` |
| Last update check | `%APPDATA%\bible-rs\update-check` |

## How it differs from the Mac app

| | Mac | Windows |
|---|---|---|
| Window | Toolbar in the title bar, traffic lights | Standard title bar, drawn light or dark to match the app's theme |
| Menus | Menu bar at the top of the screen | Menu bar in the window: File, View, Go, Help. Alt and the underlined letters open it; focus mode hides it |
| Shortcuts | ⌘K, ⇧⌘P… | Ctrl+K, Ctrl+Shift+P…, shown that way everywhere in the app. Focus mode is F11 (⌃⌘F on the Mac; ⌃ and ⌘ are the same Ctrl key on Windows) |
| Share Verse | Share… opens the system share picker | Copy Image puts the card on the clipboard; Save Image… saves a PNG |
| Updates | Sparkle checks daily and installs | The app asks GitHub for the latest release once a day, and when you choose Help › Check for Updates…. If it's newer, it offers the installer (or the release page) in your browser. Nothing installs by itself |
| API keys | Keychain | Credential Manager |
| Rendering | Metal | Direct3D 12 (Windows' software renderer if there is no GPU driver) |
| Dark mode | "Match system appearance" follows macOS | Follows Settings › Personalization › Colors |
| Reduce motion | Accessibility › Display › Reduce motion | Accessibility › Visual effects › Animation effects off |
| Text | Inter and Literata, bundled | The same fonts, bundled |

The command line works too (`bible-rs.exe search "love one another"`). The app is a windowed
program, so in cmd and PowerShell the prompt comes back before the output appears; pipe it
(`bible-rs.exe list | more`) to wait for it.

## Build

### On Windows

1. Install [Rust](https://rustup.rs) (the default MSVC toolchain), and the Visual Studio Build
   Tools with "Desktop development with C++" (the linker and `rc.exe`, which puts the icon and
   version information into the executable).
2. `cargo build --release -p bible-rs` builds `target\release\bible-rs.exe`.
3. For the installer, install [NSIS](https://nsis.sourceforge.io) 3 (`choco install nsis`) and, in
   Git Bash: `BIN=target/release/bible-rs.exe scripts/package-windows.sh`. It writes
   `dist/bible-rs-<version>-windows-x64-setup.exe` and `dist/bible-rs-<version>-windows-x64-SHA256SUMS.txt`.

`cargo test --workspace` and `cargo clippy --workspace --all-targets` work as on the other
platforms. `.cargo/config.toml` links the C runtime statically, so the app doesn't need the
Visual C++ Redistributable.

### On a Mac or Linux (cross-compiling)

```sh
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin            # downloads the Windows SDK headers and libraries on first use
brew install llvm lld makensis      # llvm-rc and lld-link; makensis builds the installer
export PATH="$(brew --prefix llvm)/bin:$(brew --prefix lld)/bin:$PATH"
cargo xwin build --release -p bible-rs --target x86_64-pc-windows-msvc
BIN=target/x86_64-pc-windows-msvc/release/bible-rs.exe scripts/package-windows.sh
```

`cargo check --target x86_64-pc-windows-gnu` also works with MinGW (`brew install mingw-w64`).
Without `llvm-rc` (or MinGW's `windres` for the -gnu target) a cross build still succeeds, with a
warning, but the executable has no icon or version information.

### CI

- The *Windows* job of **CI** (`.github/workflows/ci.yml`) runs clippy, the tests, a release
  build and the command-line search. It launches the app, which renders its first frame with
  Direct3D 12 and exits, then builds the unsigned installer, installs it silently, starts the
  installed app, uninstalls it, and uploads the installer as the artifact
  **bible-rs-windows-x64-unsigned**.
- On a version tag, the *windows* job of **Release** (`.github/workflows/release.yml`) uploads the
  installer as the artifact **bible-rs-v<version>-windows-x64-unsigned** (`…-windows-x64` once
  signing is on). It attaches nothing to the GitHub Release.

## Signing

Unsigned installers work, but SmartScreen blocks them until you choose "Run anyway", and some
company PCs refuse them. Signing with a certificate in Fabrika LLC's name removes the warning
(SmartScreen still learns a new certificate's reputation over the first downloads).

`scripts/package-windows.sh` signs when `WINDOWS_SIGN_COMMAND` is set. It runs that command on
`bible-rs.exe`, the uninstaller and the installer, with the file's path as the last argument, and
the command must sign the file in place. `scripts/sign-windows.sh` is the hook; it checks each signature
with `signtool verify /pa` when it can find signtool.

### Azure Artifact Signing (recommended)

Microsoft's signing service, formerly Azure Trusted Signing, is a monthly subscription. It needs no
hardware token, and Windows and SmartScreen trust its certificates. Micky has to do these steps; none can
be done from the repository:

1. **Azure.** In an Azure subscription, create an **Artifact Signing account** (pick a region and
   note its endpoint, such as `https://eus.codesigning.azure.net`). Check
   [eligibility](https://learn.microsoft.com/azure/artifact-signing/) first: Microsoft decides
   which organizations and countries can get public-trust certificates.
2. **Identity validation.** In the account, request **Public Trust** identity validation for
   Fabrika LLC and wait for Microsoft to approve it (it checks the business's records).
3. **Certificate profile.** Create a **Public Trust** certificate profile on that identity. Note
   the account name and the profile name.
4. **Who may sign.** In Microsoft Entra ID, register an app (for example "bible-rs release"),
   add a client secret, and on the Artifact Signing account give that app the
   **Artifact Signing Certificate Profile Signer** role.
5. **GitHub.** In the repository's Settings › Secrets and variables › Actions:
   - variables `WINDOWS_SIGNING` = `artifact-signing`, `ARTIFACT_SIGNING_ENDPOINT`,
     `ARTIFACT_SIGNING_ACCOUNT`, `ARTIFACT_SIGNING_PROFILE`;
   - secrets `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` (the app registration).
6. Push a version tag. The release workflow's *windows* job then downloads signtool and the
   Artifact Signing client (pinned NuGet packages), signs the app, the uninstaller and the
   installer, and uploads **bible-rs-v<version>-windows-x64**. Check it on a Windows PC. The
   installer's Properties › Digital Signatures tab should name Fabrika LLC.

To sign on a Windows PC instead, install the signing tools
(`winget install -e --id Microsoft.Azure.ArtifactSigningClientTools`), sign in with `az login`,
write a `metadata.json` with `Endpoint`, `CodeSigningAccountName` and `CertificateProfileName`, and
in Git Bash:

```sh
export WINDOWS_SIGN_COMMAND='signtool sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib "C:\path\to\Azure.CodeSigning.Dlib.dll" /dmdf "C:\path\to\metadata.json"'
scripts/package-windows.sh
```

The release workflow's signing step has not run yet, because there are no credentials to run it
with. The
hook itself was tested by signing all three files with a throwaway self-signed certificate and
`osslsigncode` as the command.

### A certificate from a certificate authority

An OV or EV code-signing certificate (on a hardware token or a cloud HSM) works with the same
hook, for example
`WINDOWS_SIGN_COMMAND='signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /sha1 <thumbprint>'`.

### After signing

To publish Windows builds, attach the installer to the release as
`bible-rs-<version>-windows-x64-setup.exe`. The app's update check looks for an asset whose name
ends in `-windows-x64-setup.exe` and offers the release page when there is none. The website's
download page links only the Mac disk image today.

## Known gaps

- **Not tried on a Windows 10 or 11 PC.** Development happened on a Mac, and every Windows run so
  far is on GitHub's Windows Server 2025 runner, which has no GPU and a 100% scale 1024x768 screen.
  There the tests passed, the installer installed and uninstalled, and the menu bar, F11, Copy
  Image and the update check worked when driven on the runner's desktop. Still unseen: a real GPU
  driver, high-DPI and mixed-DPI monitors, and the title bar on Windows 10.
- **Unsigned** until the steps above are done.
- **No `bible://` links yet.** Opening `bible://` links in the app is still an open pull request
  (#19); once it lands, the installer can register the scheme for the current user
  (`HKCU\Software\Classes\bible`).
- **Updates download, they don't install.** The app opens the new installer in the browser;
  running it upgrades in place.
- **Drop-down menus stay light in dark mode.** The menu bar follows the theme; Windows draws the
  menus under it in its light style.
- **Windows 10 before version 20H1** keeps a light title bar in the dark theme.
- **Menus show the default shortcuts** even if you change one in Settings › Shortcuts (the
  shortcut you chose is the one that works), as on the Mac.
- **x64 only.** Windows on Arm runs it through emulation (untried); there is no native Arm build.
- **No Windows share sheet.** Copy Image and Save Image… stand in for it.
