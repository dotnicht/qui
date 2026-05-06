#!/bin/bash
# Set alacritty as the default terminal in the current VM.
# When run from dom0, also propagates to all running qubes.
#
# Usage (from dom0):
#   bash set-alacritty-terminal.sh [vm1 vm2 ...]
set -eu

APPS_DIR="$HOME/.local/share/applications"
DESKTOP_FILE="qubes-run-terminal.desktop"
MIMEAPPS="$HOME/.config/mimeapps.list"

mkdir -p "$APPS_DIR" "$(dirname "$MIMEAPPS")"

printf '[Desktop Entry]\nName=Alacritty\nExec=alacritty\nType=Application\nNoDisplay=true\n' \
    > "$APPS_DIR/$DESKTOP_FILE"

if grep -qs '\[Default Applications\]' "$MIMEAPPS"; then
    sed -i '/^x-scheme-handler\/qubes-run-terminal=/d' "$MIMEAPPS"
    sed -i '/\[Default Applications\]/a x-scheme-handler\/qubes-run-terminal=qubes-run-terminal.desktop' "$MIMEAPPS"
else
    printf '[Default Applications]\nx-scheme-handler/qubes-run-terminal=qubes-run-terminal.desktop\n' >> "$MIMEAPPS"
fi

update-desktop-database "$APPS_DIR" 2>/dev/null || true
echo "[+] done"

if ! command -v qvm-run >/dev/null 2>&1; then
    exit 0
fi

SCRIPT=$(readlink -f "$0")

if [ $# -gt 0 ]; then
    VMS="$*"
else
    VMS=$(qvm-ls --raw-data --fields name,state 2>/dev/null \
        | awk -F'|' '$2=="Running" && $1!="dom0" {print $1}')
fi

for vm in $VMS; do
    echo "[*] Configuring $vm"
    qvm-copy-to-vm "$vm" "$SCRIPT" 2>/dev/null \
        && qvm-run --pass-io --user=user "$vm" \
            "bash \"\$HOME/QubesIncoming/dom0/$(basename "$SCRIPT")\"" \
        && echo "[+] $vm done" \
        || echo "[!] $vm failed (skipping)"
done
