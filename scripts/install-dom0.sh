#!/usr/bin/env bash
# Install qui in dom0 by building it inside a dev VM.
#
# Copy this script to dom0 and run it — no local paths required.
# Sources are cloned fresh from the repository.
#
# Usage:
#   bash install-dom0.sh [dev-vm] [repo-url] [install-path]
#
# Defaults:
#   dev-vm        personal
#   repo-url      https://github.com/dotnicht/qwe3
#   install-path  ~/bin/qui
set -euo pipefail

DEV_VM="${1:-personal}"
REPO_URL="${2:-https://github.com/dotnicht/qwe3}"
INSTALL="${3:-$HOME/bin/qui}"

echo "[*] Cloning and building in VM: $DEV_VM"
mkdir -p "$(dirname "$INSTALL")"

qvm-run --pass-io "$DEV_VM" "
  set -euo pipefail
  TMPDIR=\$(mktemp -d)
  trap 'rm -rf \"\$TMPDIR\"' EXIT
  git clone --depth 1 '$REPO_URL' \"\$TMPDIR/qui\"
  cd \"\$TMPDIR/qui\"
  cargo build --release --quiet
  cat target/release/qui
" > "$INSTALL"

chmod +x "$INSTALL"
echo "[+] Installed: $INSTALL"
