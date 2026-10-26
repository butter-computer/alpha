#!/bin/sh
# Build target/release/Alpha-<version>-arm64.dmg: Alpha.app plus an /Applications
# symlink for drag-to-install. Runs bundle-mac.sh first (which ad-hoc signs the app).
set -eu

cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(uname -m)
APP=target/release/bundle/Alpha.app
DMG=target/release/Alpha-$VERSION-$ARCH.dmg
STAGE=$(mktemp -d /tmp/alpha-dmg.XXXXXX)
RW=$STAGE/rw.dmg
VOL=Alpha

cleanup() {
    hdiutil detach "/Volumes/$VOL" -force >/dev/null 2>&1 || true
    rm -rf "$STAGE"
}
trap cleanup EXIT

if [ -d "/Volumes/$VOL" ]; then
    echo "A volume named '$VOL' is already mounted; eject it first." >&2
    exit 1
fi

scripts/bundle-mac.sh

mkdir "$STAGE/src"
ditto "$APP" "$STAGE/src/Alpha.app"
ln -s /Applications "$STAGE/src/Applications"

# Read-write image first, so the Finder window layout can be written into it.
hdiutil create -quiet -srcfolder "$STAGE/src" -volname "$VOL" -fs HFS+ -format UDRW -ov "$RW"
hdiutil attach -quiet -readwrite -noverify -noautoopen "$RW"

# Window layout: best effort (needs permission to script Finder); skipped on failure.
if ! timeout 30 osascript >/dev/null 2>&1 <<OSA
tell application "Finder"
  tell disk "$VOL"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {200, 140, 740, 480}
    set opts to the icon view options of container window
    set icon size of opts to 112
    set arrangement of opts to not arranged
    set position of item "Alpha.app" of container window to {140, 160}
    set position of item "Applications" of container window to {400, 160}
    close
    update without registering applications
    delay 1
  end tell
end tell
OSA
then
    echo "note: Finder layout skipped (AppleScript not permitted); the DMG still works."
fi

# Volume icon last (srcfolder drops dotfiles; Finder scripting can drop it too).
cp assets/icon/Alpha.icns "/Volumes/$VOL/.VolumeIcon.icns"
SetFile -a C "/Volumes/$VOL"

sync
hdiutil detach -quiet "/Volumes/$VOL"

rm -f "$DMG"
hdiutil convert -quiet "$RW" -format UDZO -imagekey zlib-level=9 -o "$DMG"
hdiutil verify -quiet "$DMG"
echo "Built $DMG ($(du -h "$DMG" | cut -f1))"
