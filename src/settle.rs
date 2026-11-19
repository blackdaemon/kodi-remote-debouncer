//! Tracks device capabilities across scans. A device that is still being
//! enumerated by the kernel reports changing capabilities, so it must not be
//! cloned until two scans in a row have seen the same ones.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Capabilities that the clone has to reproduce: key codes and relative axes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caps {
    pub keys: Vec<u16>,
    pub rel: Vec<u16>,
}

#[derive(Default)]
pub struct Settle {
    seen: HashMap<PathBuf, Caps>,
}

impl Settle {
    /// Record the capabilities seen in this scan. Returns true when they are
    /// the same as in the previous scan.
    pub fn is_settled(&mut self, path: &Path, caps: &Caps) -> bool {
        if self.seen.get(path) == Some(caps) {
            return true;
        }
        self.seen.insert(path.to_path_buf(), caps.clone());
        false
    }

    /// Drop state for paths that are gone, so a reused path starts fresh.
    pub fn retain(&mut self, paths: &[PathBuf]) {
        self.seen.retain(|p, _| paths.contains(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(keys: &[u16]) -> Caps {
        Caps { keys: keys.to_vec(), rel: vec![] }
    }

    #[test]
    fn first_sight_is_not_settled() {
        let mut s = Settle::default();
        assert!(!s.is_settled(Path::new("/dev/input/event7"), &caps(&[28, 114])));
    }

    #[test]
    fn same_caps_twice_is_settled() {
        let mut s = Settle::default();
        let p = Path::new("/dev/input/event7");
        s.is_settled(p, &caps(&[28, 114]));
        assert!(s.is_settled(p, &caps(&[28, 114])));
    }

    #[test]
    fn changing_caps_restart_the_wait() {
        let mut s = Settle::default();
        let p = Path::new("/dev/input/event7");
        s.is_settled(p, &caps(&[28]));
        assert!(!s.is_settled(p, &caps(&[28, 114])));
        assert!(s.is_settled(p, &caps(&[28, 114])));
    }

    #[test]
    fn reused_path_starts_fresh() {
        let mut s = Settle::default();
        let p = PathBuf::from("/dev/input/event7");
        s.is_settled(&p, &caps(&[28]));
        s.retain(&[]);
        assert!(!s.is_settled(&p, &caps(&[28])));
    }

    #[test]
    fn paths_are_independent() {
        let mut s = Settle::default();
        s.is_settled(Path::new("/dev/input/event7"), &caps(&[28]));
        assert!(!s.is_settled(Path::new("/dev/input/event8"), &caps(&[28])));
    }
}
