
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::machine::{Emu, Mach};

fn safe(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() {
        "unnamed".into()
    } else {
        s
    }
}

fn home() -> PathBuf {
    match std::env::var("NIECHE_HOME").or_else(|_| std::env::var("NICAI_HOME")) {
        Ok(v) if !v.is_empty() => PathBuf::from(v),

        _ => crate::runtime::user_home().join(".nieche-emu"),
    }
}

fn save_path(module: &str, name: &str) -> PathBuf {
    home().join("saves").join(safe(module)).join(safe(name))
}

pub fn storage_date(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 256).unwrap_or_else(|| b"save".to_vec());
    let name = String::from_utf8_lossy(&raw).to_string();
    let (buf, n) = (uc.arg(1), uc.arg(2));
    let write = uc.arg(3) == 0;
    let module = uc.get_data().rt.module_name.clone();
    let path = save_path(&module, &name);

    if write {

        if n == 0 {
            uc.ret(0);
            return;
        }
        let Ok(data) = uc.mem_read_as_vec(buf as u64, n as usize) else {
            uc.ret(0);
            return;
        };
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }

        let r = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .and_then(|mut f| {
                f.seek(SeekFrom::Start(0))?;
                f.write_all(&data)
            });
        uc.ret(u32::from(r.is_ok()));
        return;
    }

    let mut d = Vec::new();
    let ok = std::fs::File::open(&path)
        .and_then(|f| Read::take(f, n as u64).read_to_end(&mut d))
        .is_ok();

    if buf != 0 && n != 0 {
        let mut v = d.clone();
        v.resize(n as usize, 0);
        uc.write(buf, &v);
    }

    uc.ret(u32::from(ok && !d.is_empty()));
}

const NV_SIZE: usize = 10240;

fn nv_path(uc: &Emu) -> PathBuf {
    save_path(&uc.get_data().rt.module_name, "nvram.bin")
}

fn nv_blob(path: &PathBuf) -> Option<Vec<u8>> {
    let mut v = std::fs::read(path).ok()?;
    v.resize(NV_SIZE, 0);
    Some(v)
}

pub fn nv_read(uc: &mut Emu) {
    let (off, buf, n, okp) = (uc.arg(0), uc.arg(1), uc.arg(2), uc.arg(3));
    let ok = nv_read_at(uc, off, buf, n, okp);
    uc.ret(ok);
}

pub fn nv_write(uc: &mut Emu) {
    let (off, buf, n, okp) = (uc.arg(0), uc.arg(1), uc.arg(2), uc.arg(3));
    let ok = nv_write_at(uc, off, buf, n, okp);
    uc.ret(ok);
}

pub fn nv_read_at(uc: &mut Emu, off: u32, buf: u32, n: u32, okp: u32) -> u32 {
    let (off, n) = (off as usize, n as usize);
    let mut ok = 0u8;
    if let Some(blob) = nv_blob(&nv_path(uc)) {
        if buf != 0 && off + n <= NV_SIZE {
            uc.write(buf, &blob[off..off + n]);
            ok = 1;
        }
    }
    if okp != 0 {
        uc.write(okp, &[ok]);
    }
    ok as u32
}

pub fn nv_record(uc: &Emu, off: usize, n: usize) -> Option<Vec<u8>> {
    nv_blob(&nv_path(uc)).map(|b| b[off..off + n].to_vec())
}

pub fn nv_write_at(uc: &mut Emu, off: u32, buf: u32, n: u32, okp: u32) -> u32 {
    let (off, n) = (off as usize, n as usize);
    let mut ok = 0u8;
    if buf != 0 && off + n <= NV_SIZE {
        let path = nv_path(uc);
        let mut blob = nv_blob(&path).unwrap_or_else(|| vec![0; NV_SIZE]);
        if let Ok(data) = uc.mem_read_as_vec(buf as u64, n) {
            blob[off..off + n].copy_from_slice(&data);
            if let Some(d) = path.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            if std::fs::write(&path, &blob).is_ok() {
                ok = 1;
            }
        }
    }
    if okp != 0 {
        uc.write(okp, &[ok]);
    }
    ok as u32
}

pub fn nv_write_tail(_uc: &mut Emu) {}
