#!/usr/bin/env bash
# Download a published Mobux Android snapshot. Run explicitly with bash in Termux.
set -euo pipefail

if [[ -z ${TERMUX_VERSION:-} || ${PREFIX:-} != /* ]]; then
    echo 'Run this installer inside Termux on Android.' >&2
    exit 1
fi
case "$(uname -m)" in
    aarch64|arm64) target=aarch64-linux-android ;;
    x86_64) target=x86_64-linux-android ;;
    *) echo 'No prebuilt binary for this architecture. See the source-build instructions in SETUP.md.' >&2; exit 1 ;;
esac
sdk=$(getprop ro.build.version.sdk)
if [[ ! $sdk =~ ^[0-9]+$ || $sdk -lt 28 ]]; then
    echo 'Prebuilt snapshots require Android 9 or newer. See SETUP.md for source builds.' >&2
    exit 1
fi
for command in curl jq tar sha256sum install; do
    if ! command -v "$command" >/dev/null; then
        echo 'Missing installer tools. Run: pkg install curl jq tar coreutils' >&2
        exit 1
    fi
done
work=$(mktemp -d "${TMPDIR:-$PREFIX/tmp}/mobux-install.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
asset="mobux-$target.tar.gz"
fetch() { curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$@"; }
fetch 'https://api.github.com/repos/h0rv/mobux/releases?per_page=20' > "$work/releases.json"
release=$(jq -c '[.[] | select(.draft == false and (.tag_name | startswith("snapshot-")))][0] // empty' "$work/releases.json")
if [[ -z $release ]]; then
    echo 'No Android snapshot has been published yet. CI may still be building. See SETUP.md for a source build.' >&2
    exit 1
fi
archive_url=$(jq -r --arg name "$asset" '.assets[] | select(.name == $name) | .browser_download_url' <<< "$release")
checksum_url=$(jq -r --arg name "$asset.sha256" '.assets[] | select(.name == $name) | .browser_download_url' <<< "$release")
if [[ -z $archive_url || -z $checksum_url ]]; then
    echo 'This snapshot does not contain the expected Android files.' >&2
    exit 1
fi
fetch "$archive_url" -o "$work/$asset"
fetch "$checksum_url" -o "$work/$asset.sha256"
expected=$(awk -v name="$asset" '$2 == name {print $1}' "$work/$asset.sha256")
if [[ ! $expected =~ ^[[:xdigit:]]{64}$ ]]; then
    echo 'Invalid checksum manifest.' >&2
    exit 1
fi
(cd "$work" && printf '%s  %s\n' "$expected" "$asset" | sha256sum --check -)
# Extract only fixed files to stdout. Archive paths and links cannot write into
# the filesystem. The checksum detects corruption; it is not a signature.
for file in mobux LICENSE THIRD_PARTY_LICENSES.txt; do
    tar -xOzf "$work/$asset" "mobux/$file" > "$work/$file"
done
install -d "$PREFIX/bin" "$PREFIX/share/doc/mobux"
install -m 644 "$work/LICENSE" "$work/THIRD_PARTY_LICENSES.txt" "$PREFIX/share/doc/mobux/"
install -m 755 "$work/mobux" "$PREFIX/bin/.mobux-new-$$"
mv -f -- "$PREFIX/bin/.mobux-new-$$" "$PREFIX/bin/mobux"
printf 'Installed %s\n' "$(jq -r '.tag_name' <<< "$release")"
"$PREFIX/bin/mobux" --version
printf '\nRun mobux in your project directory. Phone support is experimental.\n'
