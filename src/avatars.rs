//! 记住每把公钥选的头像，下次进廟不必再选。
//! 和鐘聲簿一样是 append-only 文本，每行 `哈希\t头像下标`，重启时读回内存。

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct Avatars {
    path: PathBuf,
    log: BufWriter<File>,
    chosen: HashMap<String, usize>,
}

impl Avatars {
    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(data_dir)?;
        let path = data_dir.join("avatars.log");
        let chosen = load(&path);
        let log = BufWriter::new(OpenOptions::new().create(true).append(true).open(&path)?);
        Ok(Self { path, log, chosen })
    }

    pub fn get(&self, id: &str) -> Option<usize> {
        self.chosen.get(id).copied()
    }

    pub fn set(&mut self, id: &str, avatar: usize) {
        if self.chosen.get(id) == Some(&avatar) {
            return;
        }
        self.chosen.insert(id.to_string(), avatar);
        let _ = writeln!(self.log, "{id}\t{avatar}");
        let _ = self.log.flush();
    }

    /// 同一个人改过头像会留下多行，重启时压实成每人一行
    pub fn compact_if_needed(&mut self) {
        let lines = fs::read_to_string(&self.path)
            .map(|s| s.lines().count())
            .unwrap_or(0);
        if lines <= self.chosen.len() {
            return;
        }
        let tmp = self.path.with_extension("tmp");
        let Ok(file) = File::create(&tmp) else { return };
        let mut out = BufWriter::new(file);
        for (id, avatar) in &self.chosen {
            let _ = writeln!(out, "{id}\t{avatar}");
        }
        let _ = out.flush();
        drop(out);
        if fs::rename(&tmp, &self.path).is_ok() {
            if let Ok(f) = OpenOptions::new().append(true).open(&self.path) {
                self.log = BufWriter::new(f);
            }
        }
    }
}

fn load(path: &Path) -> HashMap<String, usize> {
    let mut chosen = HashMap::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return chosen;
    };
    for line in raw.lines() {
        // 后写的覆盖先写的，所以改过头像以最后一次为准
        if let Some((id, avatar)) = line.split_once('\t') {
            if let Ok(n) = avatar.parse::<usize>() {
                chosen.insert(id.to_string(), n);
            }
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_and_compacts() {
        let dir = std::env::temp_dir().join(format!("xiaozhongsi-av-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mut a = Avatars::open(&dir).unwrap();
        assert_eq!(a.get("abc"), None);
        a.set("abc", 3);
        a.set("def", 7);
        a.set("abc", 5); // 改了头像
        assert_eq!(a.get("abc"), Some(5));
        drop(a);

        // 重启后仍记得，且以最后一次为准
        let mut a = Avatars::open(&dir).unwrap();
        assert_eq!(a.get("abc"), Some(5));
        assert_eq!(a.get("def"), Some(7));
        assert_eq!(fs::read_to_string(dir.join("avatars.log")).unwrap().lines().count(), 3);
        a.compact_if_needed();
        assert_eq!(fs::read_to_string(dir.join("avatars.log")).unwrap().lines().count(), 2);

        let _ = fs::remove_dir_all(&dir);
    }
}
