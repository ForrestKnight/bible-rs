# Bible for Linux

The Linux app is the Mac app built for Linux. It has the same reader, search, study tools,
themes and data format, from the same code. It runs on Wayland and X11, on x86_64 and aarch64.

## Install

1. Download `bible-rs-<version>-linux-<arch>.tar.gz` from the
   [latest release](https://github.com/michaelshimeles/bible-rs/releases/latest) (`<arch>` is
   what `uname -m` prints: `x86_64` or `aarch64`) and its SHA-256 next to it.
2. Unpack it and run `./install.sh` inside. It puts the binary, a desktop entry and the icons
   under `~/.local`, no root needed, so **Bible** is in the app launcher and `bible-rs` on the
   command line (if `~/.local/bin` is on your PATH). `sudo PREFIX=/usr/local ./install.sh`
   installs for every user.

`./install.sh --uninstall` removes exactly those files. It keeps your notes, highlights,
settings and API keys, as dragging the Mac app to the Trash does.

The binary carries the translations and fonts; at run time it needs the desktop's Wayland (or
X11), xkbcommon and OpenGL libraries, which every desktop installation already has (on Arch:
`wayland`, `libxkbcommon`, `libglvnd`; on Debian and Ubuntu: `libwayland-client0`,
`libxkbcommon0`, `libgl1`).

## Where things live

| What | Where |
|---|---|
| The app | `~/.local/bin/bible-rs` (or `$PREFIX/bin`) |
| Desktop entry and icons | `~/.local/share/applications/bible-rs.desktop`, `~/.local/share/icons/hicolor/*/apps/bible-rs.png` |
| Notes, highlights, bookmarks, settings (`study.sqlite`), imported translations, backups | `~/.local/share/bible-rs` (or `BIBLE_RS_HOME`) |
| API keys (ESV, API.Bible) | `~/.local/share/bible-rs/secrets.json`, readable by you alone (mode 0600) |

## How it differs from the Mac app

| | Mac | Linux |
|---|---|---|
| Window | Toolbar in the title bar, traffic lights | The compositor's decorations, if it draws any; tiling compositors draw none |
| Menus | Menu bar at the top of the screen | No menu bar: the toolbar, the Ctrl+K palette and the shortcuts (Ctrl+/ lists them) reach everything |
| Shortcuts | ⌘K, ⇧⌘P… | Ctrl+K, Ctrl+Shift+P…, shown that way everywhere in the app. Focus mode is F11 (⌃⌘F on the Mac; ⌃ and ⌘ are the same Ctrl key on Linux) |
| Share Verse | Share… opens the system share picker | Copy Image puts the card on the clipboard; Save Image… saves a PNG |
| Updates | Sparkle checks daily and installs | Settings › About › Check for Updates… opens the releases page in your browser. Nothing checks or installs by itself |
| API keys | Keychain | A file only you can read (above) |
| Rendering | Metal | OpenGL; `BIBLE_RS_RENDERER=wgpu` for Vulkan |
| Dark mode | "Match system appearance" follows macOS | Follows the desktop's colour scheme through the settings portal (`xdg-desktop-portal`) |
| Text | Inter and Literata, bundled | The same fonts, bundled |

The command line works as on the Mac: `bible-rs search "love one another"`, `bible-rs read "jn 3:16"`.

## Build

Rust 1.95+ and the development packages of the libraries above:

```sh
# Arch:          sudo pacman -S --needed libxkbcommon mesa wayland libx11
# Debian/Ubuntu: sudo apt install libxkbcommon-dev libgl1-mesa-dev libwayland-dev libx11-dev
cargo build --release -p bible-rs
./target/release/bible-rs
```

`scripts/package-linux.sh` builds the archive in `dist/` (`BIN=path/to/bible-rs` to package a
binary built already). CI builds the x86_64 archive on every push and uploads it as a workflow
artifact; on a version tag `release.yml` builds both architectures and attaches them to the draft
release next to the Mac files ([RELEASING.md](RELEASING.md)).

The icons are rendered from the logo master by `scripts/make-icons.py`, into
`packaging/linux/icons/hicolor/`, one PNG per size the icon theme wants.
