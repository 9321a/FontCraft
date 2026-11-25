#!/bin/bash
# FontCraft for Linux (Debian, Ubuntu, Linux Mint, Pop!_OS, Raspberry Pi OS 64-bit…): one .deb for
# 64-bit PCs (amd64) and ARM computers (arm64). It carries both programs; when it's installed it
# sees which one the computer needs, keeps that one, removes the other and adds FontCraft to the
# applications menu, with its icon and the .fcl library file type: target/dist/fontcraft_<version>_all.deb.
#
#   packaging/linux/build-deb.sh             builds both programs first
#   packaging/linux/build-deb.sh --no-build  uses the ones already built
#
# Runs on Debian or Ubuntu with Rust, dpkg-deb and FontCraft's libraries for this computer
# (sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev). FontCraft links GTK (its
# menu bar icon and print dialog), so the half for the other processor is built with `cross`, which
# brings that processor's compiler and libraries in a container (Docker or Podman; Cross.toml says
# which libraries); with FONTCRAFT_NO_CROSS=1 it is built with plain cargo instead, for a computer
# that already has them.
set -euo pipefail
APP="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$APP"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
X64=target/x86_64-unknown-linux-gnu/release/FontCraft
ARM=target/aarch64-unknown-linux-gnu/release/FontCraft

if [ "${1:-}" != "--no-build" ]; then
  case "$(uname -m)" in aarch64 | arm64) HERE=aarch64-unknown-linux-gnu OTHER=x86_64-unknown-linux-gnu ;; *) HERE=x86_64-unknown-linux-gnu OTHER=aarch64-unknown-linux-gnu ;; esac
  rustup target add "$HERE" >/dev/null 2>&1 || true
  cargo build --release --target "$HERE"
  if [ -n "${FONTCRAFT_NO_CROSS:-}" ]; then
    rustup target add "$OTHER" >/dev/null 2>&1 || true
    cargo build --release --target "$OTHER"
  else
    command -v cross >/dev/null 2>&1 || cargo install cross --git https://github.com/cross-rs/cross --locked
    cross build --release --target "$OTHER"
  fi
fi
for b in "$X64" "$ARM"; do
  [ -x "$b" ] || { echo "Falta $b: compílalo primero (sin --no-build)." >&2; exit 1; }
done

# The newest system library (glibc) either program asks for: the package asks for at least that.
glibc_of() { readelf -V "$1" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -n 1; }
GLIBC="$(printf '%s\n%s\n' "$(glibc_of "$X64")" "$(glibc_of "$ARM")" | sort -V | tail -n 1)"

WORK="$APP/target/dist/linux"
ROOT="$WORK/fontcraft"
rm -rf "$WORK"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/lib/fontcraft/amd64" "$ROOT/usr/lib/fontcraft/arm64" \
  "$ROOT/usr/share/applications" "$ROOT/usr/share/mime/packages" "$ROOT/usr/share/doc/fontcraft"
install -m 755 "$X64" "$ROOT/usr/lib/fontcraft/amd64/FontCraft"
install -m 755 "$ARM" "$ROOT/usr/lib/fontcraft/arm64/FontCraft"
install -m 644 packaging/linux/fontcraft.desktop "$ROOT/usr/share/applications/fontcraft.desktop"
install -m 644 packaging/linux/fontcraft-library.xml "$ROOT/usr/share/mime/packages/fontcraft-library.xml"
# The icons at every size of the icon theme (app and .fcl libraries), as FontCraft draws them.
for n in 16 32 48 64 128 256 512; do
  mkdir -p "$ROOT/usr/share/icons/hicolor/${n}x${n}/apps" "$ROOT/usr/share/icons/hicolor/${n}x${n}/mimetypes"
  install -m 644 "assets/icons/png/fontcraft-$n.png" "$ROOT/usr/share/icons/hicolor/${n}x${n}/apps/fontcraft.png"
  install -m 644 "assets/icons/png/fontcraft-library-$n.png" "$ROOT/usr/share/icons/hicolor/${n}x${n}/mimetypes/application-x-fontcraft-library.png"
done
cat > "$ROOT/usr/share/doc/fontcraft/copyright" <<'COPYRIGHT'
FontCraft
COPYRIGHT

SIZE_KB="$(du -sk "$ROOT/usr" | cut -f1)"
cat > "$ROOT/DEBIAN/control" <<CONTROL
Package: fontcraft
Version: $VERSION
Section: graphics
Priority: optional
Architecture: all
Installed-Size: $((SIZE_KB / 2))
Depends: libc6 (>= $GLIBC), libgcc-s1, libgtk-3-0, libxdo3, libayatana-appindicator3-1
Recommends: xdotool, libvulkan1, mesa-vulkan-drivers, libegl1, libxkbcommon-x11-0, xdg-desktop-portal
Maintainer: FontCraft <packaging@fontcraft.invalid>
Description: Explora, previsualiza, imprime y activa fuentes
 FontCraft reúne las fuentes del equipo, bibliotecas (.fcl), carpetas, listas
 inteligentes y Google Fonts, y activa fuentes para las apps de diseño. Este
 paquete sirve para equipos amd64 y arm64: al instalarse conserva solo el
 programa del procesador del equipo.
CONTROL

# Before unpacking: only amd64 and arm64 computers.
cat > "$ROOT/DEBIAN/preinst" <<'PREINST'
#!/bin/sh
set -e
case "$(dpkg --print-architecture)" in
  amd64 | arm64) ;;
  *)
    echo "FontCraft funciona en equipos amd64 (Intel y AMD de 64 bits) y arm64; este es $(dpkg --print-architecture)." >&2
    exit 1
    ;;
esac
exit 0
PREINST

# After unpacking: keep this computer's program, link it as «fontcraft», refresh the menu, the
# icons and the file types.
cat > "$ROOT/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
  ARCH="$(dpkg --print-architecture)"
  ln -sf "/usr/lib/fontcraft/$ARCH/FontCraft" /usr/bin/fontcraft
  for other in amd64 arm64; do
    [ "$other" = "$ARCH" ] || rm -f "/usr/lib/fontcraft/$other/FontCraft"
  done
  if command -v update-mime-database >/dev/null 2>&1; then update-mime-database /usr/share/mime || true; fi
  if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database -q /usr/share/applications || true; fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true; fi
fi
exit 0
POSTINST

# After removing: the link, and the folders dpkg left because of the program removed above.
cat > "$ROOT/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e
case "$1" in
  remove | purge)
    rm -f /usr/bin/fontcraft
    rmdir /usr/lib/fontcraft/amd64 /usr/lib/fontcraft/arm64 /usr/lib/fontcraft 2>/dev/null || true
    if command -v update-mime-database >/dev/null 2>&1; then update-mime-database /usr/share/mime || true; fi
    if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database -q /usr/share/applications || true; fi
    ;;
esac
exit 0
POSTRM
chmod 755 "$ROOT/DEBIAN/preinst" "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"

DEB="$APP/target/dist/fontcraft_${VERSION}_all.deb"
rm -f "$DEB"
dpkg-deb --build --root-owner-group "$ROOT" "$DEB" >/dev/null
echo "Listo: $DEB ($(du -h "$DEB" | cut -f1)); pide glibc $GLIBC o posterior."
