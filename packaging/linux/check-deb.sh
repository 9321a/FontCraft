#!/bin/bash
# Installs FontCraft's .deb (as root), checks that it kept the program made for this computer's
# processor, linked it as «fontcraft», added it to the applications menu with its icon and the .fcl
# file type, and that it starts; then removes it and checks nothing is left.
#
#   sudo packaging/linux/check-deb.sh [target/dist/fontcraft_<version>_all.deb]
set -euo pipefail
APP="$(cd "$(dirname "$0")/../.." && pwd)"
DEB="${1:-$(ls -t "$APP"/target/dist/fontcraft_*_all.deb | head -n 1)}"
# apt installs a file only when its path has a folder in it.
case "$DEB" in /*) ;; *) DEB="$PWD/$DEB" ;; esac
ARCH="$(dpkg --print-architecture)"
case "$ARCH" in amd64) OTHER=arm64 WANT=62 ;; arm64) OTHER=amd64 WANT=183 ;; *) echo "Equipo $ARCH: FontCraft no se instala aquí." >&2; exit 1 ;; esac
export DEBIAN_FRONTEND=noninteractive
apt-get install -y --no-install-recommends "$DEB" >/dev/null

ok=1
check() { if eval "$2"; then echo "✓ $1"; else echo "✗ $1"; ok=0; fi; }
KEPT="/usr/lib/fontcraft/$ARCH/FontCraft"
# The processor an ELF program is for: e_machine, 62 for x86-64 and 183 for ARM64.
MACHINE="$(od -An -tu2 -j18 -N2 "$KEPT" 2>/dev/null | tr -d ' ')"
check "equipo $ARCH: se conserva su programa ($KEPT)" "[ -x '$KEPT' ] && [ '$MACHINE' = '$WANT' ]"
check "el programa de $OTHER se quitó" "[ ! -e /usr/lib/fontcraft/$OTHER/FontCraft ]"
check "«fontcraft» abre ese programa" "[ \"\$(readlink -f /usr/bin/fontcraft)\" = '$KEPT' ]"
check "está en el menú de aplicaciones, con su icono" "[ -f /usr/share/applications/fontcraft.desktop ] && [ -f /usr/share/icons/hicolor/256x256/apps/fontcraft.png ]"
check "las bibliotecas .fcl tienen su tipo de archivo" "[ -f /usr/share/mime/packages/fontcraft-library.xml ]"
VERSION_TEXT="$(fontcraft --version 2>/dev/null || true)"
check "arranca: ${VERSION_TEXT:-no}" "[ -n \"\$VERSION_TEXT\" ]"

apt-get remove -y fontcraft >/dev/null
check "al desinstalarlo no queda nada" "[ ! -e /usr/bin/fontcraft ] && [ ! -e /usr/lib/fontcraft ]"
if [ "$ok" = 1 ]; then echo "Todo bien."; else echo "Hay problemas."; exit 1; fi
