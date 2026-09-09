
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

use crate::session::{Event, Session, Touch};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

mod keys {
    pub const LSK: u32 = 1 << 12;
    pub const RSK: u32 = 1 << 13;
    pub const CALL: u32 = 1 << 20;
    pub const UP: u32 = (1 << 2) | (1 << 17);
    pub const DOWN: u32 = (1 << 8) | (1 << 18);
    pub const LEFT: u32 = (1 << 4) | (1 << 15);
    pub const RIGHT: u32 = (1 << 6) | (1 << 16);
    pub const OK: u32 = (1 << 5) | (1 << 14);
    pub const K1: u32 = 1 << 19;
    pub const K2: u32 = 1 << 18;
    pub const K3: u32 = 1 << 20;
    pub const K4: u32 = 1 << 15;
    pub const K5: u32 = 1 << 14;
    pub const K6: u32 = 1 << 16;
    pub const K7: u32 = 1 << 21;
    pub const K8: u32 = 1 << 17;
    pub const K9: u32 = 1 << 22;
    pub const STAR: u32 = 1 << 23;
    pub const K0: u32 = 1 << 24;
    pub const POUND: u32 = 1 << 25;
}

struct Pad(&'static str, u32, i32);

const PAD_ROWS: &[&[Pad]] = &[
    &[Pad("左软键", keys::LSK, 3), Pad("右软键", keys::RSK, 3)],
    &[Pad("呼叫", keys::CALL, 2), Pad("▲", keys::UP, 2), Pad("挂断", 0, 2)],
    &[Pad("◀", keys::LEFT, 2), Pad("OK", keys::OK, 2), Pad("▶", keys::RIGHT, 2)],
    &[Pad("", 0, 2), Pad("▼", keys::DOWN, 2), Pad("", 0, 2)],
    &[Pad("1", keys::K1, 2), Pad("2", keys::K2, 2), Pad("3", keys::K3, 2)],
    &[Pad("4", keys::K4, 2), Pad("5", keys::K5, 2), Pad("6", keys::K6, 2)],
    &[Pad("7", keys::K7, 2), Pad("8", keys::K8, 2), Pad("9", keys::K9, 2)],
    &[Pad("✱", keys::STAR, 2), Pad("0", keys::K0, 2), Pad("#", keys::POUND, 2)],
];

fn vk_to_mask(vk: u32) -> Option<u32> {
    Some(match vk {
        0x57 | 0x26 => keys::UP,
        0x53 | 0x28 => keys::DOWN,
        0x41 | 0x25 => keys::LEFT,
        0x44 | 0x27 => keys::RIGHT,
        0x4A | 0x20 | 0x0D => keys::OK,
        0x4B => keys::LSK,
        0x4C => keys::RSK,
        0x55 => keys::CALL,
        0x31 => keys::K1,
        0x32 => keys::K2,
        0x33 => keys::K3,
        0x34 => keys::K4,
        0x35 => keys::K5,
        0x36 => keys::K6,
        0x37 => keys::K7,
        0x38 => keys::K8,
        0x39 => keys::K9,
        0x30 => keys::K0,
        _ => return None,
    })
}

const ID_OPEN: usize = 1000;
const ID_STOP: usize = 1001;
const ID_FPS: usize = 1002;
const ID_ZOOM: usize = 1003;
const ID_PAD_BASE: usize = 2000;

const FPS_MIN: i32 = 1;
const FPS_MAX: i32 = 240;

const BAR_H: i32 = 34;
const STATUS_H: i32 = 22;

struct App {
    sess: Option<Session>,

    held: u32,

    latched: u32,

    pad_held: u32,
    fps: i32,
    scale: i32,

    real_fps: f64,
    frames: u32,
    mark: Instant,
    title: String,

    argb: Vec<u32>,
    fw: i32,
    fh: i32,
    pad_btns: Vec<HWND>,
    status: HWND,
    quitting: bool,
}

impl App {
    fn new() -> App {
        App {
            sess: None,
            held: 0,
            latched: 0,
            pad_held: 0,
            fps: 30,
            scale: 2,
            real_fps: 0.0,
            frames: 0,
            mark: Instant::now(),
            title: String::from("未加载模块"),
            argb: Vec::new(),
            fw: 240,
            fh: 400,
            pad_btns: Vec::new(),
            status: HWND(std::ptr::null_mut()),
            quitting: false,
        }
    }

    fn set_keys(&mut self, mask: u32) {
        self.latched |= mask & !self.held;
        self.held = mask;
    }

    fn status_text(&self) -> String {
        format!(
            "{}  {}x{}  {}fps（实测 {:.1}）",
            self.title, self.fw, self.fh, self.fps, self.real_fps
        )
    }
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn set_text(h: HWND, s: &str) {
    let w = wide(s);
    unsafe { let _ = SetWindowTextW(h, PCWSTR(w.as_ptr())); }
}

static mut APP: Option<Box<App>> = None;

fn app() -> &'static mut App {
    unsafe {
        let p = &raw mut APP;
        (*p).as_mut().expect("APP 未初始化").as_mut()
    }
}

pub fn run() {
    unsafe {

        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        APP = Some(Box::new(App::new()));

        let inst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let cls = w!("NiecheEmuWindow");
        let wc = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hInstance: inst,
            lpszClassName: cls,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),

            hbrBackground: HBRUSH(COLOR_BTNFACE.0 as isize as *mut _),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            cls,
            w!("尼彩 CBE 模拟器"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            560,
            980,
            None,
            None,
            Some(inst),
            None,
        )
        .expect("建窗口失败");

        let args: Vec<String> = std::env::args().collect();
        if let Some(p) = args.get(1) {
            start(hwnd, PathBuf::from(p));
        }

        message_loop(hwnd);
    }
}

