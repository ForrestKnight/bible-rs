# AUR package

`PKGBUILD` builds bible-rs from a release's source tarball and installs the binary, the desktop
entry and the icons from `packaging/linux/`, the way the Linux archive's `install.sh` does but
through pacman. It is the recipe for the [AUR](https://aur.archlinux.org), where any Arch user
installs it with `yay -S bible-rs` (or `paru`, or by cloning the AUR repository and running
`makepkg -si`).

Publishing a release:

```sh
cd packaging/arch
# pkgver in PKGBUILD must match the tag (v0.2.0 → 0.2.0); bump pkgrel only for a rebuild of
# the same version.
updpkgsums                              # fetch the tarball, fill in sha256sums
makepkg --printsrcinfo > .SRCINFO       # the AUR reads this, not the PKGBUILD
makepkg -si                             # build and install, to make sure
```

Then copy `PKGBUILD` and `.SRCINFO` into a clone of `ssh://aur@aur.archlinux.org/bible-rs.git`
and push. The AUR package has a maintainer of record: whoever first pushes it, until they
disown it in the web interface.
