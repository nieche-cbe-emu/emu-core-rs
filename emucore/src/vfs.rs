
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const OPEN_FAIL: u32 = 0xFFFF_FFFF;

pub const GLUE_PREFIXES: &[&str] = &[
    "dfwsms",
    "dfwmix",
    "wpay",
    "cdlist",
    "cwstorecfg",
    "wstore_host",
    "coolbar_list",
    "downinfo3",
];

pub struct Vfs {
    pub root: PathBuf,
    pub base: Option<PathBuf>,
    pub handles: HashMap<u32, File>,
    next_h: u32,

    pub self_path: Option<PathBuf>,
}

pub fn downinfo_blob() -> Vec<u8> {
    let name = b"offline";
    let mut body = vec![0u8; 8];
    body.push(name.len() as u8);
    body.extend_from_slice(name);
    body.extend_from_slice(&[0, 0, 0, 0, 1, 1]);
    body.extend_from_slice(&[0u8; 12]);
    body.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&[0u8; 11]);
    body.resize(0x80, 0);
    let n = (body.len() - 4) as u16;
    body[2..4].copy_from_slice(&n.to_be_bytes());
    let mut out = Vec::new();
    for chunk in body.chunks(127) {
        out.push(0x80 | chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out
}

pub const SELF_PATH: &str = "C:\\.nieche\\self.cbe";
const SELF_NORM: &str = ".nieche/self.cbe";

impl Vfs {
    pub fn new(root: PathBuf, base: Option<PathBuf>) -> Vfs {
        let _ = std::fs::create_dir_all(&root);
        let base = base.filter(|b| b.is_dir());

        if base.is_none() {
            let _ = std::fs::create_dir_all(root.join(crate::api::fileio::COOLBAR_DIR));
        }
        Vfs {
            root,
            base,
            handles: HashMap::new(),
            next_h: 1,
            self_path: None,
        }
    }

    fn norm(path: &str) -> String {
        let p = path.replace('\\', "/");
        let p = p.trim_start_matches('/');

        let b = p.as_bytes();
        let p = if b.len() > 1 && b[1] == b':' {
            p[2..].trim_start_matches('/')
        } else {
            p
        };
        let mut out: Vec<&str> = Vec::new();
        for comp in p.split('/') {
            match comp {
                "" | "." => {}
                ".." => {
                    out.pop();
                }
                c => out.push(c),
            }
        }
        out.join("/")
    }

    pub fn host_path(&self, path: &str) -> PathBuf {
        ci_join(&self.root, &Self::norm(path))
    }

    pub fn resolve(&self, path: &str) -> PathBuf {
        let rel = Self::norm(path);
        if rel.eq_ignore_ascii_case(SELF_NORM) {
            if let Some(p) = &self.self_path {
                return p.clone();
            }
        }
        let over = ci_join(&self.root, &rel);
        if over.exists() {
            return over;
        }
        if let Some(b) = &self.base {
            let p = ci_join(b, &rel);
            if p.exists() {
                return p;
            }
        }
        over
    }

    pub fn is_glue_file(path: &str) -> bool {
        let n = Self::norm(path);
        let base = n.rsplit('/').next().unwrap_or("").to_lowercase();
        GLUE_PREFIXES.iter().any(|p| base.starts_with(p))
    }

    pub fn synth_content(path: &str) -> Option<Vec<u8>> {
        let n = Self::norm(path);
        let base = n.rsplit('/').next().unwrap_or("").to_lowercase();
        (base == "downinfo.dat").then(downinfo_blob)
    }

    pub fn open(&mut self, path: &str, mode: &str) -> u32 {
        let m = mode.to_lowercase();
        let m = if m.chars().any(|c| "rwa+".contains(c)) {
            m
        } else {

            "r".to_string()
        };
        let writing = m.contains('w') || m.contains('a') || m.contains('+');
        if Self::norm(path).eq_ignore_ascii_case(SELF_NORM) {

            let Some(p) = self.self_path.clone().filter(|_| !writing) else {
                return OPEN_FAIL;
            };
            let Ok(f) = File::open(p) else { return OPEN_FAIL };
            let h = self.next_h;
            self.next_h += 1;
            self.handles.insert(h, f);
            return h;
        }
        let hp = if writing {
            self.host_path(path)
        } else {
            self.resolve(path)
        };

        if path.is_empty() || path.ends_with('\\') || path.ends_with('/') || hp.is_dir() {
            return OPEN_FAIL;
        }
        let create = m.contains('w') || m.contains('a');
        if !hp.exists() {
            let synth = if create { None } else { Self::synth_content(path) };
            if let Some(data) = synth {
                mkparent(&hp);
                let _ = std::fs::write(&hp, data);
            } else if !create && Self::is_glue_file(path) {
                mkparent(&hp);
                let _ = File::create(&hp);
            } else if !create {
                return OPEN_FAIL;
            } else {
                mkparent(&hp);
                let _ = File::create(&hp);
            }
        }
        let mut o = OpenOptions::new();
        let r = if m.contains('w') {

            o.read(true).write(true).truncate(true).create(true).open(&hp)
        } else if m.contains('a') {
            o.read(true).append(true).create(true).open(&hp)
        } else if m.contains('+') {
            o.read(true).write(true).open(&hp)
        } else {
            o.read(true).open(&hp)
        };
        let Ok(f) = r else { return OPEN_FAIL };
        let h = self.next_h;
        self.next_h += 1;
        self.handles.insert(h, f);
        h
    }

    pub fn close(&mut self, h: u32) {
        self.handles.remove(&h);
    }

    pub fn exists(&self, path: &str) -> bool {
        self.resolve(path).exists()
    }

    pub fn size(&mut self, h: u32) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return 0;
        };
        let cur = f.stream_position().unwrap_or(0);
        let n = f.seek(SeekFrom::End(0)).unwrap_or(0);
        let _ = f.seek(SeekFrom::Start(cur));
        n as u32
    }

    pub fn read(&mut self, h: u32, n: usize) -> Vec<u8> {
        let Some(f) = self.handles.get_mut(&h) else {
            return Vec::new();
        };
        let mut v = vec![0u8; n];
        match f.read(&mut v) {
            Ok(k) => {
                v.truncate(k);
                v
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn write(&mut self, h: u32, data: &[u8]) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return 0;
        };
        match f.write(data) {
            Ok(k) => k as u32,
            Err(_) => 0,
        }
    }

    pub fn seek(&mut self, h: u32, off: i64, whence: u32) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return OPEN_FAIL;
        };

        let pos = match whence {
            0 => SeekFrom::Start(off.max(0) as u64),
            1 => SeekFrom::Current(off),
            2 => SeekFrom::End(off),
            _ => return OPEN_FAIL,
        };
        match f.seek(pos) {
            Ok(p) => p as u32,
            Err(_) => OPEN_FAIL,
        }
    }

    pub fn tell(&mut self, h: u32) -> u32 {
        self.handles
            .get_mut(&h)
            .and_then(|f| f.stream_position().ok())
            .unwrap_or(0) as u32
    }

    pub fn mkdir(&self, path: &str) -> bool {
        std::fs::create_dir_all(self.host_path(path)).is_ok()
    }
}