unsafe fn message_loop(hwnd: HWND) {
    let mut msg = MSG::default();
    let mut next = Instant::now();
    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if app().quitting {
            return;
        }
        let now = Instant::now();
        if now >= next {
            let period = Duration::from_secs_f64(1.0 / app().fps.max(1) as f64);
            next = now + period;
            if step(hwnd) {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        } else {
            std::thread::sleep(Duration::from_millis(1).min(next - now));
        }
    }
}

unsafe fn step(hwnd: HWND) -> bool {
    let a = app();
    let Some(s) = a.sess.as_mut() else { return false };
    let bits = a.held | a.pad_held | a.latched;
    a.latched = 0;
    s.set_keys(bits);
    let px = s.step();
    let (w, h) = s.size();
    a.fw = w as i32;
    a.fh = h as i32;

    let n = (w * h) as usize;
    if a.argb.len() != n {
        a.argb.resize(n, 0);
    }

    for i in 0..n.min(px.len() / 2) {
        let v = u16::from_le_bytes([px[i * 2], px[i * 2 + 1]]) as u32;
        let r = ((v >> 11) & 0x1F) * 255 / 31;
        let g = ((v >> 5) & 0x3F) * 255 / 63;
        let b = (v & 0x1F) * 255 / 31;
        a.argb[i] = (r << 16) | (g << 8) | b;
    }

    let mut bye = false;
    for e in s.take_events() {
        if matches!(e, Event::Exit) {
            bye = true;
        }
    }
    if bye {
        stop();
    }

    a.frames += 1;
    let dt = a.mark.elapsed().as_secs_f64();
    if dt >= 1.0 {
        a.real_fps = a.frames as f64 / dt;
        a.frames = 0;
        a.mark = Instant::now();
        let t = a.status_text();
        set_text(a.status, &t);
    }
    let _ = hwnd;
    true
}

unsafe fn start(hwnd: HWND, path: PathBuf) {
    stop();
    let a = app();
    match Session::open(&path.to_string_lossy()) {
        Ok(mut s) => {
            s.boot();
            let (w, h) = s.size();
            a.fw = w as i32;
            a.fh = h as i32;
            a.title = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            a.sess = Some(s);
        }
        Err(e) => {
            a.title = format!("打不开：{e}");
        }
    }
    let t = a.status_text();
    set_text(a.status, &t);
    layout(hwnd);
}

unsafe fn stop() {
    let a = app();
    if let Some(s) = a.sess.as_mut() {
        s.stop();
    }
    a.sess = None;
    a.held = 0;
    a.pad_held = 0;
    a.latched = 0;
}

