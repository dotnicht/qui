#!/usr/bin/env bash
# Set alacritty as the default terminal in the current VM.
# When run from dom0, also propagates to all running qubes.
#
# Usage:
#   bash set-alacritty-terminal.sh [vm1 vm2 ...]   # dom0 only: target specific VMs
set -euo pipefail

APPS_DIR="$HOME/.local/share/applications"
DESKTOP_FILE="qubes-run-terminal.desktop"
MIMEAPPS="$HOME/.config/mimeapps.list"

# ── configure local VM (or dom0) ─────────────────────────────────────────────
configure_local() {
    mkdir -p "$APPS_DIR" "$(dirname "$MIMEAPPS")"

    cat > "$APPS_DIR/$DESKTOP_FILE" << 'EOF'
[Desktop Entry]
Name=Alacritty
Exec=alacritty
Type=Application
NoDisplay=true
EOF

    # mimeapps.list is more reliable than xdg-mime in minimal environments
    if grep -q '^\[Default Applications\]' "$MIMEAPPS" 2>/dev/null; then
        # remove any existing entry for this mime type then re-add
        sed -i '/^x-scheme-handler\/qubes-run-terminal=/d' "$MIMEAPPS"
        sed -i '/^\[Default Applications\]/a x-scheme-handler\/qubes-run-terminal=qubes-run-terminal.desktop' "$MIMEAPPS"
    else
        printf '[Default Applications]\nx-scheme-handler/qubes-run-terminal=qubes-run-terminal.desktop\n' >> "$MIMEAPPS"
    fi

    update-desktop-database "$APPS_DIR" 2>/dev/null || true
}

echo "[*] Configuring local VM"
configure_local
echo "[+] Local done"

# ── dispatch to other qubes if running in dom0 ───────────────────────────────
if ! command -v qvm-run &>/dev/null; then
    echo "[*] qvm-run not found — skipping other VMs (run from dom0 to propagate)"
    exit 0
fi

if [[ $# -gt 0 ]]; then
    mapfile -t VMS < <(printf '%s\n' "$@")
else
    mapfile -t VMS < <(qvm-ls --raw-data --fields name,state 2>/dev/null \
        | awk -F'|' '$2=="Running" && $1!="dom0" {print $1}')
fi

SCRIPT=$(readlink -f "$0")

for vm in "${VMS[@]}"; do
    [[ -z "$vm" ]] && continue
    echo "[*] Configuring $vm"
    qvm-copy-to-vm "$vm" "$SCRIPT" 2>/dev/null \
        && qvm-run --pass-io --user=user "$vm" \
            "bash \"\$HOME/QubesIncoming/dom0/$(basename "$SCRIPT")\"" \
        && echo "[+] $vm done" \
        || echo "[!] $vm failed (skipping)"
done

echo "[*] Done."
