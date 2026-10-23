
use std::collections::{HashMap, VecDeque};

use unicorn_engine::unicorn_const::{Arch, HookType, MemType, Mode, Prot};
use unicorn_engine::{RegisterARM, Unicorn};

use crate::mem::{layout::*, place, Bump, Region, PAGE};

pub type Emu<'a> = Unicorn<'a, State>;

pub type ApiFn = fn(&mut Emu);

pub struct Slot {
    pub name: String,
    pub handler: Option<ApiFn>,

    pub tag: Option<&'static str>,
    pub off: u32,
}

pub const CALL_TIMEOUT_MS: u64 = 5_000;

extern "C" {

    fn uc_emu_stop(uc: *mut core::ffi::c_void) -> i32;
}

fn mono_ms() -> u64 {
    static BASE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    BASE.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64 + 1
}

pub struct Watchdog {

    deadline: std::sync::Arc<std::sync::atomic::AtomicU64>,
    fired: std::sync::Arc<std::sync::atomic::AtomicBool>,
    quit: std::sync::Arc<std::sync::atomic::AtomicBool>,
    th: Option<std::thread::JoinHandle<()>>,
}

impl Watchdog {
    fn start(handle: usize) -> Watchdog {
        use std::sync::atomic::Ordering::SeqCst;
        use std::sync::atomic::{AtomicBool, AtomicU64};
        use std::sync::Arc;
        let deadline = Arc::new(AtomicU64::new(0));
        let fired = Arc::new(AtomicBool::new(false));
        let quit = Arc::new(AtomicBool::new(false));
        let (d, f, q) = (deadline.clone(), fired.clone(), quit.clone());
        let th = std::thread::Builder::new()
            .name("nieche-watchdog".into())
            .spawn(move || {
                while !q.load(SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let t = d.load(SeqCst);
                    if t != 0 && mono_ms() >= t && d.compare_exchange(t, 0, SeqCst, SeqCst).is_ok() {
                        f.store(true, SeqCst);
                        unsafe {
                            uc_emu_stop(handle as *mut core::ffi::c_void);
                        }
                    }
                }
            })
            .ok();
        Watchdog { deadline, fired, quit, th }
    }

    fn arm(&self) {
        self.arm_for(CALL_TIMEOUT_MS);
    }

    fn arm_for(&self, ms: u64) {
        use std::sync::atomic::Ordering::SeqCst;
        self.fired.store(false, SeqCst);
        self.deadline.store(mono_ms() + ms, SeqCst);
    }

    fn disarm(&self) {
        self.deadline.store(0, std::sync::atomic::Ordering::SeqCst);
    }

    fn fired(&self) -> bool {
        self.fired.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.deadline.store(0, std::sync::atomic::Ordering::SeqCst);
        self.quit.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(t) = self.th.take() {
            let _ = t.join();
        }
    }
}

const SH_WRITEC: u32 = 0x03;
const SH_WRITE0: u32 = 0x04;
const SH_WRITE: u32 = 0x05;
const SH_EXIT: u32 = 0x18;

pub const CALL_LOG_CAP: usize = 4096;

pub struct State {
    pub le: bool,
    pub ro_base: u32,
    pub ro_size: u32,
    pub rw_base: u32,
    pub rw_size: u32,
    pub regions: Vec<Region>,
    pub heap: Bump,
    pub data: Bump,

    pub slots: Vec<Slot>,
    pub trap_hits: HashMap<u32, u64>,
    pub null_calls: HashMap<u32, u64>,

    pub host_errors: HashMap<(String, String), u64>,

    pub r9_live: u32,
    pub last_trap: i64,
    pub same_trap: u32,

    pub log_calls: bool,
    pub call_log_cap: usize,
    pub call_log: VecDeque<String>,

    pub in_emu: bool,
    pub resume_pc: Option<u32>,
    pub exit_reason: Option<String>,
    pub stopped: bool,
    pub next_native: u32,
    pub semihost_out: Vec<u8>,

    pub cur_slot: u32,

    pub auto_slots: std::collections::HashSet<u32>,

    pub auto_objs: HashMap<u32, u32>,

    pub rt: crate::runtime::Rt,

