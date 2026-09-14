
use std::collections::HashMap;

use crate::machine::{self, Emu, Mach};
use crate::vmspec;

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub id: u32,
    pub due: u64,
    pub cb: u32,
    pub param: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {

    pub aw: u32,

    pub hw: u32,

    pub hh: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    New,
    Old,
}

pub struct Rt {
    pub style: Style,

    pub gamelib_v3: bool,
    pub host: u32,
    pub sys_tbl: u32,
    pub game_tbl: u32,

    pub managers: HashMap<u32, u32>,
    pub tag_managers: HashMap<&'static str, u32>,

    pub aps_dev: u32,

    pub billing_reg: HashMap<u16, (u8, u8)>,

    pub host_bufs: HashMap<&'static str, u32>,

    pub init_map: HashMap<u32, u32>,
    pub mod_cb0: u32,
    pub mod_cb1: u32,
    pub screen_w: u32,
    pub screen_h: u32,

    pub unimpl: HashMap<(String, u32), u64>,

    pub old_objs: HashMap<i64, u32>,

    pub old_calls: HashMap<u32, u64>,

    pub gblock: u32,

    pub rand_state: u32,

    pub slots_impl: u32,
    pub slots_stub: u32,

    pub gfx: crate::gfx::Gfx,

    pub pending: Vec<(u32, Vec<u32>, &'static str)>,

    pub screens: Vec<(u32, u32, u32)>,
    pub keys_down: u32,
    pub keys_up: u32,
    pub keys_hold: u32,

    pub pointer: (i32, i32),

    pub touch_down: u32,
    pub touch_up: u32,
    pub touch_hold: u32,
    pub touch_drag: u32,

    pub exit_requested: bool,
    pub tick: u64,

    pub frame_ms: u64,
    pub frames: u64,

    pub installed: HashMap<(u32, &'static str), u32>,

    pub methods: HashMap<&'static str, u32>,

    pub co_frames: Vec<(CoK, u32, u32)>,
    pub co_trap: u32,

    pub oldlib_clip: [i32; 4],

    pub piclib_tmpimg: u32,

    pub datapackage: u32,

    pub dfblock: u32,

    pub module_name: String,

    pub vfs: crate::vfs::Vfs,

    pub finds: HashMap<u32, Vec<String>>,
    pub next_find: u32,

    pub emptystr: u32,

    pub fallback_pkg: u32,

    pub timers: Vec<Timer>,
    pub next_timer: u32,

    pub nethandle: u32,

    pub audio: crate::audio::Audio,

    pub font: FontMetrics,

    pub font_data: Option<crate::font::Font>,

    pub res: Option<cbelib::ResArchive>,
    pub icons: Option<cbelib::ResArchive>,
    pub packages: Vec<(String, cbelib::ResArchive)>,

    pub dp_cache: HashMap<String, (u32, u32, u32, u32)>,

    pub pkg_entries: Vec<(u32, Vec<(String, u32)>)>,

    pub pkg_wanted: Vec<(u32, Vec<String>)>,
    pub logs: Vec<String>,
}

impl Default for Rt {
    fn default() -> Self {
        Self::new()
    }
}

impl Rt {
    pub fn new() -> Self {
        Rt {
            style: Style::New,
            host: 0,
            sys_tbl: 0,
            game_tbl: 0,
            managers: HashMap::new(),
            tag_managers: HashMap::new(),
            aps_dev: 0,
            billing_reg: HashMap::new(),
            gamelib_v3: false,
            host_bufs: HashMap::new(),
            init_map: HashMap::new(),
            mod_cb0: 0,
            mod_cb1: 0,
            screen_w: 240,
            screen_h: 400,
            unimpl: HashMap::new(),
            old_objs: HashMap::new(),
            old_calls: HashMap::new(),
            gblock: 0,
            rand_state: 0x12345678,
            slots_impl: 0,
            slots_stub: 0,
            gfx: crate::gfx::Gfx::default(),
            pending: Vec::new(),
            screens: Vec::new(),
            keys_down: 0,
            keys_up: 0,
            keys_hold: 0,
            pointer: (0, 0),
            touch_down: 0,
            touch_up: 0,
            touch_hold: 0,
            touch_drag: 0,
            exit_requested: false,
            tick: 0,
            frame_ms: 40,
            frames: 0,
            installed: HashMap::new(),
            methods: HashMap::new(),
            co_frames: Vec::new(),
            co_trap: 0,
            oldlib_clip: [0; 4],
            piclib_tmpimg: 0,
            datapackage: 0,
            dfblock: 0,
            module_name: String::new(),
            vfs: crate::vfs::Vfs::new(std::path::PathBuf::from("."), None),
            finds: HashMap::new(),
            next_find: 1,
            emptystr: 0,
            fallback_pkg: 0,
            timers: Vec::new(),
            next_timer: 1,
            nethandle: 0,
            audio: crate::audio::Audio { volume: 5, ..Default::default() },
            font: FontMetrics { aw: 6, hw: 12, hh: 12 },
            font_data: None,
            res: None,
            icons: None,
            packages: Vec::new(),
            dp_cache: HashMap::new(),
            pkg_entries: Vec::new(),
            pkg_wanted: Vec::new(),
            logs: Vec::new(),
        }
    }
}

pub fn entry_style(m: &cbelib::CbeModule) -> Style {
    use capstone::arch::arm::ArmOperandType;
    use capstone::arch::BuildsCapstone;
    use capstone::prelude::*;

    let mut b = Capstone::new().arm().mode(arch::arm::ArchMode::Thumb);
    if m.endian == cbelib::Endian::Be {
        b = b.endian(capstone::Endian::Big);
    }
    let cs = match b.detail(true).build() {
        Ok(c) => c,
        Err(_) => return Style::New,
    };
    let n = m.ro.len().min(0x40);
    let insns = match cs.disasm_all(&m.ro[..n], 0) {
        Ok(i) => i,
        Err(_) => return Style::New,
    };

    let mut argregs: Vec<RegId> = vec![RegId(arch::arm::ArmReg::ARM_REG_R0 as u16)];
    for i in insns.iter() {
        let mn = i.mnemonic().unwrap_or("");
        if mn == "bl" {
            break;
        }
        let Ok(det) = cs.insn_detail(i) else { continue };
        let ops: Vec<_> = match det.arch_detail().arm() {
            Some(a) => a.operands().collect(),
            None => continue,
        };
        let reg_of = |k: usize| -> Option<RegId> {
            match ops.get(k).map(|o| &o.op_type) {
                Some(ArmOperandType::Reg(r)) => Some(*r),
                _ => None,
            }
        };
        if mn.starts_with("str") {
            if ops.len() == 2 {
                if let Some(ArmOperandType::Mem(mem)) = ops.get(1).map(|o| &o.op_type) {
                    if argregs.contains(&mem.base()) {
                        return Style::New;
                    }
                    if let Some(r) = reg_of(0) {
                        if argregs.contains(&r) {
                            return Style::Old;
                        }
                    }
                }
            }
            continue;
        }

        let movish = matches!(mn, "mov" | "movs" | "add" | "adds");
        if movish && ops.len() >= 2 {
            if let (Some(d), Some(sr)) = (reg_of(0), reg_of(1)) {
                let plain_move = ops.len() == 2
                    || matches!(ops.get(2).map(|o| &o.op_type), Some(ArmOperandType::Imm(0)));
                if plain_move && argregs.contains(&sr) {
                    if !argregs.contains(&d) {
                        argregs.push(d);
                    }
                    continue;
                }
            }
        }
        mark_written(&cs, i, mn, reg_of(0), &mut argregs);
    }
    Style::New
}

fn mark_written(
    cs: &capstone::Capstone,
    i: &capstone::Insn,
    mn: &str,
    dst: Option<capstone::RegId>,
    argregs: &mut Vec<capstone::RegId>,
) {

    let no_dst = mn.starts_with("str")
        || mn.starts_with("cmp")
        || mn.starts_with("cmn")
        || mn.starts_with("tst")
        || mn.starts_with("teq")
        || mn.starts_with("push")
        || mn.starts_with('b');
    if !no_dst {
        if let Some(d) = dst {
            argregs.retain(|&x| x != d);
        }
    }
    if let Ok(det) = cs.insn_detail(i) {
        for r in det.regs_write() {
            argregs.retain(|&x| x != *r);
        }
    }
}

fn h_getter(uc: &mut Emu) {
    let off = slot_off(uc);
    let a = get_manager(uc, off);
    uc.ret(a);
}

fn h_initer(uc: &mut Emu) {
    let init_off = slot_off(uc);
    let get_off = match uc.get_data().rt.init_map.get(&init_off) {
        Some(&g) => g,
        None => {
            uc.ret(0);
            return;
        }
    };
    let dst = uc.arg(0);
    let src = get_manager(uc, get_off);
    let n = mgr_size(get_off);
    if dst != 0 {
        if let Ok(buf) = uc.mem_read_as_vec(src as u64, n as usize) {
            uc.write(dst, &buf);
        }
    }
    uc.ret(0);
}

fn h_zero(uc: &mut Emu) {
    uc.ret(0);
}

fn h_vm_log(uc: &mut Emu) {
    let s = crate::api::printf::vm_printf(uc, uc.arg(0), 1);
    let n = s
        .iter()
        .rposition(|c| !c.is_ascii_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let msg = String::from_utf8_lossy(&s[..n]).to_string();
    uc.get_data_mut().rt.logs.push(msg);
    uc.ret(n as u32);
}

fn h_unimpl(uc: &mut Emu) {
    let (tag, off) = {
        let d = uc.get_data();
        let s = &d.slots[d.cur_slot as usize];
        (s.tag.unwrap_or("?").to_string(), s.off)
    };
    *uc.get_data_mut().rt.unimpl.entry((tag, off)).or_insert(0) += 1;
    uc.ret(0);
}

fn slot_off(uc: &Emu) -> u32 {
    let d = uc.get_data();
    d.slots[d.cur_slot as usize].off
}

fn mgr_size(get_off: u32) -> u32 {
    let tag = vmspec::SYS
        .iter()
        .find(|(o, _, _)| *o == get_off)
        .and_then(|(_, _, t)| *t);
    match tag.and_then(vmspec::mgr) {
        Some(m) if m.size != 0 => m.size,
        _ => 0x400,
    }
}

pub fn get_manager(uc: &mut Emu, sysoff: u32) -> u32 {
    if let Some(&a) = uc.get_data().rt.managers.get(&sysoff) {
        return a;
    }

    if sysoff == OLD_GAMEMGR_SYSOFF && uc.get_data().rt.style == Style::Old {
        return old_game_manager(uc);
    }
    let ent = vmspec::SYS.iter().find(|(o, _, _)| *o == sysoff);
    let tag: Option<&'static str> = ent.and_then(|(_, _, t)| *t);
    let addr = build_manager(uc, tag);
    uc.get_data_mut().rt.managers.insert(sysoff, addr);
    addr
}

pub fn manager_by_tag(uc: &mut Emu, tag: &'static str) -> u32 {
    if let Some(&a) = uc.get_data().rt.tag_managers.get(tag) {
        return a;
    }
    let addr = build_manager(uc, Some(tag));
    uc.get_data_mut().rt.tag_managers.insert(tag, addr);
    addr
}

fn build_manager(uc: &mut Emu, tag: Option<&'static str>) -> u32 {
    let size = match tag.and_then(vmspec::mgr) {
        Some(m) if m.size != 0 => m.size + 0x100,
        _ => 0x400 + 0x100,
    };
    let name = tag.unwrap_or("mgr");
    let addr = uc
        .get_data_mut()
        .data
        .alloc(size, name, true)
        .expect("宿主结构区耗尽");

    let mut off = 0u32;
    while off < size {
        let fname = tag
            .and_then(|t| vmspec::field(t, off))
            .map(|f| f.name.to_string())
            .unwrap_or_else(|| {
                let base = tag.map(|t| &t[..t.len().saturating_sub(3)]).unwrap_or("mgr");
                format!("{base}+{off:#x}")
            });

        if std::env::var("NO_NATIVE").is_err() {
        if let Some(v) = native_const_value(uc, &fname) {
            let p = machine::native_const(uc, v);
            uc.w32(addr + off, p);
            uc.get_data_mut().rt.slots_impl += 1;
            off += 4;
            continue;
        }
        }
        let real = tag.and_then(|t| crate::api::lookup(t, &fname));
        {
            let d = uc.get_data_mut();
            if real.is_some() {
                d.rt.slots_impl += 1;
            } else {
                d.rt.slots_stub += 1;
            }
        }
        let h = real.unwrap_or(h_unimpl as machine::ApiFn);
        let p = machine::new_trap_tagged(uc, &fname, Some(h), tag, off);
        uc.w32(addr + off, p);
        off += 4;
    }
    addr
}

fn native_const_value(uc: &Emu, name: &str) -> Option<u32> {
    let g = &uc.get_data().rt.gfx;
    match name {
        "VMGetCurrMainScreenImage" => Some(g.img),
        "VMGetLCDBuffer" => Some(g.buf),
        "VmGetScreenWidth" | "GetScreenWidth" => Some(g.w),
        "VmGetScreenHeight" | "GetScreenHeight" => Some(g.h),
        _ => None,
    }
}

pub const OLD_GAMEMGR_SYSOFF: u32 = 0x090;
pub const OLD_GAMEMGR_SHIFT: u32 = 0x1c;

fn old_game_manager(uc: &mut Emu) -> u32 {
    let base = get_manager(uc, 0x084);
    let size = match vmspec::mgr("GameManagerOldTag") {
        Some(m) if m.size != 0 => m.size + 0x100,
        _ => 0x400 + 0x100,
    };
    let sh = OLD_GAMEMGR_SHIFT;
    let addr = uc
        .get_data_mut()
        .data
        .alloc(size + sh, "GameManagerOld@old", true)
        .expect("宿主结构区耗尽");
    let mut off = 0u32;
    while off < size {
        let v = uc.r32(base + off);
        uc.w32(addr + sh + off, v);
        off += 4;
    }
    uc.get_data_mut()
        .rt
        .managers
        .insert(OLD_GAMEMGR_SYSOFF, addr);
    addr
}

pub fn setup(uc: &mut Emu, m: &cbelib::CbeModule) {

    let mut init_map: HashMap<u32, u32> = HashMap::new();
    if let Some(vm) = vmspec::mgr("VmManagerTag") {
        for f in vm.fields {
            let low = f.name.to_lowercase();
            if !low.contains("init") {
                continue;
            }
            let key = low.replacen("vminit", "", 1).replacen("init", "", 1);
            for (goff, gnm, _) in vmspec::SYS {
                let gl = gnm.to_lowercase();
                let gkey = gl.replacen("vmget", "", 1).replacen("get", "", 1);
                if gkey == key {
                    init_map.insert(f.off, *goff);
                    break;
                }
            }
        }
    }
    uc.get_data_mut().rt.init_map = init_map;

    let vmsize = vmspec::mgr("VmManagerTag").map(|m| m.size).unwrap_or(0xd4);
    let sys = uc
        .get_data_mut()
        .data
        .alloc(vmsize, "VmManager", true)
        .expect("宿主结构区耗尽");
    uc.get_data_mut().rt.sys_tbl = sys;

    let mut off = 0u32;
    while off < vmsize {
        let name = vmspec::field("VmManagerTag", off)
            .map(|f| f.name.to_string())
            .unwrap_or_else(|| format!("VmManager+{off:#x}"));
        let is_get = vmspec::SYS.iter().any(|(o, _, _)| *o == off);
        let is_init = uc.get_data().rt.init_map.contains_key(&off);
        let h = if is_get {
            h_getter
        } else if is_init {
            h_initer
        } else {
            h_zero
        };
        let p = machine::new_trap_tagged(uc, &name, Some(h), Some("VmManagerTag"), off);
        uc.w32(sys + off, p);
        off += 4;
    }

    let host = uc
        .get_data_mut()
        .data
        .alloc(0x10, "VmSysCallRegParam", true)
        .expect("宿主结构区耗尽");
    uc.get_data_mut().rt.host = host;
    uc.w32(host + 0x08, sys);

    let tr = machine::new_trap(uc, "gm_TRACE", Some(h_vm_log));
    uc.w32(host + 0x0c, tr);

    uc.get_data_mut().rt.style = entry_style(m);
    uc.get_data_mut().rt.screen_w = 240;
    uc.get_data_mut().rt.screen_h = 400;
    crate::gfx::init_fb(uc, 240, 400);
    {

        let home = std::env::var("NIECHE_HOME")
            .or_else(|_| std::env::var("NICAI_HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| user_home().join(".nieche-emu"));
        let safe: String = m
            .name
            .chars()
            .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
            .collect();
        let root = home.join("fs").join(if safe.trim().is_empty() {
            "unnamed".to_string()
        } else {
            safe.trim().to_string()
        });

        let base = std::env::var("NIECHE_FSBASE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("assets/fatfs"));
        uc.get_data_mut().rt.vfs = crate::vfs::Vfs::new(root, Some(base));
    }
    if let Some(f) = crate::font::Font::load() {
        let d = &mut uc.get_data_mut().rt;
        d.font = FontMetrics { aw: f.aw, hw: f.hw, hh: f.hh };
        d.font_data = Some(f);
    }
    {
        let d = &mut uc.get_data_mut().rt;
        d.res = m.res.clone();
        d.icons = m.icons.clone();
        d.packages = m.packages.clone();
        d.module_name = m.name.clone();

        d.audio.outdir = Some(std::path::PathBuf::from(format!("out/{}/audio", m.name)));
    }
}

pub const OLD_REGISTER_APP: u32 = 1950;

pub const OLD_FETCH: u32 = 2001;

pub const OLD_MEM_SID: u32 = 143;

fn h_old_syscall(uc: &mut Emu) {
    let sid = uc.arg(0);
    *uc.get_data_mut().rt.old_calls.entry(sid).or_insert(0) += 1;

    if sid == OLD_REGISTER_APP {
        let blk = uc.arg(1);
        if blk != 0 {
            let cb0 = uc.r32(blk);
            let cb1 = uc.r32(blk + 4);
            uc.get_data_mut().rt.mod_cb0 = cb0;
            uc.get_data_mut().rt.mod_cb1 = cb1;

            let h = old_helper(uc);
            uc.w32(blk + 8, h);
        }

        let o = old_obj(uc, sid as i64);
        uc.ret(o);
        return;
    }
    if sid == OLD_FETCH {
        old_fetch(uc);
        return;
    }
    if sid == OLD_GAMELIB {
        let t = old_gamelib(uc);
        uc.ret(t);
        return;
    }
    if sid == OLD_GAMELIB_COPY {
        let buf = uc.arg(1);
        if buf != 0 {
            let t = old_gamelib(uc);
            if let Ok(v) = uc.mem_read_as_vec(t as u64, OLD_GAMELIB_COPY_SIZE as usize) {
                uc.write(buf, &v);
            }
        }
        uc.ret(0);
        return;
    }
    if sid == 185 {

        let frame = uc.arg(1);
        if frame != 0 {
            let (pp, n) = (uc.r32(frame), uc.r32(frame + 4));
            uc.setreg(0, n);
            crate::api::mem::gblock_malloc(uc);
            let p = uc.reg(0);
            if pp != 0 {
                uc.w32(pp, p);
            }
            uc.write(frame + 8, &[u8::from(p != 0)]);
        }
        uc.ret(0);
        return;
    }
    if sid == 190 {

        let frame = uc.arg(1);
        let pp = if frame != 0 { uc.r32(frame) } else { 0 };
        if pp != 0 {
            uc.w32(pp, 0);
        }
        uc.ret(0);
        return;
    }
    if OLD_NET.contains(&sid) {
        old_net(uc, sid);
        return;
    }
    if let Some(&(_, tag, name, spec)) = OLD_FORWARD.iter().find(|e| e.0 == sid) {
        old_forward(uc, tag, name, spec);
        return;
    }

    let obj = old_obj(uc, sid as i64);
    let frame = uc.arg(1);
    if frame != 0 {
        uc.w32(frame + 8, obj);
    }
    uc.ret(obj);
}

const OLD_FORWARD: &[(u32, &str, &str, &str)] = &[
    (183, "VmMemoryManagerTag", "dF_InitMemory", "I"),
    (142, "VmMemoryManagerTag", "mF_GetGMemoryBlockPtr", ""),
    (103, "VmMemoryManagerTag", "mF_InitMemoryBlock", "II"),
    (184, "VmMemoryManagerTag", "dF_ReleaseMemory", ""),
    (4, "VmSysManagerTag", "VmEnterWinClose", ""),
    (61, "GameManagerOldTag", "Storage_Date", "IIIBB"),
    (156, "VmDFEnginelManagerTag", "DF_SendMessage", "IHI"),
    (1050, "VmIoManagerTag", "Vm_file_open", "III"),
    (1051, "VmIoManagerTag", "Vm_file_write", "III"),
    (1052, "VmIoManagerTag", "Vm_file_close", "I"),
    (1063, "VmIoManagerTag", "Vm_file_read", "III"),
    (1066, "VmIoManagerTag", "Vm_file_getfilesize", "I"),
];

fn old_unpack(uc: &Emu, frame: u32, spec: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut off = 0u32;
    for c in spec.chars() {
        let w = match c {
            'I' => 4,
            'H' => 2,
            _ => 1,
        };
        off = off.div_ceil(w) * w;
        let v = if frame == 0 {
            0
        } else {
            match c {
                'I' => uc.r32(frame + off),
                'H' => uc.r16(frame + off) as i16 as i32 as u32,
                _ => uc.r8(frame + off) as u32,
            }
        };
        out.push(v);
        off += w;
    }
    out
}

fn old_forward(uc: &mut Emu, tag: &str, name: &str, spec: &str) {
    let frame = uc.arg(1);
    let args = old_unpack(uc, frame, spec);
    for (i, v) in args.iter().take(4).enumerate() {
        uc.setreg(i as u32, *v);
    }
    let sp = uc.reg_read(unicorn_engine::RegisterARM::SP).unwrap_or(0) as u32;
    let extra: Vec<u32> = args.iter().skip(4).copied().collect();
    let saved = if extra.is_empty() {
        Vec::new()
    } else {
        uc.mem_read_as_vec(sp as u64, 4 * extra.len()).unwrap_or_default()
    };
    for (i, v) in extra.iter().enumerate() {
        uc.w32(sp + 4 * i as u32, *v);
    }
    match crate::api::lookup(tag, name) {
        Some(f) => f(uc),
        None => uc.ret(0),
    }
    if !extra.is_empty() {
        uc.write(sp, &saved);
    }
}

const OLD_NET: &[u32] = &[1004, 1005, 1006, 1030, 1031, 1057];

fn old_net(uc: &mut Emu, sid: u32) {
    let frame = uc.arg(1);
    match sid {
        1004 if frame != 0 => {
            let (url, cb) = (uc.r32(frame), uc.r32(frame + 12));
            uc.setreg(0, url);
            uc.setreg(1, cb);
            uc.setreg(2, 0);
            crate::api::misc3::get_http(uc);
            uc.ret(1);
        }
        1030 | 1006 | 1031 => uc.ret(1),
        _ => uc.ret(0),
    }
}

pub const OLD_GAMELIB: u32 = 143;
pub const OLD_GAMELIB_COPY: u32 = 82;
pub const OLD_GAMELIB_COPY_SIZE: u32 = 0x26c;
pub const OLD_GAMELIB_GAP: (u32, u32) = (0x114, 8);

pub const OLD_SMS_DIFF: u32 = 3764;
pub const OLD_SMS_ADD: u32 = 1376;

fn old_gamelib(uc: &mut Emu) -> u32 {
    const KEY: i64 = -3;
    if let Some(&a) = uc.get_data().rt.old_objs.get(&KEY) {
        return a;
    }

    uc.get_data_mut().rt.gamelib_v3 = true;
    uc.get_data_mut().rt.gfx.wide = true;
    crate::gfx::write_fb_header(uc);
    let base = get_manager(uc, 0x084);
    let size = match vmspec::mgr("GameManagerOldTag") {
        Some(m) if m.size != 0 => m.size + 0x100,
        _ => 0x400 + 0x100,
    };
    let addr = uc
        .get_data_mut()
        .data
        .alloc(size, "GameManagerOld@v3", true)
        .expect("宿主结构区耗尽");
    let (at, gap) = OLD_GAMELIB_GAP;
    let mut off = 0u32;
    while off + gap < size {
        let src = if off < at { off } else { off + gap };
        let v = uc.r32(base + src);
        uc.w32(addr + off, v);
        off += 4;
    }
    let pay = machine::new_trap(uc, "BILLING_GetPayNumByAppId@v3", Some(crate::api::sysmisc::billing_paynum));
    reserve_traps(uc, OLD_SMS_DIFF / 4 - 1);
    let remain = machine::new_trap(uc, "BILLING_GetRemainDay@v3", Some(crate::api::sysmisc::billing_remain_day));
    reserve_traps(uc, OLD_SMS_ADD / 4 - 1);
    machine::new_trap(uc, "vMSendSms@v3", Some(h_old_send_sms));
    uc.w32(addr + 0x240, pay);
    uc.w32(addr + 0x244, remain);
    uc.get_data_mut().rt.old_objs.insert(KEY, addr);
    addr
}

fn reserve_traps(uc: &mut Emu, n: u32) {
    for _ in 0..n {
        machine::new_trap(uc, "", None);
    }
}

fn h_old_send_sms(uc: &mut Emu) {
    let sp = uc.reg_read(unicorn_engine::RegisterARM::SP).unwrap_or(0) as u32;
    let cb = uc.r32(sp + 4);
    defer(uc, cb, vec![1], "smsResult");
    uc.ret(1);
}

fn old_fetch(uc: &mut Emu) {
    let desc = uc.arg(1);
    if desc == 0 {
        uc.ret(0);
        return;
    }
    let ptr = uc.r32(desc);
    let handle = uc.r32(desc + 4);
    let ln = uc.r32(desc + 8);

    if ptr != 0 && ln >= 4 {
        uc.w32(ptr, handle);
    } else if ptr != 0 && ln == 2 {
        uc.w16(ptr, handle as u16);
    } else if ptr != 0 && ln == 1 {
        uc.write(ptr, &[handle as u8]);
    }
    uc.ret(1);
}

fn old_helper(uc: &mut Emu) -> u32 {
    const HELPER: i64 = -1;
    if let Some(&a) = uc.get_data().rt.old_objs.get(&HELPER) {
        return a;
    }
    let t = machine::new_trap(uc, "oldsys_helper", Some(h_old_helper));
    uc.get_data_mut().rt.old_objs.insert(HELPER, t);
    t
}

fn h_old_helper(uc: &mut Emu) {
    const HELPER_OBJ: i64 = -2;
    let o = old_obj(uc, HELPER_OBJ);
    uc.ret(o);
}

fn old_obj(uc: &mut Emu, sid: i64) -> u32 {
    if let Some(&a) = uc.get_data().rt.old_objs.get(&sid) {
        return a;
    }
    let a = if sid == OLD_MEM_SID as i64 {
        old_mem_table(uc)
    } else {
        machine::new_table(uc, &format!("oldsys#{sid}"), 256)
    };
    uc.get_data_mut().rt.old_objs.insert(sid, a);
    a
}

fn old_mem_table(uc: &mut Emu) -> u32 {
    let addr = machine::new_table(uc, "oldsys#143(mem)", 256);
    for (off, h, nm) in [
        (0x9cu32, h_old_alloc as machine::ApiFn, "alloc"),
        (0xa0, h_old_free, "free"),
        (0x214, h_old_memset, "memset"),
    ] {
        let p = machine::new_trap(uc, &format!("oldsys#143(mem).{nm}"), Some(h));
        uc.w32(addr + off, p);
    }
    addr
}

fn h_old_alloc(uc: &mut Emu) {
    let n = uc.arg(0);
    if n == 0 {
        uc.ret(0);
        return;
    }
    let p = uc
        .get_data_mut()
        .heap
        .alloc(n.max(4), "oldsdk", false)
        .unwrap_or(0);
    uc.ret(p);
}

fn h_old_free(uc: &mut Emu) {
    uc.ret(0);
}

fn h_old_memset(uc: &mut Emu) {
    let (p, v, n) = (uc.arg(0), (uc.arg(1) & 0xFF) as u8, uc.arg(2));
    if p != 0 && n > 0 && n <= 0x40_0000 {
        let buf = vec![v; n as usize];
        uc.write(p, &buf);
    }
    uc.ret(p);
}

pub fn boot(uc: &mut Emu) -> u32 {
    let host = uc.get_data().rt.host;

    if uc.get_data().rt.style == Style::Old {
        let g = machine::new_table_ex(uc, "GameManagerOld", 0x100, true);
        uc.get_data_mut().rt.game_tbl = g;
        uc.w32(host + 0x0c, g);
    }
    let tramp = machine::new_trap(uc, "OldSysCall", Some(h_old_syscall));
    uc.w32(host + 0x00, 0xE51F_F004);
    uc.w32(host + 0x04, tramp);

    let ro = uc.get_data().ro_base;
    let r = machine::call(uc, ro | 1, &[host]);

    let cb0 = uc.r32(host + 0x00);
    if cb0 != 0xE51F_F004 {

        let cb1 = uc.r32(host + 0x04);
        let d = uc.get_data_mut();
        d.rt.mod_cb0 = cb0;
        d.rt.mod_cb1 = cb1;
    }
    r
}

pub const NO_EVENT: u32 = 0xFF;

pub const S_INIT: u32 = 0;
pub const S_DESTROY: u32 = 1;
pub const S_LOGIC: u32 = 2;
pub const S_RENDER: u32 = 3;
pub const S_PAUSE: u32 = 4;
pub const S_RESUME: u32 = 5;
pub const S_LOADRES: u32 = 6;

pub fn defer(uc: &mut Emu, fnptr: u32, args: Vec<u32>, tag: &'static str) {
    if fnptr != 0 {
        uc.get_data_mut().rt.pending.push((fnptr, args, tag));
    }
}

pub fn pump(uc: &mut Emu, limit: usize) -> usize {
    let mut n = 0;
    while n < limit {
        let Some((f, args, _)) = ({
            let p = &mut uc.get_data_mut().rt.pending;
            if p.is_empty() {
                None
            } else {
                Some(p.remove(0))
            }
        }) else {
            break;
        };
        machine::call(uc, f, &args);
        n += 1;
    }
    n
}

pub fn call_screen(uc: &mut Emu, scr: u32, slot: u32, args: &[u32]) -> Option<u32> {
    let f = uc.r32(scr + 4 * slot);
    if f == 0 {
        return None;
    }
    if std::env::var("REGDBG").is_ok() {
        let r: Vec<String> = (0..13).map(|i| format!("{:#x}", uc.reg(i))).collect();
        eprintln!("REG slot={slot} fn={f:#x} args={args:?} r0-12={}", r.join(","));
    }
    Some(machine::call(uc, f, args))
}

pub fn live_screens(uc: &Emu) -> Vec<(u32, u32, u32)> {
    uc.get_data()
        .rt
        .screens
        .iter()
        .copied()
        .filter(|&(scr, _, _)| scr != 0 && (0..7).any(|k| uc.r32(scr + 4 * k) != 0))
        .collect()
}

pub fn push_screen(uc: &mut Emu, scr: u32, param: u32, flag: u32) {
    uc.get_data_mut().rt.screens.push((scr, param, flag));

    let init = uc.r32(scr + 4 * S_INIT);
    let load = uc.r32(scr + 4 * S_LOADRES);
    defer(uc, init, vec![param], "screenInit");
    defer(uc, load, vec![param], "screenLoadResource");
}

pub fn app_start(uc: &mut Emu) -> u32 {
    let cb0 = uc.get_data().rt.mod_cb0;
    let r = if cb0 != 0 { machine::call(uc, cb0, &[]) } else { 0 };
    pump(uc, 64);
    r
}

pub fn app_stop(uc: &mut Emu) -> u32 {
    let cb = uc.get_data().rt.mod_cb1;
    if cb == 0 {
        return 0;
    }
    let r = machine::call(uc, cb, &[]);
    pump(uc, 64);
    r
}

pub fn frame(uc: &mut Emu, event: u32, data: u32) {
    if uc.get_data().rt.style == Style::Old && live_screens(uc).is_empty() {

        let cb0 = uc.get_data().rt.mod_cb0;
        if cb0 != 0 {
            machine::call(uc, cb0, &[]);
        }
        pump(uc, 64);
        uc.get_data_mut().rt.frames += 1;
        return;
    }
    {
        let d = uc.get_data_mut();
        d.rt.tick += d.rt.frame_ms;
    }
    {
        let now = uc.get_data().rt.tick;
        uc.get_data_mut().rt.audio.tick(now);
    }
    fire_timers(uc);
    pump(uc, 64);

    let live = uc.get_data().rt.screens.clone();
    for &(scr, param, _) in live.iter().rev() {
        if let Some(r) = call_screen(uc, scr, S_LOGIC, &[param, event, data]) {
            if r != 0 {
                break;
            }
        }
    }

    let mut i = 0usize;
    loop {
        let Some(&(scr, param, _)) = uc.get_data().rt.screens.get(i) else {
            break;
        };
        call_screen(uc, scr, S_RENDER, &[param]);
        i += 1;
    }
    uc.get_data_mut().rt.frames += 1;
    uc.get_data_mut().rt.gfx.frames += 1;
}

pub fn start_timer(uc: &mut Emu, ms: u64, cb: u32, param: u32) -> u32 {
    if cb == 0 {
        return 0;
    }
    let d = &mut uc.get_data_mut().rt;
    let id = d.next_timer;
    d.next_timer += 1;
    let due = d.tick + ms;
    d.timers.push(Timer { id, due, cb, param });
    id
}

fn fire_timers(uc: &mut Emu) {
    let now = uc.get_data().rt.tick;
    let due: Vec<Timer> = {
        let d = &mut uc.get_data_mut().rt;
        let (fire, keep): (Vec<_>, Vec<_>) = d.timers.iter().partition(|t| t.due <= now);
        d.timers = keep;
        fire
    };
    for t in due {
        defer(uc, t.cb, vec![t.param], "timer");
    }
}

pub fn press(uc: &mut Emu, mask: u32) {
    let d = &mut uc.get_data_mut().rt;
    d.keys_down |= mask;
    d.keys_hold |= mask;
    d.keys_up &= !mask;
}

pub fn release_all(uc: &mut Emu) {
    let d = &mut uc.get_data_mut().rt;
    d.keys_up = d.keys_down;
    d.keys_down = 0;
    d.keys_hold = 0;
}

pub fn install(uc: &mut Emu, addr: u32, name: &'static str, f: machine::ApiFn) {
    let key = (addr, name);
    let t = match uc.get_data().rt.installed.get(&key) {
        Some(&t) => t,
        None => {
            let t = machine::new_trap(uc, name, Some(f));
            uc.get_data_mut().rt.installed.insert(key, t);
            t
        }
    };
    uc.w32(addr, t);
}

pub fn method(uc: &mut Emu, name: &'static str, f: machine::ApiFn) -> u32 {
    if let Some(&t) = uc.get_data().rt.methods.get(name) {
        return t;
    }
    let t = machine::new_trap(uc, name, Some(f));
    uc.get_data_mut().rt.methods.insert(name, t);
    t
}

pub type CoK = Box<dyn FnOnce(&mut Emu, u32) -> Co>;

pub enum Co {
    Call(u32, Vec<u32>, CoK),
    Done(u32),
}

pub fn cocall(uc: &mut Emu, co: Co) {
    let lr = uc.lr();
    let sp = uc.reg_read(unicorn_engine::RegisterARM::SP).unwrap_or(0) as u32;
    co_step(uc, co, lr, sp, true);
}

fn co_step(uc: &mut Emu, co: Co, lr: u32, sp: u32, first: bool) {
    use unicorn_engine::RegisterARM;
    match co {
        Co::Done(v) => {
            uc.ret(v);
            if !first {
                let _ = uc.reg_write(RegisterARM::SP, sp as u64);
                let _ = uc.reg_write(RegisterARM::LR, lr as u64);
                uc.get_data_mut().resume_pc = Some(lr);
                let _ = uc.emu_stop();
            }
        }
        Co::Call(f, args, k) => {
            if uc.get_data().rt.co_trap == 0 {
                let t = machine::new_trap(uc, "<hostco-return>", Some(co_return));
                uc.get_data_mut().rt.co_trap = t;
            }
            let extra: Vec<u32> = args.iter().skip(4).copied().collect();
            let nsp = if extra.is_empty() {
                sp
            } else {
                (sp - 4 * extra.len() as u32) & !7
            };
            for (i, a) in extra.iter().enumerate() {
                uc.w32(nsp + 4 * i as u32, *a);
            }
            for (i, a) in args.iter().take(4).enumerate() {
                uc.setreg(i as u32, *a);
            }
            let _ = uc.reg_write(RegisterARM::SP, nsp as u64);
            uc.get_data_mut().rt.co_frames.push((k, lr, sp));
            let t = uc.get_data().rt.co_trap;
            let _ = uc.reg_write(RegisterARM::LR, t as u64);
            uc.get_data_mut().resume_pc = Some(f);
            let _ = uc.emu_stop();
        }
    }
}

fn co_return(uc: &mut Emu) {

    if uc.get_data().resume_pc.is_some() || uc.get_data().rt.co_frames.is_empty() {
        return;
    }
    let Some((k, lr, sp)) = uc.get_data_mut().rt.co_frames.pop() else {
        return;
    };
    let r0 = uc.reg(0);
    let co = k(uc, r0);
    co_step(uc, co, lr, sp, false);
}

pub fn user_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
