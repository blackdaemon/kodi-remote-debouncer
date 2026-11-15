#!/bin/sh
# Install or update remote-debounce on OSMC. Safe to run again.
#
# usage: sudo ./install.sh [--debounce-ms N] [--vid HEX] [--pid HEX]
#
# --vid and --pid select the remote by its USB IDs, for example 2017 and 1690
# (the OSMC remote dongle). Find them with lsusb or in
# /proc/bus/input/devices (Vendor= and Product=).
# Without an option the script keeps the value that is already installed.
# On a first install the values are 120 ms, 2017 and 1690. The service restarts
# only if a file or a value changed.
set -eu

readonly bin=/usr/local/bin/remote-debounce
readonly unit=/etc/systemd/system/remote-debounce.service
readonly conf=/etc/default/remote-debounce
readonly default_ms=120
readonly default_vid=2017
readonly default_pid=1690

if [ "$(id -un)" != "root" ]; then
    echo "Need root - sudoing..."
    exec sudo "$0" "$@"
fi

cd "$(dirname "$0")"

debounce_ms=
vid=
pid=
while [ $# -gt 0 ]; do
    case $1 in
        --debounce-ms)
            [ $# -ge 2 ] || { echo "--debounce-ms needs a value" >&2; exit 2; }
            debounce_ms=$2
            shift 2
            ;;
        --vid)
            [ $# -ge 2 ] || { echo "--vid needs a value" >&2; exit 2; }
            vid=$2
            shift 2
            ;;
        --pid)
            [ $# -ge 2 ] || { echo "--pid needs a value" >&2; exit 2; }
            pid=$2
            shift 2
            ;;
        *)
            echo "usage: $0 [--debounce-ms N] [--vid HEX] [--pid HEX]" >&2
            exit 2
            ;;
    esac
done

# Read a saved value from the config file.
saved() { # key
    [ -f "$conf" ] && sed -n "s/^$1=//p" "$conf" | tail -n 1 || true
}

# Keep the installed value if the option was not given.
[ -n "$debounce_ms" ] || debounce_ms=$(saved DEBOUNCE_MS)
[ -n "$vid" ] || vid=$(saved VID)
[ -n "$pid" ] || pid=$(saved PID)
debounce_ms=${debounce_ms:-$default_ms}
vid=${vid:-$default_vid}
pid=${pid:-$default_pid}

case $debounce_ms in
    ''|*[!0-9]*) echo "debounce-ms must be a whole number: '$debounce_ms'" >&2; exit 2 ;;
esac
for id in "$vid" "$pid"; do
    case $id in
        ''|*[!0-9a-fA-F]*) echo "vid and pid must be hex digits: '$id'" >&2; exit 2 ;;
    esac
    [ ${#id} -le 4 ] || { echo "vid and pid have at most 4 hex digits: '$id'" >&2; exit 2; }
done

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
printf 'DEBOUNCE_MS=%s\nVID=%s\nPID=%s\n' "$debounce_ms" "$vid" "$pid" > "$tmp"
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

echo "debounce-ms: $debounce_ms, vid: $vid, pid: $pid"
systemctl status remote-debounce --no-pager | head -n 5
