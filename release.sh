#!/bin/sh
# Build a release zip with the aarch64 binary and publish it on GitHub.
#
# usage: ./release.sh --repo OWNER/NAME [--dry-run] [--yes] [--allow-dirty]
#
# --dry-run      build the zip in dist/ and stop. Nothing goes to GitHub.
# --yes          do not ask before publishing.
# --allow-dirty  allow uncommitted changes in this directory.
#
# The tag is remote-debounce-v<version from Cargo.toml>. If the release
# exists, the script replaces the files of the release. It needs cargo, zip
# and an authenticated gh.
set -eu
cd "$(dirname "$0")"

readonly arch=aarch64-unknown-linux-musl

repo=
dry_run=0
assume_yes=0
allow_dirty=0
while [ $# -gt 0 ]; do
    case $1 in
        --repo)
            [ $# -ge 2 ] || { echo "--repo needs a value" >&2; exit 2; }
            repo=$2
            shift 2
            ;;
        --dry-run) dry_run=1; shift ;;
        --yes) assume_yes=1; shift ;;
        --allow-dirty) allow_dirty=1; shift ;;
        *) echo "usage: $0 --repo OWNER/NAME [--dry-run] [--yes] [--allow-dirty]" >&2; exit 2 ;;
    esac
done
if [ -z "$repo" ] && [ "$dry_run" = 0 ]; then
    echo "--repo OWNER/NAME is required" >&2
    exit 2
fi

if [ "$allow_dirty" = 0 ] && [ -n "$(git status --porcelain -- .)" ]; then
    echo "Uncommitted changes in $(pwd). Commit them or use --allow-dirty." >&2
    exit 1
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
tag=remote-debounce-v$version
name=remote-debounce-$version-$arch

cargo test
cargo build --release --target "$arch"

stage=dist/$name
rm -rf dist
mkdir -p "$stage"
cp "target/$arch/release/remote-debounce" remote-debounce.service \
    install.sh install_to_osmc.sh README.md "$stage/"
chmod +x "$stage/remote-debounce" "$stage/install.sh" "$stage/install_to_osmc.sh"
(cd dist && zip -qr "$name.zip" "$name" && sha256sum "$name.zip" > "$name.zip.sha256")
echo "built dist/$name.zip"
cat "dist/$name.zip.sha256"

[ "$dry_run" = 1 ] && exit 0

if [ "$assume_yes" = 0 ]; then
    printf 'Publish %s to %s? [y/N] ' "$tag" "$repo"
    read -r answer
    [ "$answer" = y ] || { echo "cancelled"; exit 1; }
fi

files="dist/$name.zip dist/$name.zip.sha256"
if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    gh release upload "$tag" $files --clobber --repo "$repo"
else
    # shellcheck disable=SC2086
    gh release create "$tag" $files --repo "$repo" \
        --title "remote-debounce $version" \
        --notes "Debounce proxy for the OSMC remote dongle (aarch64, static binary). Unzip, then run ./install_to_osmc.sh. See README.md."
fi