fn ci_join(base: &Path, rel: &str) -> PathBuf {
    let mut cur = base.to_path_buf();
    let mut parts = rel.split('/').filter(|c| !c.is_empty()).peekable();
    while let Some(comp) = parts.next() {
        let exact = cur.join(comp);
        if exact.exists() {
            cur = exact;
            continue;
        }
        let want = comp.to_lowercase();
        let hit = std::fs::read_dir(&cur).ok().and_then(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .find(|n| n.to_string_lossy().to_lowercase() == want)
        });
        match hit {
            Some(n) => cur = cur.join(n),
            None => {

                cur = exact;
                for rest in parts {
                    cur = cur.join(rest);
                }
                break;
            }
        }
    }
    cur
}

fn mkparent(p: &Path) {
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn downinfo_blob_matches_python() {
        let want = "ff0000007c00000000076f66666c696e650000000001010000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008100";
        let got: String = downinfo_blob().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(got, want);
    }

    use super::*;

    #[test]
    fn norm_clamps_parent_dir() {
        for (raw, want) in [
            ("a/b.dat", "a/b.dat"),
            ("..\\..\\escape.txt", "escape.txt"),
            ("C:\\x\\y", "x/y"),
            ("/abs/p", "abs/p"),
            ("a/./b", "a/b"),
            ("a/../b", "b"),
            ("a/b/../../../../c", "c"),
            ("", ""),
            ("..", ""),
            (".system/MB_MSTAR_WQVGA\\AppStore.CBE", ".system/MB_MSTAR_WQVGA/AppStore.CBE"),
        ] {
            assert_eq!(Vfs::norm(raw), want, "norm({raw:?})");
        }
    }

    #[test]
    fn write_stays_inside_sandbox() {
        let tmp = std::env::temp_dir().join(format!("nieche-vfs-esc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root = tmp.join("fs").join("mod");
        let mut v = Vfs::new(root.clone(), None);

        let h = v.open("..\\..\\escaped.txt", "w+");
        assert_ne!(h, OPEN_FAIL);
        v.write(h, b"x");
        v.close(h);
        assert!(root.join("escaped.txt").exists(), "应落在沙盒里");
        assert!(!tmp.join("escaped.txt").exists(), "不能落到沙盒外面");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn case_insensitive_like_fat() {
        let tmp = std::env::temp_dir().join(format!("nieche-vfs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut v = Vfs::new(tmp.clone(), None);

        let h = v.open(".system/MB_MSTAR_WQVGA\\IsLet1.dat", "w+");
        assert_ne!(h, OPEN_FAIL);
        v.write(h, b"save");
        v.close(h);

        assert!(v.exists(".system/MB_MSTAR_WQVGA\\Islet1.dat"), "换大小写要找得到");
        assert!(v.exists(".SYSTEM/mb_mstar_wqvga\\ISLET1.DAT"), "目录层级也要不敏感");

        let h = v.open(".system/MB_MSTAR_WQVGA\\Islet1.dat", "r");
        assert_ne!(h, OPEN_FAIL);
        assert_eq!(v.read(h, 16), b"save");
        v.close(h);

        let h = v.open(".system/MB_MSTAR_WQVGA\\ISLET1.DAT", "w+");
        v.write(h, b"v2");
        v.close(h);
        let n = std::fs::read_dir(tmp.join(".system/MB_MSTAR_WQVGA")).unwrap().count();
        assert_eq!(n, 1, "只差大小写的写入必须落到同一个文件");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