    pub watchdog: Option<Watchdog>,
}

impl State {
    pub fn where_(&self, a: u32) -> String {
        for r in &self.regions {
            if r.contains(a) {
                return format!("{}+{:#x}", r.name, a - r.base);
            }
        }
        format!("{a:#x}")
    }
}

pub trait Mach {
    fn le(&self) -> bool;

    fn r8(&self, a: u32) -> u8;
    fn r16(&self, a: u32) -> u16;
    fn r32(&self, a: u32) -> u32;
    fn w32(&mut self, a: u32, v: u32);
    fn w16(&mut self, a: u32, v: u16);
    fn write(&mut self, a: u32, b: &[u8]);

    fn cstr(&self, a: u32, maxlen: usize) -> Option<Vec<u8>>;

    fn read_upto(&self, a: u32, n: usize) -> Vec<u8>;

    fn reg(&self, i: u32) -> u32;
    fn setreg(&mut self, i: u32, v: u32);

    fn arg(&self, i: u32) -> u32;
    fn lr(&self) -> u32;
    fn ret(&mut self, v: u32);
    fn pc(&self) -> u32;
}

fn reg_of(i: u32) -> RegisterARM {
    match i {
        0 => RegisterARM::R0,
        1 => RegisterARM::R1,
        2 => RegisterARM::R2,
        3 => RegisterARM::R3,
        4 => RegisterARM::R4,
        5 => RegisterARM::R5,
        6 => RegisterARM::R6,
        7 => RegisterARM::R7,
        8 => RegisterARM::R8,
        9 => RegisterARM::R9,
        10 => RegisterARM::R10,
        11 => RegisterARM::R11,
        _ => RegisterARM::R12,
    }
}

