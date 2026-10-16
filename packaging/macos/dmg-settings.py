# dmgbuild settings for the bible-rs disk image. scripts/release-macos.sh runs
#   dmgbuild -s packaging/macos/dmg-settings.py -D app=dist/bible-rs.app \
#     -D background=packaging/macos/dmg-background.png bible-rs out.dmg
#
# The background is the Paper file "Bible", page Distribution, artboard
# "DMG Background (640×420)", exported at 1x and 2x (the 2x file set to 144 dpi). dmgbuild
# finds dmg-background@2x.png next to the 1x file and merges the two into a Retina TIFF.
# dmgbuild writes Finder's window layout (.DS_Store) itself, so no Finder scripting is needed
# and it works headless.
import os.path

app = defines["app"]  # noqa: F821 (dmgbuild provides `defines`)
app_name = os.path.basename(app.rstrip("/"))

# lzma: 16 % smaller than zlib (UDZO) for this app; opens on macOS 10.15+, the app needs 11.
format = "ULMO"
filesystem = "HFS+"
files = [app]
symlinks = {"Applications": "/Applications"}
# No hide_extensions: it sets a Finder flag (com.apple.FinderInfo) on the bundle, which a drag to
# Applications copies along and `codesign --verify --strict` rejects. Finder hides ".app" anyway.
# The mounted volume shows the app's icon.
icon = os.path.join(app, "Contents", "Resources", "AppIcon.icns")

background = defines["background"]  # noqa: F821
# Finder's window bounds include the title bar: 448 pt leaves 416 pt of content under macOS 27's
# 32 pt title bar and 420 pt under older releases' 28 pt one. Trimming the image's blank bottom
# edge is harmless; a taller window would show a white strip below it.
window_rect = ((200, 120), (640, 448))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False

# Icon centres in points from the top left of the image, either side of its arrow
# (x 254 to 383, y about 192).
icon_size = 128
text_size = 13
icon_locations = {
    app_name: (160, 196),
    "Applications": (480, 196),
}
