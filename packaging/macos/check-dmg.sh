#!/bin/bash
# Opens FontCraft's disk image read-only (without showing it in the Finder) and checks what a person
# would get: the universal app (Apple Silicon and Intel), its signature, version and icon, and the
# shortcut to Applications. It always closes the image again.
#
#   packaging/macos/check-dmg.sh [target/dist/FontCraft-<version>-macOS.dmg]
#
# Where disk images can't be opened (some sandboxes answer «Device not configured») it checks the
# folder that build-dmg.sh put into the image instead, and says so; with CHECK_DMG_STRICT=1 (the
# GitHub build) the image itself must open.
set -euo pipefail
APP="$(cd "$(dirname "$0")/../.." && pwd)"
DMG="${1:-$(ls -t "$APP"/target/dist/FontCraft-*-macOS.dmg | head -n 1)}"
MNT="$APP/target/dist/macos/mnt"
mkdir -p "$MNT"
if hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$MNT" "$DMG" >/dev/null 2>"$APP/target/dist/macos/attach.log"; then
  trap 'hdiutil detach "$MNT" >/dev/null 2>&1 || hdiutil detach -force "$MNT" >/dev/null 2>&1' EXIT
  ROOT="$MNT"
  echo "Imagen abierta: $(basename "$DMG")"
elif [ "${CHECK_DMG_STRICT:-0}" = 1 ]; then
  echo "No se pudo abrir la imagen: $(tail -n 1 "$APP/target/dist/macos/attach.log")" >&2
  exit 1
else
  ROOT="$APP/target/dist/macos/dmg"
  echo "No se pudo abrir la imagen aquí ($(tail -n 1 "$APP/target/dist/macos/attach.log")): se comprueba la carpeta que se metió en ella."
fi
ok=1
check() { if eval "$2"; then echo "✓ $1"; else echo "✗ $1"; ok=0; fi; }
BUNDLE="$ROOT/FontCraft.app"
EXE="$(/usr/libexec/PlistBuddy -c "Print CFBundleExecutable" "$BUNDLE/Contents/Info.plist" 2>/dev/null || echo fontcraft)"
BIN="$BUNDLE/Contents/MacOS/$EXE"
check "la app está en la imagen" "[ -d '$BUNDLE' ]"
check "el programa se puede ejecutar" "[ -x '$BIN' ]"
check "lleva Apple Silicon e Intel ($(lipo -archs "$BIN" 2>/dev/null))" "lipo -archs '$BIN' | grep -q arm64 && lipo -archs '$BIN' | grep -q x86_64"
check "la firma es válida" "codesign --verify --strict '$BUNDLE' 2>/dev/null"
check "$("$BIN" --version 2>/dev/null) arranca en este Mac ($(uname -m))" "'$BIN' --version 2>/dev/null | grep -q '^FontCraft '"
check "los iconos están (app y bibliotecas .fcl)" "[ -s '$BUNDLE/Contents/Resources/AppIcon.icns' ] && [ -s '$BUNDLE/Contents/Resources/fontcraft-library.icns' ]"
check "el acceso a Aplicaciones apunta a /Applications" "[ \"\$(readlink '$ROOT/Aplicaciones')\" = /Applications ]"
if [ "$ok" = 1 ]; then echo "Todo bien."; else echo "Hay problemas."; exit 1; fi
