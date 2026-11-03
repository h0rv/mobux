#!/usr/bin/env bash
set -euo pipefail

target=${1:?Usage: package-android.sh TARGET}
case "$target" in aarch64-linux-android|x86_64-linux-android) ;; *) echo 'Unsupported target' >&2; exit 1;; esac
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/mobux" dist
install -m 755 "target/$target/release/mobux" "$work/mobux/mobux"
install -m 644 LICENSE "$work/mobux/LICENSE"
notices="$work/mobux/THIRD_PARTY_LICENSES.txt"
printf 'Mobux dependency and asset notices\n\n' > "$notices"

# Include available dependency notices, including build-time crates. Cargo
# metadata resolves paths; filenames are data, never evaluated as shell code.
cargo metadata --locked --format-version 1 > "$work/metadata.json"
while IFS= read -r manifest; do
    directory=$(dirname -- "$manifest")
    name=$(basename -- "$directory")
    while IFS= read -r -d '' license; do
        printf '\n===== %s / %s =====\n' "$name" "$(basename -- "$license")" >> "$notices"
        cat -- "$license" >> "$notices"
    done < <(find "$directory" -maxdepth 1 -type f \( -iname '*license*' -o -iname '*copying*' -o -iname '*notice*' -o -iname 'OFL*' \) -print0)
done < <(jq -r '.packages[].manifest_path' "$work/metadata.json")
while IFS= read -r -d '' license; do
    printf '\n===== %s =====\n' "$license" >> "$notices"
    cat -- "$license" >> "$notices"
done < <(find assets themes -type f \( -iname '*license*' -o -iname 'OFL*' -o -name 'THIRD_PARTY.md' \) -print0)
asset="mobux-$target.tar.gz"
tar -czf "dist/$asset" -C "$work" mobux
(cd dist && sha256sum "$asset" > "$asset.sha256")
printf 'Packaged %s\n' "dist/$asset"
