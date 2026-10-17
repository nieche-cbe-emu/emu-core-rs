
use std::collections::BTreeMap;

use crate::machine::{Emu, Mach};

pub const MAX_LIST: usize = 200;

#[derive(Default)]
pub struct Cheat {
    width: u32,

    cands: Option<Vec<u32>>,

    pub locks: BTreeMap<u32, (u32, i64)>,
}

fn valid(width: u32) -> bool {
    matches!(width, 1 | 2 | 4)
}

fn pack(uc: &Emu, width: u32, value: i64) -> Vec<u8> {
    let v = (value as u64) & ((1u64 << (8 * width)) - 1);
    let b = v.to_le_bytes();
    let mut out = b[..width as usize].to_vec();
    if !uc.le() {
        out.reverse();
    }
    out
}

fn regions(uc: &Emu) -> Vec<(u32, u32)> {
    let st = uc.get_data();
    let mut r = vec![(st.rw_base, st.rw_size)];
    if st.heap.cur > st.heap.base {
        r.push((st.heap.base, st.heap.cur - st.heap.base));
    }
    r
}

impl Cheat {
    pub fn reset(&mut self) {
        self.width = 0;
        self.cands = None;
    }

    pub fn search(&mut self, uc: &Emu, width: u32, value: i64) -> u32 {
        if !valid(width) {
            return 0;
        }
        let pat = pack(uc, width, value);
        let w = width as usize;
        let full = self.cands.is_none() || width != self.width;
        if full {
            self.width = width;
            let mut out = Vec::new();
            for (base, size) in regions(uc) {
                let buf = uc.mem_read_as_vec(base as u64, size as usize).unwrap_or_default();
                let mut i = 0usize;
                while i + w <= buf.len() {
                    if buf[i..i + w] == pat[..] {
                        out.push(base + i as u32);
                    }
                    i += w;
                }
            }
            self.cands = Some(out);
        } else if let Some(c) = self.cands.take() {
            let kept: Vec<u32> = c
                .into_iter()
                .filter(|&a| uc.mem_read_as_vec(a as u64, w).map(|b| b == pat).unwrap_or(false))
                .collect();
            self.cands = Some(kept);
        }
        self.cands.as_ref().map(|c| c.len() as u32).unwrap_or(0)
    }

    pub fn results(&self, uc: &Emu, limit: usize) -> Vec<(u32, u32)> {
        let Some(c) = &self.cands else { return Vec::new() };
        let w = self.width as usize;
        c.iter()
            .take(limit)
            .map(|&a| {
                let mut b = uc.mem_read_as_vec(a as u64, w).unwrap_or_else(|_| vec![0; w]);
                if !uc.le() {
                    b.reverse();
                }
                let mut v = [0u8; 4];
                v[..w].copy_from_slice(&b);
                (a, u32::from_le_bytes(v))
            })
            .collect()
    }

    pub fn write(uc: &mut Emu, addr: u32, width: u32, value: i64) -> bool {
        if !valid(width) {
            return false;
        }
        let b = pack(uc, width, value);
        uc.mem_write(addr as u64, &b).is_ok()
    }

    pub fn lock(&mut self, addr: u32, width: u32, value: i64, on: bool) {
        if !on {
            self.locks.remove(&addr);
        } else if valid(width) {
            self.locks.insert(addr, (width, value));
        }
    }

    pub fn apply_locks(&self, uc: &mut Emu) {
        for (&addr, &(width, value)) in &self.locks {
            Self::write(uc, addr, width, value);
        }
    }
}
