# remote-debounce

Drops duplicate key presses from a worn-out remote. The program is a small Rust
service for the OSMC Vero.

## How it works

1. It finds every evdev device with the given USB vendor and product ID. The
   defaults are the OSMC remote dongle (`2017:1690`). The dongle has two
   interfaces. The program filters both.
2. It creates a uinput copy of each device, named `debounced: <name>`.
3. It grabs the original device. Kodi no longer sees it.
4. It forwards all events to the copy. It drops a key press that follows the
   previous press of the same key within the window (default 120 ms). It also
   drops the release and the repeats that belong to the dropped press.

The service scans for the dongle every 2 seconds. It handles unplug and replug.

## Build and test

```bash
cargo test
cargo build --release --target aarch64-unknown-linux-musl
```

The result is a static binary for the Vero (aarch64). The binary is not
committed.

## Install on OSMC (needs explicit approval)

With a release zip, you do not need Rust:

```bash
unzip remote-debounce-*.zip && cd remote-debounce-*/
./install_to_osmc.sh [--host HOST] [--debounce-ms N]   # copy the files to OSMC
# then on OSMC:
cd ~/remote_debounce && sudo ./install.sh [--debounce-ms N]
```

From the source tree, `install_to_osmc.sh` uses `target/<arch>/release/remote-debounce`
if it exists. It builds the binary only if none exists, or with `--rebuild`.
A build needs the Rust toolchain and the `aarch64-unknown-linux-musl` target.

`install.sh` is safe to run again. It copies a file only if the content
differs. It restarts the service only if a file or the value changed. Without
`--debounce-ms` it keeps the installed value (120 ms on a first install). The
value is stored in `/etc/default/remote-debounce`. To change it later, run
`sudo ./install.sh --debounce-ms 150` again.

To roll back, run `systemctl disable --now remote-debounce`. Kodi then sees the
original devices again.

Use `--verbose` on the binary to log dropped events. Tune the debounce time to
the chatter of your remote.

## Release

```bash
./release.sh --repo OWNER/NAME [--dry-run] [--yes] [--allow-dirty]
```

The script runs the tests, builds the binary and makes
`dist/remote-debounce-<version>-aarch64-unknown-linux-musl.zip` with a SHA-256
file. It then asks for confirmation and publishes the tag
`remote-debounce-v<version>` with `gh`. If the release exists, it replaces the
files. `--dry-run` stops after the zip. Raise the version in `Cargo.toml` first.

## Not verified

Local tests cover only the filter logic. Nobody has run the service on the
Vero. Check that Kodi picks up the `debounced:` devices.
