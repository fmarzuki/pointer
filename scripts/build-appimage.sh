#!/usr/bin/env bash
# Bangun AppImage dari target/release/pointer.
#
# Pemakaian: APPIMAGETOOL=/path/appimagetool-x86_64.AppImage scripts/build-appimage.sh
# Hasil: dist/Pointer-<versi>-x86_64.AppImage
#
# libX11/xcb/xkbcommon tidak dibundel: dimuat dari sistem lewat dlopen (ada di semua
# desktop X11/XWayland). Menjalankan AppImage butuh FUSE 2 (`libfuse2t64` di Ubuntu 24.04+).
set -euo pipefail

cd "$(dirname "$0")/.."
: "${APPIMAGETOOL:?set APPIMAGETOOL ke path appimagetool-x86_64.AppImage}"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
id=io.github.fmarzuki.Pointer
appdir=dist/Pointer.AppDir

[ -x target/release/pointer ] || cargo build --release --locked

rm -rf "$appdir"
install -Dm755 target/release/pointer "$appdir/usr/bin/pointer"
install -Dm644 "packaging/linux/$id.desktop" "$appdir/usr/share/applications/$id.desktop"
install -Dm644 "packaging/linux/$id.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/$id.svg"
# appimagetool membutuhkan .desktop dan ikon di root AppDir.
cp "packaging/linux/$id.desktop" "packaging/linux/$id.svg" "$appdir/"

cat > "$appdir/AppRun" <<'APPRUN'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/bin/pointer" "$@"
APPRUN
chmod 755 "$appdir/AppRun"

# --appimage-extract-and-run: appimagetool sendiri AppImage; jalan tanpa FUSE (mis. di CI).
ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run "$appdir" "dist/Pointer-$version-x86_64.AppImage"
echo "dist/Pointer-$version-x86_64.AppImage"