unsafe fn pick(hwnd: HWND) {
    let mut buf = [0u16; 1024];
    let filter: Vec<u16> = "CBE 模块\0*.cbe;*.CBE\0所有文件\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: windows::core::PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        ..Default::default()
    };
    if GetOpenFileNameW(&mut ofn).as_bool() {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(0);
        let p = String::from_utf16_lossy(&buf[..end]);
        start(hwnd, PathBuf::from(p));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            create_children(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            layout(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            on_command(hwnd, (wp.0 & 0xFFFF) as usize);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if let Some(m) = vk_to_mask(wp.0 as u32) {
                let a = app();
                let v = a.held | m;
                a.set_keys(v);
            }
            LRESULT(0)
        }
        WM_KEYUP => {
            if let Some(m) = vk_to_mask(wp.0 as u32) {
                let a = app();
                let v = a.held & !m;
                a.set_keys(v);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE => {
            on_mouse(hwnd, msg, lp);
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => {
            stop();
            app().quitting = true;
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn create_children(hwnd: HWND) {
    let inst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
    let mk = |txt: &str, id: usize| -> HWND {
        let t = wide(txt);
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            PCWSTR(t.as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            0,
            0,
            0,
            0,
            Some(hwnd),
            Some(HMENU(id as *mut _)),
            Some(inst),
            None,
        )
        .unwrap()
    };
    mk("打开 .cbe", ID_OPEN);
    mk("停止", ID_STOP);
    mk("帧率", ID_FPS);
    mk("缩放", ID_ZOOM);

    let a = app();
    a.status = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        w!("未加载模块"),
        WS_CHILD | WS_VISIBLE,
        0,
        0,
        0,
        0,
        Some(hwnd),
        None,
        Some(inst),
        None,
    )
    .unwrap();

    let mut id = ID_PAD_BASE;
    for row in PAD_ROWS {
        for p in *row {
            if p.0.is_empty() {
                id += 1;
                continue;
            }
            a.pad_btns.push(mk(p.0, id));
            id += 1;
        }
    }
}

unsafe fn layout(hwnd: HWND) {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let w = rc.right - rc.left;
    let a = app();

    let bw = w / 4;
    let mut x = 0;
    for id in [ID_OPEN, ID_STOP, ID_FPS, ID_ZOOM] {
        if let Ok(h) = get_child(hwnd, id) {
            let _ = MoveWindow(h, x, 2, bw - 2, BAR_H - 4, true);
        }
        x += bw;
    }
    if !a.status.is_invalid() {
        let _ = MoveWindow(a.status, 6, BAR_H, w - 12, STATUS_H, true);
    }

    let rows = PAD_ROWS.len() as i32;
    let ph = 34;
    let pad_top = rc.bottom - rows * ph - 4;
    let mut i = 0usize;
    for (r, row) in PAD_ROWS.iter().enumerate() {
        let total: i32 = row.iter().map(|p| p.2).sum();
        let mut px = 4;
        for p in *row {
            let cw = (w - 8) * p.2 / total;
            if !p.0.is_empty() {
                if let Some(&h) = a.pad_btns.get(i) {
                    let _ = MoveWindow(h, px, pad_top + r as i32 * ph, cw - 3, ph - 3, true);
                }
                i += 1;
            }
            px += cw;
        }
    }
    let _ = InvalidateRect(Some(hwnd), None, true);
}

unsafe fn get_child(hwnd: HWND, id: usize) -> Result<HWND, ()> {
    let h = GetDlgItem(Some(hwnd), id as i32);
    match h {
        Ok(h) if !h.is_invalid() => Ok(h),
        _ => Err(()),
    }
}

unsafe fn screen_rect(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let a = app();
    let top = BAR_H + STATUS_H + 4;
    let bottom = rc.bottom - PAD_ROWS.len() as i32 * 34 - 8;
    let avail_h = (bottom - top).max(1);
    let avail_w = rc.right - 8;

    let s = (avail_w / a.fw).min(avail_h / a.fh).max(1).min(a.scale.max(1));
    let dw = a.fw * s;
    let dh = a.fh * s;
    let x = (rc.right - dw) / 2;
    RECT { left: x, top, right: x + dw, bottom: top + dh }
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let a = app();
    if a.sess.is_some() && !a.argb.is_empty() {
        let r = screen_rect(hwnd);
        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: a.fw,

                biHeight: -a.fh,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        SetStretchBltMode(hdc, COLORONCOLOR);
        StretchDIBits(
            hdc,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            0,
            0,
            a.fw,
            a.fh,
            Some(a.argb.as_ptr() as *const _),
            &bi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn on_mouse(hwnd: HWND, msg: u32, lp: LPARAM) {
    let a = app();
    if a.sess.is_none() {
        return;
    }
    let x = (lp.0 & 0xFFFF) as i16 as i32;
    let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
    let r = screen_rect(hwnd);
    if x < r.left || x >= r.right || y < r.top || y >= r.bottom {
        return;
    }
    let mx = (x - r.left) * a.fw / (r.right - r.left).max(1);
    let my = (y - r.top) * a.fh / (r.bottom - r.top).max(1);
    let st = match msg {
        WM_LBUTTONDOWN => Touch::Down,
        WM_LBUTTONUP => Touch::Up,
        _ => {
            if (GetKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) == 0 {
                return;
            }
            Touch::Move
        }
    };
    if let Some(s) = a.sess.as_mut() {
        s.set_touch(mx, my, st);
    }
}

unsafe fn on_command(hwnd: HWND, id: usize) {
    match id {
        ID_OPEN => pick(hwnd),
        ID_STOP => {
            stop();
            let a = app();
            a.title = String::from("已停止");
            let t = a.status_text();
            set_text(a.status, &t);
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
        ID_FPS => ask_fps(hwnd),
        ID_ZOOM => {
            let a = app();
            a.scale = if a.scale >= 4 { 1 } else { a.scale + 1 };
            layout(hwnd);
        }
        _ if id >= ID_PAD_BASE => {

            let mut i = ID_PAD_BASE;
            for row in PAD_ROWS {
                for p in *row {
                    if i == id {
                        let a = app();
                        a.latched |= p.1;
                        return;
                    }
                    i += 1;
                }
            }
        }
        _ => {}
    }
}

unsafe fn ask_fps(hwnd: HWND) {
    let a = app();
    if let Some(v) = input_number(hwnd, &format!("{}", a.fps)) {
        a.fps = v.clamp(FPS_MIN, FPS_MAX);
        let t = a.status_text();
        set_text(a.status, &t);
    }
}

unsafe fn input_number(owner: HWND, initial: &str) -> Option<i32> {
    let inst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
    let cls = w!("NiecheEmuInput");
    static mut REGISTERED: bool = false;
    let reg = &raw mut REGISTERED;
    if !*reg {
        let wc = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hInstance: inst,
            lpszClassName: cls,
            lpfnWndProc: Some(dlgproc),
            hbrBackground: HBRUSH(COLOR_BTNFACE.0 as isize as *mut _),
            ..Default::default()
        };
        RegisterClassW(&wc);
        *reg = true;
    }

    let dlg = CreateWindowExW(
        WS_EX_DLGMODALFRAME,
        cls,
        w!("帧率"),
        WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        320,
        180,
        Some(owner),
        None,
        Some(inst),
        None,
    )
    .ok()?;

    let tip = wide("这就是游戏速度：模块按帧推进，跑多快游戏就多快。\r\n真机上这些游戏大概只有 10-15fps。范围 1-240。");
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        PCWSTR(tip.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        12,
        10,
        290,
        56,
        Some(dlg),
        None,
        Some(inst),
        None,
    )
    .ok()?;

    let init = wide(initial);
    let edit = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR(init.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_NUMBER as u32 | ES_RIGHT as u32),
        12,
        74,
        90,
        24,
        Some(dlg),
        None,
        Some(inst),
        None,
    )
    .ok()?;

    let ok_t = wide("确定");
    let _ok = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        PCWSTR(ok_t.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        130,
        110,
        80,
        26,
        Some(dlg),
        Some(HMENU(1 as *mut _)),
        Some(inst),
        None,
    )
    .ok()?;
    let cancel_t = wide("取消");
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        PCWSTR(cancel_t.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        218,
        110,
        80,
        26,
        Some(dlg),
        Some(HMENU(2 as *mut _)),
        Some(inst),
        None,
    )
    .ok()?;

    let _ = EnableWindow(owner, false);
    let _ = SetFocus(Some(edit));
    let _ = SendMessageW(edit, 0x00B1, Some(WPARAM(0)), Some(LPARAM(-1)));
    DLG_RESULT.store(0, Ordering::SeqCst);

    let mut result: Option<i32> = None;
    let mut msg = MSG::default();
    while DLG_RESULT.load(Ordering::SeqCst) == 0 {
        if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
            break;
        }

        if msg.message == WM_KEYDOWN {
            let vk = msg.wParam.0 as u32;
            if vk == VK_RETURN.0 as u32 {
                DLG_RESULT.store(1, Ordering::SeqCst);
                break;
            }
            if vk == VK_ESCAPE.0 as u32 {
                DLG_RESULT.store(2, Ordering::SeqCst);
                break;
            }
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
        if !IsWindow(Some(dlg)).as_bool() {
            break;
        }
    }
    if DLG_RESULT.load(Ordering::SeqCst) == 1 {
        result = read_int(edit);
    }

    let _ = EnableWindow(owner, true);
    let _ = DestroyWindow(dlg);
    let _ = SetForegroundWindow(owner);
    result
}

static DLG_RESULT: AtomicI32 = AtomicI32::new(0);

unsafe extern "system" fn dlgproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_COMMAND => {
            let id = (w.0 & 0xFFFF) as i32;
            if id == 1 || id == 2 {
                DLG_RESULT.store(id, Ordering::SeqCst);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            DLG_RESULT.store(2, Ordering::SeqCst);
            LRESULT(0)
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

unsafe fn read_int(edit: HWND) -> Option<i32> {
    let mut buf = [0u16; 16];
    let n = GetWindowTextW(edit, &mut buf);
    if n <= 0 {
        return None;
    }
    String::from_utf16_lossy(&buf[..n as usize]).trim().parse().ok()
}
