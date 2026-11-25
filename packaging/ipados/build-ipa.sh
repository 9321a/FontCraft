#!/bin/sh
# FontCraft for iPad and iPhone (ARM64, iPadOS 15 or later), from a Mac: the app package
# target/dist/FontCraft-<version>-iPadOS.ipa (and FontCraft-Fonts.mobileconfig, the font profile).
#
#   packaging/ipados/build-ipa.sh             builds it first
#   packaging/ipados/build-ipa.sh --no-build  uses the program already built
#
# It carries an ad-hoc signature: AltStore, Sideloadly or TrollStore install it on an iPad, and
# Xcode or Apple Configurator (Devices › install app) with an Apple account; the App Store needs
# Apple's developer account.
set -e
APP="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$APP"
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null 2>&1 || { echo "No encuentro Rust (cargo). Instálalo desde https://rustup.rs"; exit 1; }
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
BIN=ipados/target/aarch64-apple-ios/release/FontCraft

if [ "${1:-}" != "--no-build" ]; then
  rustup target add aarch64-apple-ios >/dev/null 2>&1 || true
  (cd ipados && cargo build --release --target aarch64-apple-ios)
fi
[ -f "$BIN" ] || { echo "Falta $BIN: compílalo primero (sin --no-build)."; exit 1; }

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/fontcraft-ipa.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
BUNDLE="$STAGE/Payload/FontCraft.app"
mkdir -p "$BUNDLE"
cp "$BIN" "$BUNDLE/FontCraft"
chmod 755 "$BUNDLE/FontCraft"
cp packaging/ipados/Info.plist "$BUNDLE/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$BUNDLE/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$BUNDLE/Info.plist" 2>/dev/null || true
plutil -lint "$BUNDLE/Info.plist" >/dev/null
printf "APPL????" > "$BUNDLE/PkgInfo"
for L in en es fr de it pt nl sv da nb fi pl cs tr ro hu el ru uk id vi zh-Hans zh-Hant ja ko; do
  mkdir -p "$BUNDLE/$L.lproj"
  printf '"CFBundleDisplayName" = "FontCraft";\n"CFBundleName" = "FontCraft";\n' > "$BUNDLE/$L.lproj/InfoPlist.strings"
done
# The app's icon at the sizes its Info.plist names (iPhone 60 pt, iPad 76 and 83.5 pt, at 2x),
# scaled by macOS from FontCraft's 1024-pixel icon.
for spec in "120 AppIcon60x60@2x.png" "152 AppIcon76x76@2x~ipad.png" "167 AppIcon83.5x83.5@2x~ipad.png"; do
  set -- $spec
  sips -z "$1" "$1" assets/icons/png/fontcraft-1024.png --out "$BUNDLE/$2" >/dev/null
done

# The font profile (Settings › Profile downloaded), as earlier builds made it.
UUID_PAYLOAD="B8D97D2E-73E5-4E89-B0F3-$(date +%s | cut -c1-12)"
UUID_PROFILE="C3A19F84-29E1-4C5D-97D8-$(date +%s | cut -c1-12)"
cat > "$STAGE/FontCraft-Fonts.mobileconfig" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>PayloadDisplayName</key>
	<string>FontCraft Fonts Installer</string>
	<key>PayloadDescription</key>
	<string>Instalador de fuentes para iPadOS de FontCraft para Procreate, Canva, Affinity, Pages y suites de diseño.</string>
	<key>PayloadIdentifier</key>
	<string>app.fontcraft.fonts.profile</string>
	<key>PayloadOrganization</key>
	<string>FontCraft</string>
	<key>PayloadRemovalDisallowed</key>
	<false/>
	<key>PayloadType</key>
	<string>Configuration</string>
	<key>PayloadUUID</key>
	<string>$UUID_PROFILE</string>
	<key>PayloadVersion</key>
	<integer>1</integer>
	<key>PayloadContent</key>
	<array>
		<dict>
			<key>PayloadType</key>
			<string>com.apple.font</string>
			<key>PayloadVersion</key>
			<integer>1</integer>
			<key>PayloadIdentifier</key>
			<string>app.fontcraft.font.payload</string>
			<key>PayloadUUID</key>
			<string>$UUID_PAYLOAD</string>
			<key>PayloadDisplayName</key>
			<string>FontCraft Managed Fonts</string>
		</dict>
	</array>
</dict>
</plist>
EOF
codesign --force -s - "$BUNDLE" >/dev/null 2>&1 || true

mkdir -p target/dist
IPA="$APP/target/dist/FontCraft-$VERSION-iPadOS.ipa"
rm -f "$IPA"
(cd "$STAGE" && zip -qry "$IPA" Payload)
cp "$STAGE/FontCraft-Fonts.mobileconfig" target/dist/FontCraft-Fonts.mobileconfig
echo "Listo: $IPA y target/dist/FontCraft-Fonts.mobileconfig"
