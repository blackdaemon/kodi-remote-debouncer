#!/bin/sh
# Copy remote-debounce to OSMC and print the install command.
#
# usage: ./install_to_osmc.sh [--host HOST] [--rebuild] [--debounce-ms N]
#
# The script uses a binary that already exists. It looks for ./remote-debounce
# (the release zip) and then target/<arch>/release/remote-debounce. It builds
# the binary only if none exists, or with --rebuild. A build needs the Rust
# toolchain and the aarch64-unknown-linux-musl target. With a release zip you
# do not need Rust.
# Other arguments go to install.sh, for example --debounce-ms 150.
set -eu
cd "$(dirname "$0")"

readonly remote_dir='~/remote_debounce'
readonly arch=aarch64-unknown-linux-musl
readonly built=target/$arch/release/remote-debounce

host=osmcv
rebuild=0
install_args=
while [ $# -gt 0 ]; do
    case $1 in
        --host)
            [ $# -ge 2 ] || { echo "--host needs a value" >&2; exit 2; }
            host=$2
            shift 2
            ;;
        --rebuild) rebuild=1; shift ;;
        *) install_args="$install_args $1"; shift ;;
    esac
done

binary=
if [ "$rebuild" = 0 ]; then
    for candidate in ./remote-debounce "$built"; do
        if [ -f "$candidate" ]; then
            binary=$candidate
            break
        fi
    done
fi

if [ -z "$binary" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "No remote-debounce binary found and cargo is not installed." >&2
        echo "Use the release zip, or install the Rust toolchain." >&2
        exit 1
    fi
    cargo test
    cargo build --release --target "$arch"
    binary=$built
fi
echo "using binary: $binary"

ssh "$host" "mkdir -p $remote_dir"
scp ./remote-debounce.service "$binary" ./install.sh "$host:$remote_dir/"
ssh "$host" "chmod +x $remote_dir/install.sh"

printf '\nssh into %s and run:\n\tcd %s && sudo ./install.sh%s\n' \
    "$host" "$remote_dir" "$install_args"
