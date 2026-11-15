#!/bin/sh
# Install or update remote-debounce on OSMC. Safe to run again.
#
# usage: sudo ./install.sh [--debounce-ms N]
#
# Without --debounce-ms the script keeps the value that is already installed.
# On a first install the value is 120. The service restarts only if a file or
# the value changed.
set -eu

readonly bin=/usr/local/bin/remote-debounce
readonly unit=/etc/systemd/system/remote-debounce.service
readonly conf=/etc/default/remote-debounce
readonly default_ms=120

if [ "$(id -un)" != "root" ]; then
    echo "Need root - sudoing..."
    exec sudo "$0" "$@"
fi

cd "$(dirname "$0")"

debounce_ms=
while [ $# -gt 0 ]; do
    case $1 in
        --debounce-ms)
            [ $# -ge 2 ] || { echo "--debounce-ms needs a value" >&2; exit 2; }
            debounce_ms=$2
            shift 2
            ;;
        *)
            echo "usage: $0 [--debounce-ms N]" >&2
            exit 2
            ;;
    esac
done

if [ -z "$debounce_ms" ]; then
    # Keep the installed value.
    if [ -f "$conf" ]; then
        debounce_ms=$(sed -n 's/^DEBOUNCE_MS=//p' "$conf" | tail -n 1)
    fi
    debounce_ms=${debounce_ms:-$default_ms}
fi
case $debounce_ms in
    ''|*[!0-9]*) echo "debounce-ms must be a whole number: '$debounce_ms'" >&2; exit 2 ;;
esac

changed=0
daemon_reload=0

# Copy a file only if its content differs. Set the flags on change.
install_file() { # src dst mode
    if [ -f "$2" ] && cmp -s "$1" "$2"; then
        chmod "$3" "$2"
        return 1
    fi
    install -m "$3" "$1" "$2"
    echo "updated $2"
    return 0
}

install_file ./remote-debounce "$bin" 755 && changed=1
install_file ./remote-debounce.service "$unit" 644 && { changed=1; daemon_reload=1; }

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
echo "DEBOUNCE_MS=$debounce_ms" > "$tmp"
install_file "$tmp" "$conf" 644 && changed=1

[ "$daemon_reload" = 1 ] && systemctl daemon-reload
systemctl enable remote-debounce >/dev/null 2>&1
if systemctl is-active --quiet remote-debounce; then
    if [ "$changed" = 1 ]; then
        systemctl restart remote-debounce
        echo "restarted remote-debounce"
    else
        echo "no change, service left running"
    fi
else
    systemctl start remote-debounce
    echo "started remote-debounce"
fi

echo "debounce-ms: $debounce_ms"
systemctl status remote-debounce --no-pager | head -n 5
