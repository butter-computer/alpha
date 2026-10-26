#!/bin/sh
# Build target/release/bundle/Alpha.app from a release build.
set -eu

cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
APP=target/release/bundle/Alpha.app

cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/alpha "$APP/Contents/MacOS/alpha"
cp assets/icon/Alpha.icns "$APP/Contents/Resources/Alpha.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>Alpha</string>
	<key>CFBundleDisplayName</key><string>Alpha</string>
	<key>CFBundleIdentifier</key><string>computer.butter.alpha</string>
	<key>CFBundleExecutable</key><string>alpha</string>
	<key>CFBundleIconFile</key><string>Alpha</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleVersion</key><string>$VERSION</string>
	<key>CFBundleShortVersionString</key><string>$VERSION</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSRequiresAquaSystemAppearance</key><false/>
	<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>CFBundleDocumentTypes</key>
	<array>
		<dict>
			<key>CFBundleTypeName</key><string>Markdown Document</string>
			<key>CFBundleTypeRole</key><string>Editor</string>
			<key>LSHandlerRank</key><string>Alternate</string>
			<key>CFBundleTypeExtensions</key>
			<array><string>md</string><string>markdown</string></array>
		</dict>
		<dict>
			<key>CFBundleTypeName</key><string>Plain Text Document</string>
			<key>CFBundleTypeRole</key><string>Editor</string>
			<key>LSHandlerRank</key><string>Alternate</string>
			<key>CFBundleTypeExtensions</key>
			<array><string>txt</string></array>
		</dict>
	</array>
</dict>
</plist>
PLIST

plutil -lint "$APP/Contents/Info.plist"
codesign --force --deep -s - "$APP"
echo "Built $APP"
