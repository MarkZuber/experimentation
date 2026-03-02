#!/usr/bin/env bash
# Install apt packages required to build and run the ExperiProduct backend.
# Idempotent — safe to re-run.
set -euo pipefail

log() { echo "[install-build-deps] $*"; }
ok()  { echo "[install-build-deps] ✓ $*"; }

sudo apt-get update -q

PACKAGES=(
    # Rust openssl-sys crate requires OpenSSL headers
    libssl-dev

    # pkg-config is used by openssl-sys (and others) to locate system libs
    pkg-config

    # Protocol Buffer compiler — required by tonic-build at Rust compile time
    protobuf-compiler

    # PostgreSQL client library headers (needed if any crate links against libpq)
    libpq-dev

    # Common C build toolchain (gcc, make) — needed by some *-sys crates
    build-essential
)

log "Installing: ${PACKAGES[*]}"
sudo apt-get install -y "${PACKAGES[@]}"
ok "All packages installed"
