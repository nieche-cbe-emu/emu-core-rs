
use crate::api::fill;
use crate::machine::{Emu, Mach};

pub fn df_malloc_in(uc: &mut Emu) {
    let (pp, size) = (uc.arg(0), uc.arg(1));
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "dF_Malloc", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.w32(pp, p);
    uc.ret(if p != 0 { 1 } else { 0 });
}

pub fn df_free(uc: &mut Emu) {
    let pp = uc.arg(0);
    if pp != 0 {
        let p = uc.r32(pp);
        uc.get_data_mut().heap.free(p);
        uc.w32(pp, 0);
    }
    uc.ret(0);
}

pub fn malloc_big(uc: &mut Emu) {
    let size = uc.arg(0);
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "bigmem", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.ret(p);
}

pub fn free_big(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.get_data_mut().heap.free(p);
    uc.ret(0);
}

pub fn remalloc_big(uc: &mut Emu) {
    let (old, size) = (uc.arg(0), uc.arg(1));
    let p = uc
        .get_data_mut()
        .heap
        .alloc(size, "bigmem", false)
        .unwrap_or(0);
    if old != 0 {
        let n = uc.get_data().heap.block_size(old).unwrap_or(0).min(size);
        if n != 0 && p != 0 {
            if let Ok(b) = uc.mem_read_as_vec(old as u64, n as usize) {
                uc.write(p, &b);
            }
        }
        uc.get_data_mut().heap.free(old);
    }
    uc.ret(p);
}

fn gblock_ptr(uc: &mut Emu) -> u32 {
    let cur = uc.get_data().rt.gblock;
    if cur != 0 {
        return cur;
    }
    let a = uc
        .get_data_mut()
        .heap
        .alloc(0x20, "MemoryBlock", false)
        .unwrap_or(0);
    if a != 0 {
        fill(uc, a, 0, 0x20);
    }
    uc.get_data_mut().rt.gblock = a;
    a
}

pub fn get_gblock(uc: &mut Emu) {
    let p = gblock_ptr(uc);
    uc.ret(p);
}

pub fn init_gblock(uc: &mut Emu) {
    let size = uc.arg(0);
    let blk = gblock_ptr(uc);
    if blk != 0 && (size as i32) > 0 {
        let old = uc.r32(blk);
        if old != 0 {
            uc.get_data_mut().heap.free(old);
        }
        crate::api::sysmisc::setup_memory_block(uc, blk, size);
    }
    uc.ret(0);
}

pub fn release_gblock(uc: &mut Emu) {
    let blk = uc.get_data().rt.gblock;
    if blk != 0 && (uc.r32(blk + 8) as i32) > 0 {
        let base = uc.r32(blk);
        if base != 0 {
            uc.get_data_mut().heap.free(base);
        }
        uc.w32(blk + 8, 0);
    }
    uc.ret(0);
}

pub fn reset_gblock(uc: &mut Emu) {
    let blk = uc.get_data().rt.gblock;
    if blk != 0 && (uc.r32(blk + 8) as i32) > 0 {
        uc.w32(blk + 4, 0);
    }
    uc.ret(0);
}

pub fn gblock_malloc(uc: &mut Emu) {
    let size = uc.arg(0);
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "gblock", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.ret(p);
}

pub fn noop(uc: &mut Emu) {
    uc.ret(0);
}
