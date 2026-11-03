#!/usr/bin/env bash
# Isolated fixtures only. Never change the developer's Termux or keyboard.
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/.." && pwd)
root=$(mktemp -d)
trap 'rm -rf -- "$root"' EXIT
mkdir -p "$root/home/.termux" "$root/prefix" "$root/bin"
printf '#!/usr/bin/env bash\nexit 0\n' > "$root/bin/termux-reload-settings"
# Expansion belongs to the fake program, not this test.
# shellcheck disable=SC2016
printf '#!/usr/bin/env bash\nif [[ ${MOCK_THEME_FAIL:-0} == 1 ]]; then exit 1; fi\nprintf "background=#282828\\nforeground=#d4be98\\n"\n' > "$root/bin/mobux"
chmod +x "$root/bin/"*
run_setup() {
    HOME="$root/home" PREFIX="$root/prefix" TERMUX_VERSION=test PATH="$root/bin:$PATH" bash "$repo/scripts/setup-termux-vim.sh" "$@"
}
properties="$root/home/.termux/termux.properties"
printf '# keep comment\nextra-keys-style = default\nextra-keys = [[\x27OLD\x27], \\\n [\x27ESC\x27]]\nfont-size=18\n' > "$properties"
cp "$properties" "$root/original"
printf 'n\n' | run_setup > /dev/null
cmp "$properties" "$root/original"
printf 'y\n' | run_setup --theme > /dev/null
grep -qx 'font-size=18' "$properties"
grep -qx 'extra-keys-style = default' "$properties"
grep -qx '# keep comment' "$properties"
[[ $(grep -c '^extra-keys =' "$properties") == 1 ]]
if grep -q OLD "$properties" || grep -q "^ \['ESC'\]\]" "$properties"; then
    echo "Old multiline keys were not removed" >&2; exit 1
fi
grep -q "'ESC','CTRL','ALT','TAB','HOME','END'" "$properties"
grep -qx 'background=#282828' "$root/home/.termux/colors.properties"
backups=("$properties".mobux-backup.*)
cmp "$root/original" "${backups[0]}"
cp "$properties" "$root/before-failure"
if printf 'y\n' | MOCK_THEME_FAIL=1 run_setup --theme > /dev/null; then echo 'Expected failed export' >&2; exit 1; fi
cmp "$properties" "$root/before-failure"
# A second run is idempotent and creates another backup.
printf 'y\n' | run_setup > /dev/null
cmp "$properties" "$root/before-failure"
[[ $(grep -c '^extra-keys =' "$properties") == 1 ]]
# Existing fallback config is used instead of creating a competing config.
mv "$root/home/.termux" "$root/previous"
mkdir -p "$root/home/.config/termux"
printf 'font-size=22\n' > "$root/home/.config/termux/termux.properties"
printf 'y\n' | run_setup > /dev/null
[[ ! -e $properties ]]
grep -qx 'font-size=22' "$root/home/.config/termux/termux.properties"
# Symlinks are not replaced or followed.
mkdir -p "$root/home/.termux"
printf 'outside\n' > "$root/outside"
ln -s "$root/outside" "$properties"
if printf 'y\n' | run_setup > /dev/null 2>&1; then echo 'Expected symlink rejection' >&2; exit 1; fi
grep -qx outside "$root/outside"
echo 'PASS: confirmed key setup preserves settings, backs up, handles continuations, aborts failed exports, and rejects symlinks.'
