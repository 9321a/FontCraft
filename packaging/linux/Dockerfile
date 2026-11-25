# The other processor's half of FontCraft for Linux (packaging/linux/build-deb.sh): Ubuntu 22.04
# with that processor's C compiler and the libraries FontCraft links for it (GTK for the menu bar
# icon and the print dialog, xdo for its menus), installed beside the computer's own packages
# (Debian multiarch): amd64 packages come from archive.ubuntu.com, arm64 ones from
# ports.ubuntu.com. Rust itself is the computer's, lent to the container when it runs.
#
# Ubuntu 22.04 on purpose: there the development packages of the other processor install beside the
# computer's own. On 24.04 (the image `cross` uses) libgtk-3-dev:arm64 brings ARM64's Python too,
# whose setup can't run on a PC («python3.12-minimal:arm64 … Exec format error», the first
# GitHub build of these installers).
FROM ubuntu:22.04
# arm64 on a PC (amd64), amd64 on an ARM computer; GCC is that processor's C compiler package.
ARG FOREIGN=arm64
ARG GCC=gcc-aarch64-linux-gnu
ENV DEBIAN_FRONTEND=noninteractive
RUN printf '%s\n' \
      "deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy main universe" \
      "deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy-updates main universe" \
      "deb [arch=amd64] http://security.ubuntu.com/ubuntu jammy-security main universe" \
      "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports jammy main universe" \
      "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports jammy-updates main universe" \
      "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports jammy-security main universe" \
      > /etc/apt/sources.list \
 && dpkg --add-architecture "$FOREIGN" \
 && apt-get update \
 && apt-get upgrade --yes \
 && apt-get install --yes --no-install-recommends ca-certificates gcc libc6-dev pkg-config "$GCC" "libc6-dev-$FOREIGN-cross" \
 && apt-get install --yes --no-install-recommends "libgtk-3-dev:$FOREIGN" "libxdo-dev:$FOREIGN" \
 && rm -rf /var/lib/apt/lists/*
