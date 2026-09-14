
use crate::api::fill;
use crate::machine::{Emu, Mach};
use crate::runtime;

pub fn total_seconds(uc: &mut Emu) {
    let t = uc.get_data().rt.tick / 1000;
    uc.ret(t as u32);
}

pub fn billing_paynum(uc: &mut Emu) {
    uc.ret(0);
}

pub fn billing_remain_day(uc: &mut Emu) {
    let r = if uc.arg(1) == 3 { 1 } else { 0 };
    uc.ret(r);
}

pub fn billing_send_sms(uc: &mut Emu) {
    if uc.arg(0) != 14 {
        uc.ret(0);
        return;
    }
    let cb = uc.arg(6);
    runtime::defer(uc, cb, vec![0], "smsResult");
    uc.ret(1);
}

pub fn billing_pay(uc: &mut Emu) {
    let cb = uc.arg(2);
    runtime::defer(uc, cb, vec![1], "payResult");
    uc.ret(0);
}

pub fn billing_pay_cbb(uc: &mut Emu) {
    let cb = uc.arg(3);
    runtime::defer(uc, cb, vec![1], "payResult");
    uc.ret(0);
}

pub fn billing_pay_pwd(uc: &mut Emu) {
    let cb = uc.arg(0);
    runtime::defer(uc, cb, vec![1], "payResult");
    uc.ret(0);
}

fn bill_reg(uc: &Emu) -> Option<(u8, u8)> {
    let id = uc.arg(0) as u16;
    uc.get_data().rt.billing_reg.get(&id).copied()
}

pub fn billing_is_registered(uc: &mut Emu) {
    let r = matches!(bill_reg(uc), Some((_, u)) if u != 0);
    uc.ret(u32::from(r));
}

pub fn billing_register_info(uc: &mut Emu) {
    let id = uc.arg(0) as u16;
    uc.get_data_mut().rt.billing_reg.insert(id, (0, 1));
    uc.ret(1);
}

pub fn billing_set_status(uc: &mut Emu) {
    let (id, v) = (uc.arg(0) as u16, uc.arg(1) as u8);
    let r = match uc.get_data_mut().rt.billing_reg.get_mut(&id) {
        Some(e) => {
            e.0 = v;
            1
        }
        None => 0,
    };
    uc.ret(r);
}

pub fn billing_get_status(uc: &mut Emu) {
    let r = bill_reg(uc).map_or(1, |e| u32::from(e.0));
    uc.ret(r);
}

pub fn billing_get_used(uc: &mut Emu) {
    let r = bill_reg(uc).map_or(0, |e| u32::from(e.1));
    uc.ret(r);
}

pub fn billing_set_used(uc: &mut Emu) {
    let (id, v) = (uc.arg(0) as u16, uc.arg(1) as u8);
    if let Some(e) = uc.get_data_mut().rt.billing_reg.get_mut(&id) {
        e.1 = v;
    }
    uc.ret(0);
}

pub fn billing_clean_month(uc: &mut Emu) {
    let id = uc.arg(0) as u16;
    uc.get_data_mut().rt.billing_reg.remove(&id);
    uc.ret(1);
}

pub fn billing_pay_times(uc: &mut Emu) {
    uc.ret(2);
}

pub fn billing_valid_day(uc: &mut Emu) {
    uc.ret(30);
}

pub fn billing_file_name(uc: &mut Emu) {
    let p = uc.arg(1);
    if p != 0 {
        uc.w32(p, 0);
    }
    uc.ret(0);
}

pub fn billing_cancel_sms(uc: &mut Emu) {
    uc.get_data_mut().rt.pending.retain(|p| p.2 != "smsResult");
    uc.ret(1);
}

pub fn screen_type(uc: &mut Emu) {
    uc.ret(5);
}

pub fn main_screen_image(uc: &mut Emu) {
    let img = uc.get_data().rt.gfx.img;
    uc.ret(img);
}

pub fn kernel_ver(uc: &mut Emu) {
    uc.ret(42);
}

pub fn zero(uc: &mut Emu) {
    uc.ret(0);
}

pub fn screen_w(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.w;
    uc.ret(v);
}

pub fn screen_h(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.h;
    uc.ret(v);
}

pub fn screen_change(uc: &mut Emu) {
    let scr = uc.arg(0);
    uc.get_data_mut().rt.screens.clear();
    runtime::push_screen(uc, scr, 0, 1);
    uc.ret(0);
}

pub fn df_set_pkg(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.get_data_mut().rt.datapackage = p;
    uc.ret(0);
}

pub fn init_memory_block(uc: &mut Emu) {
    let mut blk = uc.arg(0);
    let size = uc.arg(1);
    if blk == 0 {
        blk = uc
            .get_data_mut()
            .heap
            .alloc(0x18, "MEMORY_BLOCK", false)
            .unwrap_or(0);
        if blk == 0 {
            uc.ret(0);
            return;
        }
    }
    let base = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "memblock", false)
            .unwrap_or(0)
    } else {
        0
    };
    if base != 0 {
        fill(uc, base, 0, size);
    }
    uc.w32(blk, base);
    uc.w32(blk + 0x04, 0);
    uc.w32(blk + 0x08, size);
    runtime::install(uc, blk + 0x0c, "MB_Malloc", mb_malloc);
    runtime::install(uc, blk + 0x10, "MB_Reset", mb_reset);
    runtime::install(uc, blk + 0x14, "MB_Release", mb_release);
    uc.ret(blk);
}

fn mb_malloc(uc: &mut Emu) {
    let blk = uc.arg(0);
    let n = uc.arg(1);
    if blk == 0 {
        uc.ret(0);
        return;
    }
    let base = uc.r32(blk);
    let ptr = uc.r32(blk + 4);
    let total = uc.r32(blk + 8);
    let size = (n + 3) & !3;
    if ptr + size > total {
        uc.ret(0);
        return;
    }
    uc.w32(blk + 4, ptr + size);
    crate::api::fill(uc, base + ptr, 0, size);
    uc.ret(base + ptr);
}

fn mb_reset(uc: &mut Emu) {
    let blk = uc.arg(0);
    if blk != 0 {
        uc.w32(blk + 4, 0);
    }
    uc.ret(0);
}

fn mb_release(uc: &mut Emu) {
    uc.ret(0);
}

pub fn enter_win_close(uc: &mut Emu) {
    uc.get_data_mut().rt.exit_requested = true;
    uc.ret(0);
}
