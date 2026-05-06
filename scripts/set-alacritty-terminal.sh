#!/usr/bin/env bash
# Set alacritty as the default terminal across dom0 and all qubes.
#
# Run from dom0. Targets every running TemplateVM and AppVM by default,
# or a specific list passed as arguments.
#
# Usage:
#   bash set-alacritty-terminal.sh [vm1 vm2 ...]
#
# What it does:
#   dom0  — writes a qubes-run-terminal.desktop override and sets it via xdg-mime
#   qubes — drops the same desktop file into each VM via qvm-run
set -euo pipefail

DESKTOP_CONTENT='[Desktop Entry]
Name=Alacritty
Exec=alacritty
Type=Application
NoDisplay=true
'

DOM0_APPS_DIR="$HOME/.local/share/applications"
DESKTOP_FILE="qubes-run-terminal.desktop"
MIME_TYPE="x-scheme-handler/qubes-run-terminal"

# ── dom0 ──────────────────────────────────────────────────────────────────────
echo "[*] Configuring dom0"
mkdir -p "$DOM0_APPS_DIR"
printf '%s' "$DESKTOP_CONTENT" > "$DOM0_APPS_DIR/$DESKTOP_FILE"
xdg-mime default "$DESKTOP_FILE" "$MIME_TYPE" 2>/dev/null || true
update-desktop-database "$DOM0_APPS_DIR" 2>/dev/null || true
echo "[+] dom0 done"

# ── qubes ─────────────────────────────────────────────────────────────────────
if [[ $# -gt 0 ]]; then
    VMS=("$@")
else
    mapfile -t VMS < <(qvm-ls --running --raw-data --fields name 2>/dev/null | grep -v '^dom0$')
fi

for vm in "${VMS[@]}"; do
    [[ -z "$vm" ]] && continue
    echo "[*] Configuring $vm"
    qvm-run --pass-io --user=user "$vm" "
        set -euo pipefail
        mkdir -p \"\$HOME/.local/share/applications\"
        cat > \"\$HOME/.local/share/applications/$DESKTOP_FILE\" << 'DESKTOP'
$DESKTOP_CONTENT
DESKTOP
        xdg-mime default '$DESKTOP_FILE' '$MIME_TYPE' 2>/dev/null || true
        update-desktop-database \"\$HOME/.local/share/applications\" 2>/dev/null || true
    " && echo "[+] $vm done" || echo "[!] $vm failed (skipping)"
done

echo "[*] All done. Qubes that were not running must be configured on next boot or run manually."