impl Mach for Emu<'_> {
    fn le(&self) -> bool {
        self.get_data().le
    }

    fn r8(&self, a: u32) -> u8 {
        let mut b = [0u8; 1];
        let _ = self.mem_read(a as u64, &mut b);
        b[0]
    }

    fn r16(&self, a: u32) -> u16 {
        let mut b = [0u8; 2];
        let _ = self.mem_read(a as u64, &mut b);
        if self.le() {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        }
    }

    fn r32(&self, a: u32) -> u32 {
        let mut b = [0u8; 4];
        let _ = self.mem_read(a as u64, &mut b);
        if self.le() {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        }
    }

    fn w32(&mut self, a: u32, v: u32) {
        let b = if self.le() {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        let _ = self.mem_write(a as u64, &b);
    }

    fn w16(&mut self, a: u32, v: u16) {
        let b = if self.le() {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        let _ = self.mem_write(a as u64, &b);
    }

    fn write(&mut self, a: u32, b: &[u8]) {
        let _ = self.mem_write(a as u64, b);
    }

    fn cstr(&self, a: u32, maxlen: usize) -> Option<Vec<u8>> {
        if a == 0 {
            return None;
        }
        let mut out = Vec::new();
        while out.len() < maxlen {
            let b = self.r8(a + out.len() as u32);
            if b == 0 {
                break;
            }
            out.push(b);
        }
        Some(out)
    }

    fn read_upto(&self, a: u32, n: usize) -> Vec<u8> {
        if a == 0 {
            return Vec::new();
        }
        let mut n = n;
        if let Ok(regions) = self.mem_regions() {
            for r in regions {
                if (a as u64) >= r.begin && (a as u64) <= r.end {
                    n = n.min((r.end - a as u64 + 1) as usize);
                    break;
                }
            }
        }
        while n > 0 {
            if let Ok(v) = self.mem_read_as_vec(a as u64, n) {
                return v;
            }
            n /= 2;
        }
        Vec::new()
    }

    fn reg(&self, i: u32) -> u32 {
        self.reg_read(reg_of(i)).unwrap_or(0) as u32
    }

    fn setreg(&mut self, i: u32, v: u32) {
        let _ = self.reg_write(reg_of(i), v as u64);
    }

    fn arg(&self, i: u32) -> u32 {
        if i < 4 {
            return self.reg(i);
        }
        let sp = self.reg_read(RegisterARM::SP).unwrap_or(0) as u32;
        self.r32(sp + (i - 4) * 4)
    }

    fn lr(&self) -> u32 {
        self.reg_read(RegisterARM::LR).unwrap_or(0) as u32
    }

    fn ret(&mut self, v: u32) {
        self.setreg(0, v);
    }

    fn pc(&self) -> u32 {
        self.reg_read(RegisterARM::PC).unwrap_or(0) as u32
    }
}

pub fn build(m: &cbelib::CbeModule) -> Result<Emu<'static>, String> {
    let le = m.endian == cbelib::Endian::Le;
    let p = place(m);

    let st = State {
        le,
        ro_base: p.ro_base,
        ro_size: p.ro_size,
        rw_base: p.rw_base,
        rw_size: p.rw_size,
        regions: vec![
            Region { name: "RO", base: p.ro_base, size: p.ro_size },
            Region { name: "RW", base: p.rw_base, size: p.rw_size },
            Region { name: "STACK", base: STACK_BASE, size: STACK_SIZE },
            Region { name: "HEAP", base: HEAP_BASE, size: HEAP_SIZE },
            Region { name: "DATA", base: DATA_BASE, size: DATA_SIZE },
            Region { name: "TRAP", base: TRAP_BASE, size: TRAP_SIZE },
        ],
        heap: Bump::new(HEAP_BASE, HEAP_SIZE),
        data: Bump::new(DATA_BASE, DATA_SIZE),
        slots: Vec::new(),
        trap_hits: HashMap::new(),
        null_calls: HashMap::new(),
        host_errors: HashMap::new(),
        r9_live: p.rw_base,
        last_trap: -1,
        same_trap: 0,
        log_calls: true,
        call_log_cap: CALL_LOG_CAP,
        call_log: VecDeque::new(),
        in_emu: false,
        resume_pc: None,
        exit_reason: None,
        stopped: false,
        next_native: 0,
        semihost_out: Vec::new(),
        cur_slot: 0,
        auto_slots: std::collections::HashSet::new(),
        auto_objs: HashMap::new(),
        rt: crate::runtime::Rt::new(),
        watchdog: None,
    };

    let mode = if le {
        Mode::ARM | Mode::LITTLE_ENDIAN
    } else {
        Mode::ARM | Mode::BIG_ENDIAN
    };
    let mut uc = Unicorn::new_with_data(Arch::ARM, mode, st).map_err(|e| format!("{e:?}"))?;

    let _ = uc.ctl_set_cpu_model(unicorn_engine::ArmCpuModel::Model_926 as i32);
    let h = uc.get_handle() as usize;
    uc.get_data_mut().watchdog = Some(Watchdog::start(h));

    map_memory(&mut uc, m, &p)?;
    install_hooks(&mut uc)?;
    Ok(uc)
}

fn map_memory(uc: &mut Emu, m: &cbelib::CbeModule, p: &crate::mem::Placement) -> Result<(), String> {
    let e = |x: unicorn_engine::uc_error| format!("{x:?}");

    uc.mem_map(p.ro_base as u64, p.ro_size as u64, Prot::ALL)
        .map_err(e)?;
    uc.mem_write(p.ro_base as u64, &m.ro).map_err(e)?;

    if m.load_base != 0 {

        let need = crate::mem::align_page(p.rw_base - p.ro_base + p.rw_size);
        if need > p.ro_size {
            let extra = need - p.ro_size;
            uc.mem_map(
                (p.ro_base + p.ro_size) as u64,
                extra as u64,
                Prot::ALL,
            )
            .map_err(e)?;
            uc.get_data_mut().ro_size += extra;
            uc.get_data_mut().regions[0].size += extra;
        }
    } else {
        uc.mem_map(p.rw_base as u64, p.rw_size as u64, Prot::ALL)
            .map_err(e)?;
    }
    uc.mem_write(p.rw_base as u64, &m.rw).map_err(e)?;

    uc.mem_map(STACK_BASE as u64, STACK_SIZE as u64, Prot::ALL)
        .map_err(e)?;
    uc.mem_map(HEAP_BASE as u64, HEAP_SIZE as u64, Prot::ALL)
        .map_err(e)?;

    uc.mem_map(DATA_BASE as u64, DATA_SIZE as u64, Prot::READ | Prot::WRITE)
        .map_err(e)?;

    uc.mem_protect(DATA_BASE as u64, PAGE as u64, Prot::ALL)
        .map_err(e)?;
    uc.mem_map(TRAP_BASE as u64, TRAP_SIZE as u64, Prot::READ | Prot::EXEC)
        .map_err(e)?;
    uc.mem_map(
        NATIVE_BASE as u64,
        NATIVE_SIZE as u64,
        Prot::READ | Prot::EXEC,
    )
    .map_err(e)?;

    let le = uc.get_data().le;
    let bxlr: [u8; 2] = if le { [0x70, 0x47] } else { [0x47, 0x70] };
    let nop: [u8; 2] = if le { [0x00, 0xbf] } else { [0xbf, 0x00] };
    let mut fill = Vec::with_capacity(TRAP_SIZE as usize);
    for _ in 0..(TRAP_SIZE / 4) {
        fill.extend_from_slice(&bxlr);
        fill.extend_from_slice(&nop);
    }
    uc.mem_write(TRAP_BASE as u64, &fill).map_err(e)?;

    uc.mem_map(
        (RETURN_MAGIC & !(PAGE - 1)) as u64,
        PAGE as u64,
        Prot::READ | Prot::EXEC,
    )
    .map_err(e)?;

    uc.mem_map(0, NULL_GUARD as u64, Prot::ALL).map_err(e)?;
    Ok(())
}

