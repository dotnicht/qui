#!/usr/bin/env bash
# Runs in dom0.
# Builds lazyqubes in the dev VM and installs it in dom0.
#
# Usage:
#   ./deploy-dom0.sh [dev-vm-name] [repo-path-in-vm] [install-path]
#
# Defaults:
#   dev-vm-name        personal
#   repo-path-in-vm    ~/src/qwe3
#   install-path       ~/bin/lazyqubes
set -euo pipefail

DEV_VM="${1:-personal}"
REPO="${2:-\$HOME/src/qwe3}"
INSTALL="${3:-$HOME/bin/lazyqubes}"

echo "[*] Building in VM: $DEV_VM ($REPO)"
mkdir -p "$(dirname "$INSTALL")"

qvm-run --pass-io "$DEV_VM" "bash ~/src/qwe3/scripts/build.sh $REPO" > "$INSTALL"
chmod +x "$INSTALL"

echo "[+] Installed: $INSTALL"
"$INSTALL" --version 2>/dev/null || echo "[+] Binary ready — launch with: $INSTALL"
