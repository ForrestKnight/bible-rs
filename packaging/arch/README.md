# Arch package

`PKGBUILD` builds bible-rs from source at the pinned commit and installs the binary, the desktop
entry and the icons from `packaging/linux/`, the way the Linux archive's `install.sh` does but
through pacman, so `omarchy pkg drop bible-rs` removes it again.

```sh
cd packaging/arch
makepkg -si          # build, then install with pacman
```

The directory is laid out the way [omarchy-pkgs](https://github.com/omacom/omarchy-pkgs) wants a
recipe (`PKGBUILD` plus `.omarchy/package.json` naming where releases come from), so it can be
copied into `pkgbuilds/bible-rs/` there as a pull request when the time comes. Their checks need
every git source pinned (`#commit=`), which `_commit` is; bump it and `pkgrel` when the fork moves.

Once a release carries the Linux archives that `scripts/package-linux.sh` builds, the recipe can
switch to `source_x86_64` / `source_aarch64` tarballs and drop the Rust build, as `disktree-bin`
does there.