fn install_hooks(uc: &mut Emu) -> Result<(), String> {
    let e = |x: unicorn_engine::uc_error| format!("{x:?}");

    uc.add_code_hook(
        TRAP_BASE as u64,
        (TRAP_BASE + TRAP_SIZE - 1) as u64,
        |uc: &mut Emu, addr: u64, _size: u32| {
            let idx = ((addr as u32 - TRAP_BASE) / 4) as usize;
            {
                let d = uc.get_data_mut();
                *d.trap_hits.entry(idx as u32).or_insert(0) += 1;

                if d.last_trap == idx as i64 {
                    d.same_trap += 1;
                } else {
                    d.last_trap = idx as i64;
                    d.same_trap = 0;
                }
            }
            if uc.get_data().log_calls {
                let name = uc
                    .get_data()
                    .slots
                    .get(idx)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| format!("trap#{idx}"));

                let line = if std::env::var("TRACEARGS").is_ok() {
                    let a: Vec<String> = (0..4).map(|i| format!("{:#x}", uc.reg(i))).collect();
                    format!("{name} {}", a.join(" "))
                } else {
                    name
                };
                let d = uc.get_data_mut();
                let cap = d.call_log_cap;
                if d.call_log.len() >= cap {
                    d.call_log.pop_front();
                }
                d.call_log.push_back(line);
            }
            uc.get_data_mut().cur_slot = idx as u32;

            let h = uc.get_data().slots.get(idx).and_then(|s| s.handler);

            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match h {
                Some(f) => f(uc),

                None => {
                    if uc.get_data().auto_slots.contains(&(idx as u32)) {
                        let v = auto_result(uc, idx as u32);
                        uc.ret(v);
                    } else {
                        uc.ret(0);
                    }
                }
            }));
            if let Err(p) = r {
                host_error(uc, idx, p.as_ref());
            }

            if uc.get_data().log_calls && std::env::var("TRACERET").is_ok() {
                let r = uc.reg(0);
                if let Some(last) = uc.get_data_mut().call_log.back_mut() {
                    last.push_str(&format!(" -> {r:#x}"));
                }
            }
        },
    )
    .map_err(e)?;

    uc.add_intr_hook(|uc: &mut Emu, intno: u32| {
        if intno != 2 {
            return;
        }
        let pc = uc.reg(15);
        let thumb = uc.reg_read(RegisterARM::CPSR).unwrap_or(0) & 0x20 != 0;
        let imm = if thumb {
            match uc.mem_read_as_vec(pc.wrapping_sub(2) as u64, 1) {
                Ok(b) => b[0] as u32,
                Err(_) => return,
            }
        } else {
            match uc.mem_read_as_vec(pc.wrapping_sub(4) as u64, 4) {
                Ok(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) & 0xFF_FFFF,
                Err(_) => return,
            }
        };
        if imm != 0xAB {
            return;
        }
        let (op, arg) = (uc.reg(0), uc.reg(1));
        match op {
            SH_WRITEC => {
                if let Ok(b) = uc.mem_read_as_vec(arg as u64, 1) {
                    uc.get_data_mut().semihost_out.extend_from_slice(&b);
                }
            }
            SH_WRITE0 => {
                let s = uc.cstr(arg, 512).unwrap_or_default();
                uc.get_data_mut().semihost_out.extend_from_slice(&s);
            }
            SH_WRITE => {
                let (p, n) = (uc.r32(arg + 4), uc.r32(arg + 8));
                if let Ok(b) = uc.mem_read_as_vec(p as u64, n.min(0x10000) as usize) {
                    uc.get_data_mut().semihost_out.extend_from_slice(&b);
                }
                uc.setreg(0, 0);
            }
            SH_EXIT => {
                uc.get_data_mut().exit_reason = Some("模块调用了 semihosting SYS_EXIT".into());
                let _ = uc.emu_stop();
            }
            _ => uc.setreg(0, 0),
        }
        let out = &mut uc.get_data_mut().semihost_out;
        if out.len() > 1 << 20 {
            out.drain(..1 << 19);
        }
    })
    .map_err(e)?;

    uc.add_code_hook(0, (NULL_GUARD - 1) as u64, |uc: &mut Emu, _a, _s| {
        let lr = uc.lr();
        uc.setreg(0, 0);
        let d = uc.get_data_mut();
        *d.null_calls.entry(lr & !1).or_insert(0) += 1;
        d.resume_pc = Some(lr);
        let _ = uc.emu_stop();
    })
    .map_err(e)?;

    let mut regions: Vec<(u64, u64)> = uc
        .mem_regions()
        .map_err(e)?
        .iter()
        .map(|r| (r.begin, r.end))
        .collect();
    regions.sort();
    let mut gaps = Vec::new();
    let mut cur = 0u64;
    for (b, end) in regions {
        if b > cur {
            gaps.push((cur, b - 1));
        }
        cur = end.saturating_add(1);
    }
    gaps.push((cur, u64::MAX));
    for (b, end) in gaps {
        uc.add_mem_hook(HookType::MEM_UNMAPPED, b, end, on_unmapped)
            .map_err(e)?;
    }

    uc.add_insn_invalid_hook(|uc: &mut Emu| {
        let pc = uc.pc();
        let d = uc.get_data_mut();
        if d.exit_reason.is_none() {
            d.exit_reason = Some(format!("非法指令 pc={pc:#x}"));
        }
        let _ = uc.emu_stop();
        false
    })
    .map_err(e)?;

    Ok(())
}

