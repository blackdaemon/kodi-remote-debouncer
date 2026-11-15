//! Key debounce logic. It has no I/O so that unit tests can drive it.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_MSC: u16 = 0x04;
pub const MSC_SCAN: u16 = 0x04;

/// One input event as (type, code, value).
pub type Ev = (u16, u16, i32);

pub struct Debouncer {
    window: Duration,
    last_press: HashMap<u16, Instant>,
    /// Keys that the filter has passed as pressed and not yet as released.
    down: HashSet<u16>,
    /// Keys whose press was dropped. Their release must be dropped too.
    dropped: HashSet<u16>,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            last_press: HashMap::new(),
            down: HashSet::new(),
            dropped: HashSet::new(),
        }
    }

    /// Filter one packet (events up to and including SYN_REPORT).
    /// Returns the events to forward. The result is empty if nothing is left.
    pub fn filter_packet(&mut self, packet: &[Ev], now: Instant) -> Vec<Ev> {
        let mut out = Vec::with_capacity(packet.len());
        let mut dropped_any = false;
        for &ev in packet {
            let (ty, code, value) = ev;
            if ty != EV_KEY {
                out.push(ev);
                continue;
            }
            let keep = match value {
                1 => self.press(code, now),
                0 => self.release(code),
                // Auto-repeat counts only while the key is really down.
                _ => self.down.contains(&code),
            };
            if keep {
                out.push(ev);
            } else {
                dropped_any = true;
            }
        }
        if dropped_any {
            // The scan code belongs to the dropped key event.
            out.retain(|&(ty, code, _)| !(ty == EV_MSC && code == MSC_SCAN));
        }
        if out.iter().all(|&(ty, _, _)| ty == EV_SYN) {
            out.clear();
        }
        out
    }

    fn press(&mut self, code: u16, now: Instant) -> bool {
        if let Some(&last) = self.last_press.get(&code) {
            if now.saturating_duration_since(last) < self.window {
                self.dropped.insert(code);
                return false;
            }
        }
        self.last_press.insert(code, now);
        self.dropped.remove(&code);
        self.down.insert(code);
        true
    }

    fn release(&mut self, code: u16) -> bool {
        if self.dropped.remove(&code) {
            return false;
        }
        self.down.remove(&code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_OK: u16 = 28;
    const KEY_UP: u16 = 103;
    const SYN: Ev = (EV_SYN, 0, 0);

    fn key(code: u16, value: i32) -> Vec<Ev> {
        vec![(EV_MSC, MSC_SCAN, code as i32), (EV_KEY, code, value), SYN]
    }

    fn run(d: &mut Debouncer, t0: Instant, ms: u64, pkt: Vec<Ev>) -> Vec<Ev> {
        d.filter_packet(&pkt, t0 + Duration::from_millis(ms))
    }

    #[test]
    fn single_click_passes() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        assert_eq!(run(&mut d, t, 0, key(KEY_OK, 1)).len(), 3);
        assert_eq!(run(&mut d, t, 60, key(KEY_OK, 0)).len(), 3);
    }

    #[test]
    fn bounce_press_and_its_release_are_dropped() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        run(&mut d, t, 0, key(KEY_OK, 1));
        assert_eq!(run(&mut d, t, 20, key(KEY_OK, 0)).len(), 3);
        assert!(run(&mut d, t, 50, key(KEY_OK, 1)).is_empty());
        assert!(run(&mut d, t, 70, key(KEY_OK, 0)).is_empty());
    }

    #[test]
    fn press_after_window_passes() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        run(&mut d, t, 0, key(KEY_OK, 1));
        run(&mut d, t, 30, key(KEY_OK, 0));
        assert_eq!(run(&mut d, t, 200, key(KEY_OK, 1)).len(), 3);
    }

    #[test]
    fn other_keys_are_independent() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        run(&mut d, t, 0, key(KEY_OK, 1));
        assert_eq!(run(&mut d, t, 10, key(KEY_UP, 1)).len(), 3);
    }

    #[test]
    fn repeat_of_dropped_key_is_dropped() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        run(&mut d, t, 0, key(KEY_OK, 1));
        run(&mut d, t, 10, key(KEY_OK, 0));
        run(&mut d, t, 30, key(KEY_OK, 1));
        assert!(run(&mut d, t, 60, key(KEY_OK, 2)).is_empty());
    }

    #[test]
    fn held_key_repeats_pass() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t = Instant::now();
        run(&mut d, t, 0, key(KEY_OK, 1));
        assert_eq!(run(&mut d, t, 500, key(KEY_OK, 2)).len(), 3);
    }

    #[test]
    fn non_key_packets_pass() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let pkt = vec![(0x02, 0, 1), SYN];
        assert_eq!(d.filter_packet(&pkt, Instant::now()), pkt);
    }
}
