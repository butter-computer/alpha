#!/bin/bash
# Builds target/release/bundle/Alpha.app (release build + Info.plist + icon).
set -euo pipefail
cd "$(dirname "$0")/.."

APP="target/release/bundle/Alpha.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/alpha "$APP/Contents/MacOS/alpha"
cp assets/icon/alpha.icns "$APP/Contents/Resources/alpha.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Alpha</string>
    <key>CFBundleDisplayName</key><string>Alpha</string>
    <key>CFBundleIdentifier</key><string>computer.butter.alpha</string>
    <key>CFBundleExecutable</key><string>alpha</string>
    <key>CFBundleIconFile</key><string>alpha</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key><string>12.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSRequiresAquaSystemAppearance</key><false/>
</dict>
</plist>
PLIST

plutil -lint "$APP/Contents/Info.plist"
# Ad-hoc sign so macOS treats it as a coherent bundle.
codesign --force --sign - "$APP" >/dev/null 2>&1 || true
echo "Built $APP"
