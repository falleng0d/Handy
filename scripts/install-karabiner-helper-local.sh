#!/usr/bin/env bash
set -euo pipefail

app_path="${1:-/Applications/Handy.app}"
helper="${2:-$app_path/Contents/MacOS/karabiner-input-helper}"
plist="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/launch-daemons/com.falleng0d.karabiner-input.helper.local.plist"
installed_helper="/Library/PrivilegedHelperTools/com.falleng0d.karabiner-input.helper"
installed_plist="/Library/LaunchDaemons/com.falleng0d.karabiner-input.helper.local.plist"
label="com.falleng0d.karabiner-input.helper.local"
certificate="${KARABINER_INPUT_LOCAL_CERT_SHA1:-E3262DC4B5253DFA15864022B77D68B35CBA8302}"

test -x "$helper" || { echo "Missing helper at $helper" >&2; exit 1; }
codesign -R="certificate leaf = H\"$certificate\"" --verify "$helper"

sudo launchctl bootout "system/$label" 2>/dev/null || true
sudo install -d -o root -g wheel -m 755 /Library/PrivilegedHelperTools
sudo install -o root -g wheel -m 755 "$helper" "$installed_helper"
sudo install -o root -g wheel -m 644 "$plist" "$installed_plist"
sudo touch /var/log/karabiner-input-helper.log
sudo chown root:wheel /var/log/karabiner-input-helper.log
sudo chmod 644 /var/log/karabiner-input-helper.log
sudo launchctl bootstrap system "$installed_plist"
sudo launchctl enable "system/$label"
sudo launchctl kickstart -k "system/$label"

echo "Karabiner input helper installed for local development."
