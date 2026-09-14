
use crate::api::fileio::wwrite;
use crate::api::storage;
use crate::machine::{Emu, Mach};
use crate::runtime;

const APS_TAG: &str = "VmDlAppStoreManagerTag";
const APS_PATH: &str = ".system/AS_MSTAR_WQVGA";

pub fn im_sms(uc: &mut Emu) {
    let d = uc.arg(0);
    if d != 0 {
        let cb = uc.r32(d + 16);
        runtime::defer(uc, cb, vec![0], "smsResult");
        uc.write(d + 20, &[1]);
    }
    uc.ret(1);
}

pub fn im_nv_read(uc: &mut Emu) {
    let d = uc.arg(0);
    let r = if d != 0 {
        let (off, buf, n, okp) = (uc.r16(d) as u32, uc.r32(d + 4), uc.r16(d + 8) as u32, uc.r32(d + 12));
        storage::nv_read_at(uc, off, buf, n, okp)
    } else {
        0
    };
    uc.ret(r);
}

pub fn im_nv_write(uc: &mut Emu) {
    let d = uc.arg(0);
    let r = if d != 0 {
        let (off, buf, n, okp) = (uc.r16(d) as u32, uc.r32(d + 4), uc.r16(d + 8) as u32, uc.r32(d + 12));
        storage::nv_write_at(uc, off, buf, n, okp)
    } else {
        0
    };
    uc.ret(r);
}

pub fn im_get_aps_manager(uc: &mut Emu) {
    let d = uc.arg(0);
    let dst = if d != 0 { uc.r32(d) } else { 0 };
    if dst != 0 {
        let n = (uc.r16(d + 4) as usize).min(156);
        let src = runtime::manager_by_tag(uc, APS_TAG);
        if let Ok(v) = uc.mem_read_as_vec(src as u64, n) {
            uc.write(dst, &v);
        }
    }
    uc.ret(0);
}

pub fn aps_path(uc: &mut Emu) {
    let cached = uc.get_data().rt.host_bufs.get("aps_path").copied();
    let p = match cached {
        Some(p) => p,
        None => {
            let n = (APS_PATH.len() as u32 + 1) * 2;
            let p = uc.get_data_mut().heap.alloc(n, "aps_path", false).unwrap_or(0);
            if p != 0 {
                wwrite(uc, p, APS_PATH);
            }
            uc.get_data_mut().rt.host_bufs.insert("aps_path", p);
            p
        }
    };
    uc.ret(p);
}

pub fn aps_get_dev(uc: &mut Emu) {
    let v = uc.get_data().rt.aps_dev;
    uc.ret(v);
}

pub fn aps_set_dev(uc: &mut Emu) {
    let v = uc.arg(0) & 0xFF;
    uc.get_data_mut().rt.aps_dev = v;
    uc.ret(0);
}

pub fn aps_app_num(uc: &mut Emu) {
    let out = uc.arg(1);
    if out != 0 {
        uc.write(out, &[0, 0]);
    }
    uc.ret(1);
}

pub fn aps_support_type(uc: &mut Emu) {
    uc.ret(2);
}

pub fn aps_security_code(uc: &mut Emu) {
    let (out, n) = (uc.arg(0), uc.arg(1));
    if out == 0 || n < 0x2B {
        uc.ret(0);
        return;
    }
    let st = uc.get_data().rt.rand_state;
    let st = (1103515245u64.wrapping_mul(st as u64).wrapping_add(12345) & 0x7FFF_FFFF) as u32;
    uc.get_data_mut().rt.rand_state = st;
    let valid = |r: &[u8]| r[0] == 0xE9 && r[1..].iter().take_while(|&&c| c != 0).count() >= 8;
    let rec = match storage::nv_record(uc, 1897, 45) {
        Some(r) if valid(&r) => r,
        _ => {
            let imei = &crate::api::misc3::IMEI.as_bytes()[..crate::api::misc3::IMEI.len().min(16)];
            let mut r = vec![0u8; 45];
            r[0] = 0xE9;
            r[1..9].copy_from_slice(format!("{st:08X}").as_bytes());
            r[9..43].fill(b'1');
            for at in [9usize, 26] {
                r[at..at + imei.len()].copy_from_slice(imei);
                r[at + imei.len()] = 0;
            }
            let tmp = match uc.get_data().rt.host_bufs.get("nvtmp").copied() {
                Some(t) => t,
                None => {
                    let t = uc.get_data_mut().heap.alloc(45, "nvtmp", false).unwrap_or(0);
                    uc.get_data_mut().rt.host_bufs.insert("nvtmp", t);
                    t
                }
            };
            if tmp != 0 {
                uc.write(tmp, &r);
                storage::nv_write_at(uc, 1897, tmp, 45, 0);
            }
            r
        }
    };
    uc.write(out, &rec[1..43]);
    uc.ret(1);
}