fn on_unmapped(uc: &mut Emu, t: MemType, addr: u64, size: usize, _v: i64) -> bool {
    let pc = uc.pc();
    let d = uc.get_data_mut();
    if d.exit_reason.is_none() {
        d.exit_reason = Some(format!("未映射{t:?} {addr:#x}+{size} pc={pc:#x}"));
    }
    let _ = uc.emu_stop();
    false
}

fn host_error(uc: &mut Emu, idx: usize, payload: &(dyn std::any::Any + Send)) {
    let msg = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panic".to_string());
    let msg: String = msg.chars().take(60).collect();
    let name = uc
        .get_data()
        .slots
        .get(idx)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| format!("trap#{idx}"));
    let d = uc.get_data_mut();
    let n = d.host_errors.entry((name.clone(), msg.clone())).or_insert(0);
    *n += 1;
    if *n == 1 {
        d.rt.logs.push(format!("[宿主错误] {name}: {msg}"));
    }
    uc.ret(0);
}

pub fn new_trap(uc: &mut Emu, name: &str, handler: Option<ApiFn>) -> u32 {
    new_trap_tagged(uc, name, handler, None, 0)
}

pub fn new_trap_tagged(
    uc: &mut Emu,
    name: &str,
    handler: Option<ApiFn>,
    tag: Option<&'static str>,
    off: u32,
) -> u32 {
    let d = uc.get_data_mut();
    let i = d.slots.len() as u32;
    assert!(i * 4 < TRAP_SIZE, "陷阱槽用尽");
    d.slots.push(Slot {
        name: name.to_string(),
        handler,
        tag,
        off,
    });
    (TRAP_BASE + i * 4) | 1
}

