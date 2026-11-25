#!/bin/sh
# FontCraft for Android (from Linux or a Mac): one .apk for ARM64 phones and tablets and for x86_64
# devices and emulators (Android 12 or later); Android installs the part for its processor:
# target/dist/FontCraft-<version>-Android.apk.
#
#   packaging/android/build-apk.sh
#
# Needs (once): Rust (https://rustup.rs); the Android SDK and NDK, for example from Android Studio
# (SDK Manager › SDK Tools › NDK) or with
#   sdkmanager "platforms;android-35" "build-tools;35.0.0" "ndk;26.3.11579264"
# Java (JDK 17) to sign it; and ANDROID_HOME (the SDK's folder) and, when the NDK isn't inside the
# SDK, ANDROID_NDK_ROOT.
#
# Signing: with CARGO_APK_RELEASE_KEYSTORE and CARGO_APK_RELEASE_KEYSTORE_PASSWORD (a permanent key,
# needed for Google Play) it uses them; otherwise this computer's Android debug key, made the first
# time as Android Studio makes it (for installing and testing).
set -e
APP="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$APP"
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null 2>&1 || { echo "No encuentro Rust (cargo). Instálalo desde https://rustup.rs"; exit 1; }

# The Android SDK and NDK.
ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
[ -d "$ANDROID_HOME" ] || { echo "No encuentro el SDK de Android. Define ANDROID_HOME (ahora: $ANDROID_HOME)."; exit 1; }
# Without ANDROID_NDK_ROOT, or with one that doesn't exist: the NDK inside the SDK (26.3, the one
# FontCraft is built with, when it's there; else the newest).
if [ -z "${ANDROID_NDK_ROOT:-}" ] || [ ! -d "$ANDROID_NDK_ROOT" ]; then
  if [ -d "$ANDROID_HOME/ndk/26.3.11579264" ]; then
    ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/26.3.11579264"
  else
    ANDROID_NDK_ROOT=$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1)
  fi
fi
[ -d "$ANDROID_NDK_ROOT" ] || { echo "No encuentro el NDK de Android. Instálalo con el SDK Manager o define ANDROID_NDK_ROOT."; exit 1; }
export ANDROID_HOME ANDROID_SDK_ROOT="$ANDROID_HOME" ANDROID_NDK_ROOT ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
echo "SDK: $ANDROID_HOME"
echo "NDK: $ANDROID_NDK_ROOT"

# Android's processors for Rust and the tool that makes the .apk.
rustup target add aarch64-linux-android x86_64-linux-android
command -v cargo-apk >/dev/null 2>&1 || cargo install cargo-apk

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ -z "${CARGO_APK_RELEASE_KEYSTORE:-}" ]; then
  KS="$HOME/.android/debug.keystore"
  if [ ! -f "$KS" ]; then
    mkdir -p "$HOME/.android"
    keytool -genkeypair -v -keystore "$KS" -storepass android -keypass android -alias androiddebugkey \
      -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null
  fi
  export CARGO_APK_RELEASE_KEYSTORE="$KS"
  export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="android"
fi
command -v keytool >/dev/null 2>&1 || [ -f "$CARGO_APK_RELEASE_KEYSTORE" ] || { echo "Falta Java (keytool) para firmar el .apk."; exit 1; }
(cd android && cargo apk build --release)
mkdir -p target/dist
cp android/target/release/apk/FontCraft.apk "target/dist/FontCraft-$VERSION-Android.apk"
echo ""
echo "Listo: target/dist/FontCraft-$VERSION-Android.apk"
echo "Para instalarlo en un teléfono conectado por USB (con depuración USB activada):"
echo "  $ANDROID_HOME/platform-tools/adb install -r target/dist/FontCraft-$VERSION-Android.apk"
