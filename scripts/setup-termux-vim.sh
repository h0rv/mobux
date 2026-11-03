#!/usr/bin/env bash
# Explicit, confirmed Termux key-row setup. HeliBoard settings stay in HeliBoard.
set -euo pipefail
with_theme=false
case "${1:-}" in
    '') ;;
    --theme) with_theme=true ;;
    --help|-h) echo 'Usage: bash setup-termux-vim.sh [--theme]'; exit 0 ;;
    *) echo 'Usage: bash setup-termux-vim.sh [--theme]' >&2; exit 1 ;;
esac
if (( $# > 1 )); then echo 'Too many arguments.' >&2; exit 1; fi
if [[ -z ${TERMUX_VERSION:-} || ${PREFIX:-} != /* ]]; then
    echo 'Run this setup inside Termux.' >&2; exit 1
fi
# Match Termux preference lookup: prefer ~/.termux if its properties exist.
directory="$HOME/.termux"
if [[ ! -e $directory/termux.properties && -f $HOME/.config/termux/termux.properties ]]; then
    directory="$HOME/.config/termux"
fi
properties="$directory/termux.properties"
colors="$directory/colors.properties"
targets=("$properties")
if $with_theme; then targets+=("$colors"); fi
for file in "${targets[@]}"; do
    if [[ -L $file || ( -e $file && ! -f $file ) ]]; then
        echo "Refusing to replace a symlink or non-file: $file" >&2; exit 1
    fi
done
printf 'Set two Termux key rows: Esc/Ctrl/Alt/Tab/Home/End, then : / and arrows.\n'
printf 'Only extra-keys will change in %s. Other settings are preserved.\n' "$properties"
if $with_theme; then
    command -v mobux >/dev/null || { echo 'Install the latest Mobux first.' >&2; exit 1; }
    printf 'Also replace %s with the current Mobux palette.\n' "$colors"
fi
printf 'Existing files will have separate backup copies. HeliBoard will not change.\n'
read -r -p 'Apply these changes? [y/N] ' answer
case "$answer" in y|Y) ;; *) echo 'Cancelled.'; exit 0 ;; esac
mkdir -p "$directory"
work=$(mktemp -d "$directory/.mobux-setup.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
# Generate before writing anything. Invalid Mobux settings must abort the setup.
if $with_theme; then mobux --print-termux-theme > "$work/colors"; fi
if [[ -f $properties ]]; then
    cp -p -- "$properties" "$work/original"
    # Java properties can continue over lines with an odd number of trailing
    # backslashes. Remove the entire old extra-keys property, not its neighbors.
    awk '
      function continues(line, count) {
        sub(/\r$/, "", line)
        count=0
        while (substr(line,length(line),1)=="\\") {
          count++
          line=substr(line,1,length(line)-1)
        }
        return count % 2
      }
      /^[ \t]*extra-keys([ \t]*[=:]|[ \t]+|$)/ {
        while (continues($0) && (getline > 0)) {}
        next
      }
      { print }
    ' "$work/original" > "$work/properties"
else
    : > "$work/properties"
fi
printf '%s\n' "extra-keys = [['ESC','CTRL','ALT','TAB','HOME','END'],[':','/','LEFT','DOWN','UP','RIGHT']]" >> "$work/properties"
backup() {
    if [[ -f $1 ]]; then
        local copy
        copy=$(mktemp "$1.mobux-backup.XXXXXX")
        cp -p -- "$1" "$copy"
        printf 'Backup: %s\n' "$copy"
    fi
}
if { [[ -f $work/original ]] && ! cmp -s "$properties" "$work/original"; } || { [[ ! -f $work/original && -e $properties ]]; }; then
    echo 'Termux properties changed during setup. Nothing was replaced.' >&2; exit 1
fi
backup "$properties"
if $with_theme; then backup "$colors"; fi
chmod 600 "$work/properties"
mv -- "$work/properties" "$properties"
if $with_theme; then chmod 600 "$work/colors"; mv -- "$work/colors" "$colors"; fi
if command -v termux-reload-settings >/dev/null; then
    termux-reload-settings
else
    echo 'Restart Termux to load the settings.'
fi
echo 'Done. Import the HeliBoard theme separately if you want matching colors.'
