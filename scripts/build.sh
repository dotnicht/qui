#!/usr/bin/env bash
# Runs inside the dev VM.
# Pulls latest, builds release binary, prints it to stdout.
# Dom0 captures stdout and writes the binary.
set -euo pipefail

REPO="${1:-$HOME/src/qwe3}"

cd "$REPO"
git pull --ff-only
cargo build --release --quiet
cat target/release/lazyqubes
