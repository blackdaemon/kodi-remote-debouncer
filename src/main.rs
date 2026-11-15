//! Debounce proxy for a worn-out remote control.
//!
//! The program grabs every evdev device that matches a USB vendor and product
//! ID. It copies the device to a uinput device and drops key presses that
//! come again within a short time window. Kodi then reads the clean copy.

mod debounce;

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use evdev::uinput::VirtualDevice;
use evdev::{Device, InputEvent};

use debounce::{Debouncer, Ev, EV_SYN};

const CLONE_PREFIX: &str = "debounced: ";
const RESCAN: Duration = Duration::from_secs(2);

struct Config {
    vendor: u16,
    product: u16,
    window: Duration,
    verbose: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage: remote-debounce [--vid HEX] [--pid HEX] [--debounce-ms N] [--verbose]\n\
         defaults: --vid 2017 --pid 1690 --debounce-ms 120 (OSMC remote dongle)"
    );
    std::process::exit(2);
}

fn parse_args() -> Config {
    let mut cfg = Config {
        vendor: 0x2017,
        product: 0x1690,
        window: Duration::from_millis(120),
        verbose: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--vid" => cfg.vendor = u16::from_str_radix(&value(), 16).unwrap_or_else(|_| usage()),
            "--pid" => cfg.product = u16::from_str_radix(&value(), 16).unwrap_or_else(|_| usage()),
            "--debounce-ms" => {
                let ms: u64 = value().parse().unwrap_or_else(|_| usage());
                cfg.window = Duration::from_millis(ms);
            }
            "--verbose" => cfg.verbose = true,
            _ => usage(),
        }
    }
    cfg
}

fn event_paths() -> Vec<PathBuf> {
    let Ok(dir) = std::fs::read_dir("/dev/input") else {
        return Vec::new();
    };
    dir.flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("event"))
        })
        .collect()
}

fn build_clone(dev: &Device) -> io::Result<VirtualDevice> {
    let name = format!("{CLONE_PREFIX}{}", dev.name().unwrap_or("remote"));
    let mut builder = VirtualDevice::builder()?
        .name(&name)
        .input_id(dev.input_id());
    if let Some(keys) = dev.supported_keys() {
        builder = builder.with_keys(keys)?;
    }
    if let Some(axes) = dev.supported_relative_axes() {
        builder = builder.with_relative_axes(axes)?;
    }
    if let Some(msc) = dev.misc_properties() {
        builder = builder.with_msc(msc)?;
    }
    if dev.supported_absolute_axes().is_some() {
        eprintln!("warning: {name}: absolute axes are not copied");
    }
    builder.build()
}

fn proxy(path: &Path, mut dev: Device, cfg: &Config) -> io::Result<()> {
    let mut clone = build_clone(&dev)?;
    dev.grab()?;
    eprintln!(
        "filtering {} ({}), window {} ms",
        path.display(),
        dev.name().unwrap_or("?"),
        cfg.window.as_millis()
    );
    let mut debouncer = Debouncer::new(cfg.window);
    let mut packet: Vec<Ev> = Vec::new();
    loop {
        let events: Vec<InputEvent> = dev.fetch_events()?.collect();
        for ev in events {
            packet.push((ev.event_type().0, ev.code(), ev.value()));
            if ev.event_type().0 != EV_SYN || ev.code() != 0 {
                continue;
            }
            let out = debouncer.filter_packet(&packet, Instant::now());
            if cfg.verbose && out.len() < packet.len() {
                eprintln!("{}: dropped {:?}", path.display(), packet);
            }
            packet.clear();
            if out.is_empty() {
                continue;
            }
            let raw: Vec<InputEvent> = out
                .iter()
                .map(|&(ty, code, value)| InputEvent::new(ty, code, value))
                .collect();
            clone.emit(&raw)?;
        }
    }
}

fn main() {
    let cfg = Arc::new(parse_args());
    let active: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
    let mut ignored: HashSet<PathBuf> = HashSet::new();
    loop {
        let paths = event_paths();
        ignored.retain(|p| paths.contains(p));
        for path in paths {
            if ignored.contains(&path) || active.lock().unwrap().contains(&path) {
                continue;
            }
            let Ok(dev) = Device::open(&path) else {
                continue;
            };
            let id = dev.input_id();
            let ours = dev.name().is_some_and(|n| n.starts_with(CLONE_PREFIX));
            if ours || id.vendor() != cfg.vendor || id.product() != cfg.product {
                ignored.insert(path);
                continue;
            }
            active.lock().unwrap().insert(path.clone());
            let (cfg, active) = (Arc::clone(&cfg), Arc::clone(&active));
            thread::spawn(move || {
                if let Err(e) = proxy(&path, dev, &cfg) {
                    eprintln!("{}: stopped: {e}", path.display());
                }
                active.lock().unwrap().remove(&path);
            });
        }
        thread::sleep(RESCAN);
    }
}
