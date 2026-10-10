# Lingkungan build paket: Debian 12 (glibc 2.36) agar .deb terpasang di Ubuntu 24.04+ /
# Debian 12+, plus mingw-w64 untuk cross-compile pointer.exe (target x86_64-pc-windows-gnu).
FROM rust:1.99.0-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-pc-windows-gnu \
    && cargo install cargo-deb --locked --version 3.8.0

ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