fn auto_result(uc: &mut Emu, idx: u32) -> u32 {
    if let Some(&a) = uc.get_data().auto_objs.get(&idx) {
        return a;
    }
    let name = uc
        .get_data()
        .slots
        .get(idx as usize)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let t = new_table(uc, &format!("obj<{name}>"), 96);
    uc.get_data_mut().auto_objs.insert(idx, t);
    t
}

pub fn new_table_ex(uc: &mut Emu, name: &str, nslots: u32, getters: bool) -> u32 {
    let addr = uc
        .get_data_mut()
        .data
        .alloc(nslots * 4, name, true)
        .expect("宿主结构区耗尽");
    for k in 0..nslots {
        let p = new_trap(uc, &format!("{name}+{:#x}", k * 4), None);
        if getters {
            let idx = ((p & !1) - TRAP_BASE) / 4;
            uc.get_data_mut().auto_slots.insert(idx);
        }
        uc.w32(addr + 4 * k, p);
    }
    addr
}

pub fn new_table(uc: &mut Emu, name: &str, nslots: u32) -> u32 {
    new_table_ex(uc, name, nslots, false)
}

pub fn native_const(uc: &mut Emu, value: u32) -> u32 {
    let base = {
        let d = uc.get_data_mut();
        let b = NATIVE_BASE + d.next_native * 8;
        d.next_native += 1;
        b
    };
    let le = uc.le();

    let ldr: u16 = 0x4800;
    let bx: u16 = 0x4770;
    let mut code = Vec::with_capacity(8);
    if le {
        code.extend_from_slice(&ldr.to_le_bytes());
        code.extend_from_slice(&bx.to_le_bytes());
        code.extend_from_slice(&value.to_le_bytes());
    } else {
        code.extend_from_slice(&ldr.to_be_bytes());
        code.extend_from_slice(&bx.to_be_bytes());
        code.extend_from_slice(&value.to_be_bytes());
    }
    uc.write(base, &code);
    base | 1
}

pub fn call(uc: &mut Emu, addr: u32, args: &[u32]) -> u32 {
    if uc.get_data().in_emu {
        panic!("不能在 unicorn 回调内嵌套 call()，请使用延迟队列");
    }
    for (i, &a) in args.iter().take(4).enumerate() {
        uc.setreg(i as u32, a);
    }
    let _ = uc.reg_write(RegisterARM::SP, (STACK_BASE + STACK_SIZE - 0x1000) as u64);
    let r9 = uc.get_data().r9_live;
    let _ = uc.reg_write(RegisterARM::R9, r9 as u64);
    let _ = uc.reg_write(RegisterARM::LR, (RETURN_MAGIC | 1) as u64);

    let thumb = addr & 1 != 0;
    let mut start = if thumb { addr | 1 } else { addr };
    set_thumb(uc, thumb);

    {
        let d = uc.get_data_mut();
        d.exit_reason = None;
        d.in_emu = true;
        d.resume_pc = None;
    }
    let mut resumes = 0u32;
    if let Some(w) = &uc.get_data().watchdog {
        w.arm();
    }
    loop {
        let r = uc.emu_start(start as u64, RETURN_MAGIC as u64, 0, 0);
        if let Err(err) = r {
            let pc = uc.pc();
            let d = uc.get_data_mut();
            if d.exit_reason.is_none() {
                d.exit_reason = Some(format!("{err:?} pc={pc:#x}"));
            }
            break;
        }
        let next = uc.get_data_mut().resume_pc.take();
        let Some(n) = next else { break };

        if uc.get_data().watchdog.as_ref().is_some_and(|w| w.fired()) {
            break;
        }
        start = n;
        set_thumb(uc, start & 1 != 0);
        resumes += 1;
        if resumes > 20000 {
            uc.get_data_mut().exit_reason = Some("空指针调用过多，放弃".into());
            break;
        }
    }
    if let Some(w) = &uc.get_data().watchdog {
        w.disarm();
    }
    uc.get_data_mut().in_emu = false;
    if uc.get_data().exit_reason.is_none() && (uc.pc() & !1) != (RETURN_MAGIC & !1) {

        let pc = uc.pc();
        uc.get_data_mut().exit_reason = Some(format!("单次调用超时 pc={pc:#x}"));
    }
    if uc.get_data().exit_reason.is_none() {

        let r9 = uc.reg(9);
        uc.get_data_mut().r9_live = r9;
    }
    uc.reg(0)
}

