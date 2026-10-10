#!/usr/bin/env bash
# Bangun pointer_<versi>-1_amd64.deb dan pointer-<versi>-windows-x64.exe di dalam Docker.
#
# Pemakaian: scripts/build-packages.sh
# Hasil:     dist/
#
# Build memakai direktori target terpisah (target/docker) agar tidak bercampur dengan
# build host yang memakai glibc berbeda. Cache registry crates disimpan di volume Docker
# `pointer-cargo-registry`.
set -euo pipefail

cd "$(dirname "$0")/.."
image=pointer-build:bookworm
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)

docker build -t "$image" packaging/docker

docker run --rm \
    -v "$PWD":/src -w /src \
    -v pointer-cargo-registry:/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/src/target/docker \
    -e VERSION="$version" -e OWNER="$(id -u):$(id -g)" \
    "$image" bash -euo pipefail -c '
        cargo deb --locked
        cargo build --release --locked --target x86_64-pc-windows-gnu
        mkdir -p dist
        cp target/docker/debian/pointer_"$VERSION"-1_amd64.deb dist/
        cp target/docker/x86_64-pc-windows-gnu/release/pointer.exe dist/pointer-"$VERSION"-windows-x64.exe
        chown -R "$OWNER" target/docker dist
    '

ls -l dist/
