#!/usr/bin/env bash
# Copy this script to dom0 and run it.
# It builds lazyqubes in your dev VM and installs it in dom0.
#
# Usage:
#   bash install-dom0.sh [dev-vm-name]
#
# Default dev VM: personal
set -euo pipefail

DEV_VM="${1:-personal}"
INSTALL="$HOME/bin/lazyqubes"

mkdir -p "$HOME/bin"

echo "[*] Building in $DEV_VM..."
qvm-run --pass-io "$DEV_VM" \
  'cd ~/src/qwe3 && git pull -q && cargo build --release -q && cat target/release/lazyqubes' \
  > "$INSTALL"

chmod +x "$INSTALL"
echo "[+] Installed to $INSTALL"