fn set_thumb(uc: &mut Emu, thumb: bool) {
    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
    let v = if thumb { cpsr | 0x20 } else { cpsr & !0x20 };
    let _ = uc.reg_write(RegisterARM::CPSR, v);
}

pub fn selftest() -> bool {
    let Ok(mut uc) = Unicorn::new(Arch::ARM, Mode::LITTLE_ENDIAN) else {
        return false;
    };
    if uc.mem_map(0x1000, 0x1000, Prot::ALL).is_err() {
        return false;
    }

    let code: [u8; 12] = [
        0x01, 0x00, 0xa0, 0xe3, 0x02, 0x10, 0xa0, 0xe3, 0x01, 0x00, 0x80, 0xe0,
    ];
    if uc.mem_write(0x1000, &code).is_err() {
        return false;
    }
    if uc.emu_start(0x1000, 0x1000 + code.len() as u64, 0, 0).is_err() {
        return false;
    }
    uc.reg_read(RegisterARM::R0) == Ok(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_stops_infinite_loop() {
        let mut uc = Unicorn::new(Arch::ARM, Mode::ARM | Mode::LITTLE_ENDIAN).unwrap();
        uc.mem_map(0x1000, 0x1000, Prot::ALL).unwrap();
        uc.mem_write(0x1000, &0xeaff_fffeu32.to_le_bytes()).unwrap();
        let w = Watchdog::start(uc.get_handle() as usize);
        w.arm_for(200);
        let t0 = std::time::Instant::now();
        uc.emu_start(0x1000, 0x2000, 0, 0).unwrap();
        let dt = t0.elapsed().as_millis();
        w.disarm();
        assert!(w.fired(), "看门狗没有触发");
        assert!((150..1500).contains(&dt), "停机用了 {dt}ms");
    }

    fn blank_module() -> cbelib::CbeModule {
        cbelib::CbeModule {
            path: String::new(),
            raw: Vec::new(),
            name: "blank".into(),
            load_base: 0,
            image_size: 0x1000,
            image_end: 0,
            rw_size: 0x1000,
            ro: vec![0; 0x1000],
            rw: vec![0; 0x1000],
            ro_off: 0,
            rw_off: 0,
            ro_chk: 0,
            rw_chk: 0,
            endian: cbelib::Endian::Le,
            icons: None,
            res: None,
            packages: Vec::new(),
            res_off: 0,
        }
    }

    #[allow(clippy::useless_vec)]
    fn panicky(_uc: &mut Emu) {
        let v = vec![1u32, 2, 3];
        let i = v.len() + 1;
        let _ = v[i];
    }

    fn fine(uc: &mut Emu) {
        uc.ret(42);
    }

    #[test]
    fn handler_panic_is_contained() {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let mut uc = build(&blank_module()).expect("搭机器");
        let bad = new_trap(&mut uc, "Bad", Some(panicky));
        let good = new_trap(&mut uc, "Good", Some(fine));

        assert_eq!(call(&mut uc, bad, &[7]), 0, "panic 之后按失败返回 0");
        assert_eq!(call(&mut uc, good, &[]), 42, "之后的调用照常");
        assert_eq!(call(&mut uc, bad, &[7]), 0);
        std::panic::set_hook(prev);

        let d = uc.get_data();
        let n: u64 = d
            .host_errors
            .iter()
            .filter(|((name, _), _)| name == "Bad")
            .map(|(_, n)| *n)
            .sum();
        assert_eq!(n, 2, "两次都记上了");
        let lines = d.rt.logs.iter().filter(|l| l.starts_with("[宿主错误] Bad")).count();
        assert_eq!(lines, 1, "同一种错误只报一行，不刷屏");
    }
}
