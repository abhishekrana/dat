//! Reading positions: the top source line per file, one small file each under the state dir, so readers of
//! different notes never write the same file and a crash mid-write leaves the old entry whole.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DIR: &str = "positions";
/// Entries kept; the least recently written go first.
const MAX_ENTRIES: usize = 200;

/// One note's position. The path is stored so a hash collision reads as no entry, not as another note's line.
#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    path: String,
    line: usize,
}

#[derive(Debug)]
pub struct Positions {
    dir: PathBuf,
}

impl Positions {
    /// The store under `state_dir`; nothing is read or created until a position is.
    #[must_use]
    pub fn open(state_dir: &Path) -> Self {
        Self {
            dir: state_dir.join(DIR),
        }
    }

    /// The remembered line; a missing, unreadable or foreign entry is none.
    #[must_use]
    pub fn get(&self, file: &Path) -> Option<usize> {
        let key = key(file)?;
        let text = std::fs::read_to_string(self.entry(&key)).ok()?;
        let entry: Entry = toml::from_str(&text).ok()?;
        (entry.path == key).then_some(entry.line)
    }

    /// Writes the line for `file` atomically, then drops the oldest entries past the cap.
    pub fn set(&self, file: &Path, line: usize) -> std::io::Result<()> {
        let Some(key) = key(file) else { return Ok(()) };
        std::fs::create_dir_all(&self.dir)?;
        let path = self.entry(&key);
        let text = toml::to_string(&Entry { path: key, line }).map_err(std::io::Error::other)?;
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)?;
        self.prune()
    }

    fn entry(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{:016x}.toml", fnv1a(key.as_bytes())))
    }

    fn prune(&self) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.dir)?
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if entries.len() <= MAX_ENTRIES {
            return Ok(());
        }
        entries.sort();
        for (_, path) in &entries[..entries.len() - MAX_ENTRIES] {
            // Another reader may have pruned it first.
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

fn key(file: &Path) -> Option<String> {
    file.canonicalize().ok().map(|p| p.display().to_string())
}

/// FNV-1a, 64-bit: a file name that stays the same across builds, which `std`'s hasher does not promise.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dat-pos-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn two_readers_keep_each_others_positions() {
        let dir = scratch("two");
        let (a, b) = (dir.join("a.md"), dir.join("b.md"));
        std::fs::write(&a, "x").expect("a");
        std::fs::write(&b, "x").expect("b");
        let (first, second) = (Positions::open(&dir), Positions::open(&dir));
        assert_eq!(first.get(&a), None);
        first.set(&a, 42).expect("set a");
        second.set(&b, 7).expect("set b");
        first.set(&a, 43).expect("set a again");
        let fresh = Positions::open(&dir);
        assert_eq!((fresh.get(&a), fresh.get(&b)), (Some(43), Some(7)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_oldest_entries_go_past_the_cap() {
        let dir = scratch("cap");
        let p = Positions::open(&dir);
        let oldest = dir.join("oldest.md");
        std::fs::write(&oldest, "x").expect("note");
        p.set(&oldest, 1).expect("set");
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(60);
        let entry = p.entry(&key(&oldest).expect("key"));
        std::fs::File::options()
            .write(true)
            .open(&entry)
            .expect("open")
            .set_modified(old)
            .expect("mtime");
        for i in 0..MAX_ENTRIES {
            let f = dir.join(format!("{i}.md"));
            std::fs::write(&f, "x").expect("file");
            p.set(&f, i).expect("set");
        }
        assert_eq!(p.get(&oldest), None, "the oldest entry was pruned");
        assert_eq!(p.get(&dir.join("7.md")), Some(7));
        assert_eq!(std::fs::read_dir(dir.join(DIR)).expect("dir").count(), MAX_ENTRIES);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_file_name_hash_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
