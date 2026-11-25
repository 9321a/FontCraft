#!/bin/bash
# FontCraft for the Mac: builds it for Apple Silicon and for Intel, joins both into one universal
# app (each Mac runs the half made for its processor by itself) and puts it in a disk image with a
# shortcut to Applications: target/dist/FontCraft-<version>-macOS.dmg (and the app itself in
# target/dist/macos/FontCraft.app, to open it without installing).
#
#   packaging/macos/build-dmg.sh             builds both versions first
#   packaging/macos/build-dmg.sh --no-build  uses the ones already built
#
# Signing: with APPLE_SIGNING_IDENTITY (a "Developer ID Application: …" certificate in the
# keychain) the app is signed with it; otherwise it gets an ad-hoc signature, which every Mac needs
# to open it at all, and macOS asks once before opening it (README.md, «Instaladores»). The ad-hoc
# signature names the app by its identifier, not by its code's hash, so macOS keeps the
# Accessibility and Automation permissions FontCraft was given when a new version replaces it.
set -euo pipefail
APP="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$APP"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
ARM=target/aarch64-apple-darwin/release/FontCraft
INTEL=target/x86_64-apple-darwin/release/FontCraft
# macOS 12 Monterey (2021) or newer, Intel and Apple Silicon (Info.plist: LSMinimumSystemVersion).
export MACOSX_DEPLOYMENT_TARGET=12.0

if [ "${1:-}" != "--no-build" ]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$t" >/dev/null 2>&1 || true
    cargo build --release --target "$t"
  done
fi
for b in "$ARM" "$INTEL"; do
  [ -x "$b" ] || { echo "Falta $b: compílalo primero (sin --no-build)." >&2; exit 1; }
done

WORK="$APP/target/dist/macos"
rm -rf "$WORK"
mkdir -p "$WORK"
# The app is put together and signed in a temporary folder: in a folder iCloud keeps (such as
# Documents) macOS adds Finder information to it while it is signed, which codesign refuses.
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/fontcraft-dmg.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
BUNDLE="$STAGE/FontCraft.app"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"

# One program with both processors inside (its name is the Info.plist's CFBundleExecutable).
EXE="$(/usr/libexec/PlistBuddy -c "Print CFBundleExecutable" packaging/macos/Info.plist)"
lipo -create -output "$BUNDLE/Contents/MacOS/$EXE" "$ARM" "$INTEL"
echo "Programa universal: $(lipo -archs "$BUNDLE/Contents/MacOS/$EXE")"

# Info.plist (FontCraft owns .fcl libraries, shown as single documents with their own icon), with
# the version of Cargo.toml (macOS and FontCraft's About show it).
cp packaging/macos/Info.plist "$BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$BUNDLE/Contents/Info.plist"
plutil -lint "$BUNDLE/Contents/Info.plist" >/dev/null
cp assets/icons/fontcraft.icns "$BUNDLE/Contents/Resources/AppIcon.icns"
cp assets/icons/fontcraft.icns "$BUNDLE/Contents/Resources/fontcraft.icns"
cp assets/icons/fontcraft-library.icns "$BUNDLE/Contents/Resources/fontcraft-library.icns"
# One .lproj folder for each language FontCraft speaks (CFBundleLocalizations): macOS gives its own
# panels (Print, Open, Save, fonts, colours) in the system's language only when the app says it has
# that language.
for L in $(/usr/libexec/PlistBuddy -c "Print CFBundleLocalizations" "$BUNDLE/Contents/Info.plist" | sed -e '1d' -e '$d' | tr -d ' '); do
  mkdir -p "$BUNDLE/Contents/Resources/$L.lproj"
  printf '"CFBundleDisplayName" = "FontCraft";\n"CFBundleName" = "FontCraft";\n' > "$BUNDLE/Contents/Resources/$L.lproj/InfoPlist.strings"
done
xattr -cr "$BUNDLE"

ID="$(/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" "$BUNDLE/Contents/Info.plist")"
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$BUNDLE"
  echo "Firmada con: $APPLE_SIGNING_IDENTITY"
else
  codesign --force --sign - -i "$ID" -r="designated => identifier \"$ID\"" "$BUNDLE"
  echo "Firma local (ad hoc): macOS preguntará una vez antes de abrirla."
fi
codesign --verify --strict "$BUNDLE"

# The disk image: the app and a shortcut to Applications, to drag it there.
DMG_STAGE="$STAGE/dmg"
mkdir -p "$DMG_STAGE"
ditto "$BUNDLE" "$DMG_STAGE/FontCraft.app"
ln -s /Applications "$DMG_STAGE/Aplicaciones"
DMG="$APP/target/dist/FontCraft-$VERSION-macOS.dmg"
rm -f "$DMG"
made=0
# hdiutil sometimes answers «Resource busy» for a moment (it happens on GitHub's Macs): retry.
for attempt in 1 2 3; do
  if hdiutil create -volname "FontCraft $VERSION" -srcfolder "$DMG_STAGE" -fs HFS+ -format UDZO -ov "$DMG" >/dev/null 2>"$WORK/hdiutil.log"; then
    made=1
    break
  fi
  sleep 3
done
if [ "$made" = 0 ]; then
  # Where disk images can't be attached (some sandboxes answer «Device not configured»): the file
  # system is written straight into an image, which is then compressed.
  echo "hdiutil create no pudo ($(tail -n 1 "$WORK/hdiutil.log")); se crea con makehybrid."
  hdiutil makehybrid -hfs -hfs-volume-name "FontCraft $VERSION" -default-volume-name "FontCraft $VERSION" -o "$STAGE/raw.dmg" "$DMG_STAGE" >/dev/null
  hdiutil convert "$STAGE/raw.dmg" -format UDZO -o "$DMG" >/dev/null
fi
# The app beside the image too (check-dmg.sh falls back to it where images can't be opened).
ditto "$DMG_STAGE" "$WORK/dmg"
ditto "$BUNDLE" "$WORK/FontCraft.app"
echo "Listo: $DMG ($(du -h "$DMG" | cut -f1))"
