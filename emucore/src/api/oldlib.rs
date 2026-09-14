
use crate::api::fill;
use crate::gfx;
use crate::machine::{ApiFn, Emu, Mach};
use crate::runtime::{self, Co};

const SCREEN_W: i32 = 240;
const SCREEN_H: i32 = 400;

fn s16(v: i32) -> i32 {
    v as i16 as i32
}

fn a16(uc: &Emu, i: u32) -> i32 {
    s16(uc.arg(i) as i32)
}

fn w16(uc: &mut Emu, a: u32, v: i32) {
    let v = v as u16;
    let b = if uc.le() { v.to_le_bytes() } else { v.to_be_bytes() };
    uc.write(a, &b);
}

fn rs16(uc: &Emu, a: u32) -> i32 {
    uc.r16(a) as i16 as i32
}

fn alloc(uc: &mut Emu, size: u32, tag: &str) -> u32 {
    if size == 0 {
        return 0;
    }
    let p = uc.get_data_mut().heap.alloc(size, tag, false).unwrap_or(0);
    if p != 0 {
        fill(uc, p, 0, size);
    }
    p
}

pub fn set_clip(uc: &mut Emu) {
    let (x, y, w, h) = (a16(uc, 0), a16(uc, 1), a16(uc, 2), a16(uc, 3));
    if x <= 0 && y <= 0 && w > 0 && h > 0 {
        gfx::maybe_adopt(uc, w as u32, h as u32);
    }
    uc.get_data_mut().rt.oldlib_clip = [x, y, s16(x + w), s16(y + h)];
    uc.ret((y + h) as u32 & 0xFFFF);
}

pub fn get_clip(uc: &mut Emu) {
    let out = uc.arg(0);
    let [x0, y0, x1, y1] = uc.get_data().rt.oldlib_clip;
    let w = if x1 > x0 { x1 - x0 } else { 0 };
    let h = if y1 > y0 { y1 - y0 } else { 0 };
    if out != 0 {
        for (i, v) in [x0, y0, w, h].into_iter().enumerate() {
            w16(uc, out + 2 * i as u32, v);
        }
    }
    uc.ret(out);
}

fn check_clip_to(uc: &mut Emu, mw: i32, mh: i32) {
    let c = &mut uc.get_data_mut().rt.oldlib_clip;
    if c[2] >= 0 && c[0] <= mw && c[3] >= 0 && c[1] <= mh {
        if c[0] < 0 {
            c[0] = 0;
        }
        if c[2] > mw {
            c[2] = mw;
        }
        if c[3] > mh {
            c[3] = mh;
        }
    } else {
        *c = [0; 4];
    }
}

pub fn check_clip(uc: &mut Emu) {
    let (mw, mh) = (a16(uc, 0), a16(uc, 1));
    check_clip_to(uc, mw, mh);
    let v = uc.arg(0);
    uc.ret(v);
}

pub fn img_height(uc: &mut Emu) {
    let p = uc.arg(0);
    let v = if p != 0 { gfx::img_wh(uc, p).1 & 0xFFFF } else { 0 };
    uc.ret(v);
}

pub fn img_width(uc: &mut Emu) {
    let p = uc.arg(0);
    let v = if p != 0 { gfx::img_wh(uc, p).0 & 0xFFFF } else { 0 };
    uc.ret(v);
}

pub fn to_rgb(uc: &mut Emu) {
    let (r, g, b) = (uc.arg(0) & 0xFF, uc.arg(1) & 0xFF, uc.arg(2));
    uc.ret((((r & 0xF8) << 8) + 8 * (g & 0xFC) + (b >> 3)) & 0xFFFF);
}

#[allow(clippy::too_many_arguments)]
fn clipped_blit(
    uc: &mut Emu,
    dst: u32,
    src: u32,
    mut sx: i32,
    mut sy: i32,
    mut w: i32,
    mut h: i32,
    mut dx: i32,
    mut dy: i32,
    alpha: bool,
) {
    if src == 0 {
        return;
    }
    let [x0, y0, x1, y1] = uc.get_data().rt.oldlib_clip;
    if !(dx + w > x0 && dy + h > y0) {
        return;
    }
    if x0 > dx {
        w = s16(dx - x0 + w);
        sx = s16(sx - (dx - x0));
        dx = x0;
    }
    if y0 > dy {
        h = s16(dy - y0 + h);
        sy = s16(sy - (dy - y0));
        dy = y0;
    }
    if dx + w > x1 {
        w = s16(x1 - dx);
    }
    if w <= 0 {
        return;
    }
    if dy + h > y1 {
        h = s16(y1 - dy);
    }
    if h <= 0 {
        return;
    }
    let dst = if dst != 0 { dst } else { uc.get_data().rt.gfx.img };
    gfx::blit(uc, src, dst, dx, dy, Some(w), Some(h), sx, sy, alpha);
}

fn blit8(uc: &mut Emu, alpha: bool) {
    let (dst, src) = (uc.arg(0), uc.arg(1));
    let (sx, sy, w, h, dx, dy) = (a16(uc, 2), a16(uc, 3), a16(uc, 4), a16(uc, 5), a16(uc, 6), a16(uc, 7));
    clipped_blit(uc, dst, src, sx, sy, w, h, dx, dy, alpha);
    uc.ret(0);
}

pub fn draw_img_clip_ex(uc: &mut Emu) {
    blit8(uc, false);
}

pub fn draw_img_clip_alpha_ex(uc: &mut Emu) {
    blit8(uc, true);
}

fn blit7(uc: &mut Emu, alpha: bool) {
    let screen = uc.get_data().rt.gfx.img;
    let src = uc.arg(0);
    let (sx, sy, w, h, dx, dy) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4), a16(uc, 5), a16(uc, 6));
    clipped_blit(uc, screen, src, sx, sy, w, h, dx, dy, alpha);
    uc.ret(0);
}

pub fn draw_img_clip(uc: &mut Emu) {
    blit7(uc, false);
}

pub fn draw_img_clip_alpha(uc: &mut Emu) {
    blit7(uc, true);
}

fn full_screen(uc: &mut Emu, img: u32) {
    let (w, h) = gfx::img_wh(uc, img);
    let (w, h) = (s16(w as i32), s16(h as i32));
    let (cw, ch) = (w.min(SCREEN_W).max(0), h.min(SCREEN_H).max(0));
    uc.get_data_mut().rt.oldlib_clip = [0, 0, cw, ch];
    let n = (2 * cw * ch) as usize;
    if n != 0 {
        let data = uc.r32(img);
        if let Ok(v) = uc.mem_read_as_vec(data as u64, n) {
            let buf = uc.get_data().rt.gfx.buf;
            uc.write(buf, &v);
        }
    }
}

pub fn draw_full_screen(uc: &mut Emu) {
    let img = uc.arg(0);
    full_screen(uc, img);
    uc.ret(0);
}

pub fn get_lcd_buffer(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.buf;
    uc.ret(v);
}

pub fn fill_rect(uc: &mut Emu) {
    let (x, y, w, h) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4));
    let color = uc.arg(5) as u16;
    if x <= SCREEN_W && y <= SCREEN_H && x + w >= 0 && y + h >= 0 {
        let (r, b) = (s16(x + w - 1), s16(y + h - 1));
        if r > x && b > y {
            let (l, t) = (x & 0xFFFF, y & 0xFFFF);
            if l <= 0 && t <= 0 {
                gfx::maybe_adopt(uc, (r - x + 1).max(0) as u32, (b - y + 1).max(0) as u32);
            }
            gfx::fill_rect(uc, l, t, r - x + 1, b - y + 1, color);
        }
    }
    uc.ret(0);
}

#[allow(clippy::too_many_arguments)]
fn line_ex(uc: &mut Emu, img: u32, mut x1: i32, mut y1: i32, mut x2: i32, mut y2: i32, color: u32) {
    let (base, stride) = if img != 0 {
        (uc.r32(img), s16(gfx::img_wh(uc, img).0 as i32))
    } else {
        (uc.get_data().rt.gfx.buf, SCREEN_W)
    };
    let [x0c, y0c, x1c, y1c] = uc.get_data().rt.oldlib_clip;
    if x1 > x2 {
        std::mem::swap(&mut x1, &mut x2);
        std::mem::swap(&mut y1, &mut y2);
    }
    let dx = s16(x2 - x1);
    let mut dy = s16(y2 - y1);
    let mut step = 1;
    if dy < 0 {
        dy = -dy;
        step = -1;
    }
    let (two_dx, two_dy) = (2 * dx, 2 * dy);
    let c = color as u16;
    let pk = if uc.le() { c.to_le_bytes() } else { c.to_be_bytes() };
    let plot = |uc: &mut Emu, x: i32, y: i32| {
        if x0c <= x && x < x1c && y0c <= y && y < y1c {
            uc.write((base as i64 + 2 * y as i64 * stride as i64 + 2 * x as i64) as u32, &pk);
        }
    };
    let (mut x, mut y) = (x1, y1);
    if dx < dy {
        let mut e = s16(two_dx - dy);
        let mut n = dy;
        while n >= 0 {
            plot(uc, x, y);
            if e <= 0 {
                e = s16(e + two_dx);
            } else {
                x = s16(x + 1);
                e = s16(e + two_dx - two_dy);
            }
            n -= 1;
            y = s16(y + step);
        }
    } else {
        let mut e = s16(two_dy - dx);
        let mut n = dx;
        while n >= 0 {
            plot(uc, x, y);
            if e <= 0 {
                e = s16(e + two_dy);
            } else {
                y = s16(y + step);
                e = s16(e + two_dy - two_dx);
            }
            n -= 1;
            x = s16(x + 1);
        }
    }
}

pub fn draw_line_ex(uc: &mut Emu) {
    let img = uc.arg(0);
    let (x1, y1, x2, y2, c) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4), uc.arg(5));
    line_ex(uc, img, x1, y1, x2, y2, c);
    uc.ret(0);
}

const PIC_METHODS: &[(u32, &str, ApiFn)] = &[
    (0x18, "DF_PictureLibrary+0x18", pic_create_blank),
    (0x1c, "DF_PictureLibrary+0x1c", pic_load),
    (0x20, "DF_PictureLibrary+0x20", pic_width),
    (0x24, "DF_PictureLibrary+0x24", pic_height),
    (0x28, "DF_PictureLibrary+0x28", pic_fill_rect),
    (0x2c, "DF_PictureLibrary+0x2c", pic_line),
    (0x30, "DF_PictureLibrary+0x30", pic_full_screen),
    (0x34, "DF_PictureLibrary+0x34", pic_draw),
    (0x38, "DF_PictureLibrary+0x38", pic_draw_alpha),
    (0x3c, "DF_PictureLibrary+0x3c", pic_region),
    (0x40, "DF_PictureLibrary+0x40", pic_region_alpha),
    (0x4c, "DF_PictureLibrary+0x4c", pic_set_target),
    (0x50, "DF_PictureLibrary+0x50", pic_release),
];

pub fn init_picture_library(uc: &mut Emu) {
    let (lib, n) = (uc.arg(0), a16(uc, 1));
    let ids = alloc(uc, (2 * n).max(0) as u32, "piclib_ids");
    uc.w32(lib + 12, ids);
    let imgs = alloc(uc, (4 * n).max(0) as u32, "piclib_imgs");
    uc.w32(lib + 16, imgs);
    w16(uc, lib + 20, 0);
    w16(uc, lib + 8, n);
    for &(off, name, f) in PIC_METHODS {
        let t = runtime::method(uc, name, f);
        uc.w32(lib + off, t);
    }
    let line = alloc(uc, 480, "piclib_line");
    uc.w32(lib, line);
    uc.write(lib + 22, &[1]);
    uc.ret(1);
}

fn pic_img(uc: &Emu, lib: u32, idx: i32) -> u32 {
    uc.r32((uc.r32(lib + 16) as i64 + 4 * idx as i64) as u32)
}

fn pic_width(uc: &mut Emu) {
    let (lib, idx) = (uc.arg(0), a16(uc, 1));
    let img = pic_img(uc, lib, idx);
    let v = gfx::img_wh(uc, img).0 & 0xFFFF;
    uc.ret(v);
}

fn pic_height(uc: &mut Emu) {
    let (lib, idx) = (uc.arg(0), a16(uc, 1));
    let img = pic_img(uc, lib, idx);
    let v = gfx::img_wh(uc, img).1 & 0xFFFF;
    uc.ret(v);
}

fn pic_fill_rect(uc: &mut Emu) {
    let lib = uc.arg(0);
    let (x, y, w, h) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4));
    let color = uc.arg(5) as u16;
    let line = uc.r32(lib);
    if w > 0 && line != 0 {
        let pk = if uc.le() { color.to_le_bytes() } else { color.to_be_bytes() };
        let row: Vec<u8> = pk.iter().copied().cycle().take(2 * w as usize).collect();
        uc.write(line, &row);
    }
    if uc.get_data().rt.piclib_tmpimg == 0 {
        let n = gfx::header_size(uc);
        let t = alloc(uc, n, "piclib_tmpimg");
        uc.get_data_mut().rt.piclib_tmpimg = t;
    }
    let tmp = uc.get_data().rt.piclib_tmpimg;
    uc.w32(tmp, line);
    gfx::set_img_wh(uc, tmp, w as u32 & 0xFFFF, 1, 0);
    let target = uc.r32(lib + 4);
    let dst = if target != 0 { target } else { uc.get_data().rt.gfx.img };
    for i in 0..h.max(0) {
        clipped_blit(uc, dst, tmp, 0, 0, w, 1, x, s16(y + i), false);
    }
    uc.ret(0);
}

fn pic_line(uc: &mut Emu) {
    let lib = uc.arg(0);
    let img = uc.r32(lib + 4);
    let (x1, y1, x2, y2, c) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4), uc.arg(5));
    line_ex(uc, img, x1, y1, x2, y2, c);
    uc.ret(0);
}

fn pic_full_screen(uc: &mut Emu) {
    let (lib, idx) = (uc.arg(0), a16(uc, 1));
    if uc.r32(lib + 4) == 0 {
        let img = pic_img(uc, lib, idx);
        full_screen(uc, img);
    }
    uc.ret(0);
}

fn pic_draw_impl(uc: &mut Emu, alpha: bool) {
    let (lib, idx, x, y) = (uc.arg(0), a16(uc, 1), a16(uc, 2), a16(uc, 3));
    let img = pic_img(uc, lib, idx);
    let (w, h) = gfx::img_wh(uc, img);
    let (w, h) = (w as i32, h as i32);
    let target = uc.r32(lib + 4);
    let dst = if target != 0 { target } else { uc.get_data().rt.gfx.img };
    clipped_blit(uc, dst, img, 0, 0, w, h, x, y, alpha);
    uc.ret(0);
}

fn pic_draw(uc: &mut Emu) {
    pic_draw_impl(uc, false);
}

fn pic_draw_alpha(uc: &mut Emu) {
    pic_draw_impl(uc, true);
}

fn pic_region_impl(uc: &mut Emu, alpha: bool) {
    let (lib, idx) = (uc.arg(0), a16(uc, 1));
    let (dx, dy, sx, sy, w, h) = (a16(uc, 2), a16(uc, 3), a16(uc, 4), a16(uc, 5), a16(uc, 6), a16(uc, 7));
    let target = uc.r32(lib + 4);
    let dst = if target != 0 { target } else { uc.get_data().rt.gfx.img };
    let img = pic_img(uc, lib, idx);
    clipped_blit(uc, dst, img, sx, sy, w, h, dx, dy, alpha);
    uc.ret(0);
}

fn pic_region(uc: &mut Emu) {
    pic_region_impl(uc, false);
}

fn pic_region_alpha(uc: &mut Emu) {
    pic_region_impl(uc, true);
}

fn pic_set_target(uc: &mut Emu) {
    let (lib, img) = (uc.arg(0), uc.arg(1));
    uc.w32(lib + 4, img);
    let (w, h) = if img != 0 {
        let (w, h) = gfx::img_wh(uc, img);
        (s16(w as i32), s16(h as i32))
    } else {
        (SCREEN_W, SCREEN_H)
    };
    check_clip_to(uc, s16(w), s16(h));
    uc.ret(w as u32 & 0xFFFF);
}

fn noop(_uc: &mut Emu) {}

pub fn init_repaint_panel(uc: &mut Emu) {
    let p = uc.arg(0);
    let (xy, wh, ud_logic, ud_paint, n) = (uc.arg(1), uc.arg(2), uc.arg(3), uc.arg(4), uc.arg(5));
    let table = if n != 0 { alloc(uc, 4 * n, "panel_dirty") } else { 0 };
    uc.w32(p + 12, table);
    for i in 0..n {
        let e = alloc(uc, 8, "panel_rect");
        uc.w32(table + 4 * i, e);
    }
    uc.w32(p + 4, 0);
    uc.w32(p + 8, n);
    uc.w32(p + 24, xy);
    uc.w32(p + 28, wh);
    uc.w32(p + 16, ud_logic);
    uc.w32(p + 20, ud_paint);
    let slots: [(u32, &'static str, ApiFn); 7] = [
        (40, "DF_Windows+0x28", panel_add_child),
        (44, "DF_Windows.nullLogic", noop),
        (64, "DF_Windows.nullEvent", noop),
        (48, "DF_Windows.nullPaint", noop),
        (56, "DF_Windows+0x38", panel_repaint),
        (52, "DF_Windows+0x34", panel_update),
        (68, "DF_Windows+0x44", panel_event),
    ];
    for (off, name, f) in slots {
        let t = runtime::method(uc, name, f);
        uc.w32(p + off, t);
    }
    uc.w32(p + 32, 0);
    let t = runtime::method(uc, "DF_Windows+0x3c", panel_invalidate);
    uc.w32(p + 60, t);
    uc.w32(p + 36, 0);
    invalidate(uc, p, 4, p + 24);
    uc.ret(0);
}

fn last_sibling(uc: &Emu, mut p: u32) -> u32 {
    while uc.r32(p + 32) != 0 {
        p = uc.r32(p + 32);
    }
    p
}

fn panel_add_child(uc: &mut Emu) {
    let (p, child, wh) = (uc.arg(0), uc.arg(1), uc.arg(2));
    if wh == 0 {
        let t = last_sibling(uc, p);
        uc.w32(t + 32, child);
    } else if wh == 1 {
        if uc.r32(p + 36) == 0 {
            uc.w32(p + 36, child);
        } else {
            let t = last_sibling(uc, uc.r32(p + 36));
            uc.w32(t + 32, child);
        }
    }
}

fn add_dirty(uc: &mut Emu, p: u32, rect: u32) {
    let (n, cap) = (uc.r32(p + 4), uc.r32(p + 8));
    if n < cap {
        let e = uc.r32(uc.r32(p + 12) + 4 * n);
        if let Ok(v) = uc.mem_read_as_vec(rect as u64, 8) {
            uc.write(e, &v);
        }
        uc.w32(p + 4, n + 1);
    }
}

fn invalidate(uc: &mut Emu, mut p: u32, kind: u32, rect: u32) {
    loop {
        let (mut x, mut y, mut w, mut h) = (rs16(uc, rect), rs16(uc, rect + 2), rs16(uc, rect + 4), rs16(uc, rect + 6));
        if x + w < 0 || x > SCREEN_W || y + h < 0 || y > SCREEN_H {
            x = 0;
            y = 0;
            w = 0;
            h = 0;
        }
        if x < 0 {
            w = s16(w + x);
            x = 0;
        }
        if x + w > SCREEN_W {
            w = SCREEN_W - x;
        }
        if y + h > SCREEN_H {
            h = SCREEN_H - y;
        }
        for (i, v) in [x, y, w, h].into_iter().enumerate() {
            w16(uc, rect + 2 * i as u32, v);
        }
        if kind != 2 {
            break;
        }
        add_dirty(uc, p, rect);
        let sib = uc.r32(p + 32);
        if sib != 0 {
            invalidate(uc, sib, 2, rect);
        }
        p = uc.r32(p + 36);
        if p == 0 {
            return;
        }
    }
    if kind == 4 {
        add_dirty(uc, p, rect);
    }
}

pub fn panel_invalidate(uc: &mut Emu) {
    let (p, kind, rect) = (uc.arg(0), a16(uc, 1), uc.arg(2));
    invalidate(uc, p, kind as u32, rect);
}

fn then_children(uc: &mut Emu, p: u32, method_off: u32, extra: Vec<u32>) -> Co {
    let child = uc.r32(p + 36);
    if child != 0 {
        let f = uc.r32(p + method_off);
        let mut args = vec![child];
        args.extend(extra.iter().copied());
        return Co::Call(f, args, Box::new(move |uc, _| then_sibling(uc, p, method_off, extra)));
    }
    then_sibling(uc, p, method_off, extra)
}

fn then_sibling(uc: &mut Emu, p: u32, method_off: u32, extra: Vec<u32>) -> Co {
    let sib = uc.r32(p + 32);
    if sib != 0 {
        let f = uc.r32(p + method_off);
        let mut args = vec![sib];
        args.extend(extra);
        return Co::Call(f, args, Box::new(|_, _| Co::Done(0)));
    }
    Co::Done(0)
}

fn panel_update(uc: &mut Emu) {
    let p = uc.arg(0);
    let (f, ud) = (uc.r32(p + 44), uc.r32(p + 16));
    let co = Co::Call(f, vec![ud], Box::new(move |uc, _| then_children(uc, p, 52, vec![])));
    runtime::cocall(uc, co);
}

fn repaint_from(uc: &mut Emu, p: u32, i: u32) -> Co {
    if uc.r32(p + 4) > i {
        let e = uc.r32(uc.r32(p + 12) + 4 * i);
        let (x, y, w, h) = (rs16(uc, e), rs16(uc, e + 2), rs16(uc, e + 4), rs16(uc, e + 6));
        uc.get_data_mut().rt.oldlib_clip = [x, y, s16(x + w), s16(y + h)];
        let (f, ud) = (uc.r32(p + 48), uc.r32(p + 20));
        return Co::Call(f, vec![ud], Box::new(move |uc, _| repaint_from(uc, p, i + 1)));
    }
    uc.w32(p + 4, 0);
    then_children(uc, p, 56, vec![])
}

fn panel_repaint(uc: &mut Emu) {
    let p = uc.arg(0);
    let co = repaint_from(uc, p, 0);
    runtime::cocall(uc, co);
}

fn panel_event(uc: &mut Emu) {
    let (p, a, b) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let (f, ud) = (uc.r32(p + 64), uc.r32(p + 16));
    let co = Co::Call(f, vec![ud], Box::new(move |uc, _| then_children(uc, p, 68, vec![a, b])));
    runtime::cocall(uc, co);
}

fn cdiv(a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

fn cmod(a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a % b
    }
}

fn free_field(uc: &mut Emu, a: u32) {
    let p = uc.r32(a);
    if p != 0 {
        uc.get_data_mut().heap.free(p);
        uc.w32(a, 0);
    }
}

fn pic_create_blank(uc: &mut Emu) {
    let (lib, w, h) = (uc.arg(0), uc.arg(1) as i32, uc.arg(2) as i32);
    let r = 4 - cmod(w, 4);
    let pad = r - cdiv(r, 4) * 4;
    let (n, cap) = (rs16(uc, lib + 20), rs16(uc, lib + 8));
    if n >= cap {
        uc.ret(0xFFFF_FFFF);
        return;
    }
    let hn = gfx::header_size(uc);
    let img = alloc(uc, hn, "piclib_img");
    let slot = (uc.r32(lib + 16) as i64 + 4 * n as i64) as u32;
    uc.w32(slot, img);
    let size = (2 * (w + pad) * h) as u32;
    let data = if size != 0 {
        uc.get_data_mut().heap.alloc(size, "bigmem", false).unwrap_or(0)
    } else {
        0
    };
    if data != 0 {
        fill(uc, data, 0, size);
    }
    uc.w32(img, data);
    gfx::set_img_wh(uc, img, w as u32 & 0xFFFF, h as u32 & 0xFFFF, 1);
    let ids = uc.r32(lib + 12);
    w16(uc, (ids as i64 + 2 * n as i64) as u32, -1);
    w16(uc, lib + 20, n + 1);
    uc.ret(n as u32);
}

fn pic_load(uc: &mut Emu) {
    let lib = uc.arg(0);
    let raw = uc.cstr(uc.arg(1), 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    let rid = crate::api::dfpkg::res_index(uc, &name).map(|i| i as i32).unwrap_or(-1);
    let (n, cap) = (rs16(uc, lib + 20), rs16(uc, lib + 8));
    if rid < 0 || n >= cap {
        uc.ret(0xFFFF_FFFF);
        return;
    }
    let ids = uc.r32(lib + 12);
    for i in 0..n.max(0) {
        if rs16(uc, ids + 2 * i as u32) == s16(rid) {
            uc.ret(i as u32);
            return;
        }
    }
    let hn = gfx::header_size(uc);
    let img = alloc(uc, hn, "piclib_img");
    let slot = (uc.r32(lib + 16) as i64 + 4 * n as i64) as u32;
    uc.w32(slot, img);
    let q = crate::api::dfpkg::res_ptr(uc, rid as usize);
    uc.setreg(0, q);
    uc.setreg(1, img);
    crate::api::gfx_api::img_from_stream(uc);
    w16(uc, (ids as i64 + 2 * n as i64) as u32, rid);
    w16(uc, lib + 20, n + 1);
    uc.ret(n as u32);
}

fn pic_release(uc: &mut Emu) {
    let lib = uc.arg(0);
    let flag = uc.r8(lib + 22);
    if flag != 1 {
        uc.ret(flag as u32);
        return;
    }
    let imgs = uc.r32(lib + 16);
    for i in 0..rs16(uc, lib + 20).max(0) as u32 {
        let img = uc.r32(imgs + 4 * i);
        uc.setreg(0, img);
        crate::api::gfx_api::release_image(uc);
        free_field(uc, imgs + 4 * i);
    }
    free_field(uc, lib + 12);
    free_field(uc, lib + 16);
    free_field(uc, lib);
    uc.write(lib + 22, &[0]);
    uc.ret(0);
}

fn font_w(uc: &Emu, full: bool) -> i32 {
    let f = &uc.get_data().rt.font;
    (if full { f.hw } else { f.aw }) as i32
}

fn font_h(uc: &Emu) -> i32 {
    uc.get_data().rt.font.hh as i32
}

pub fn get_font_width(uc: &mut Emu) {
    let v = font_w(uc, true);
    uc.ret(v as u32);
}

pub fn get_font_width_char(uc: &mut Emu) {
    let v = font_w(uc, false);
    uc.ret(v as u32);
}

pub fn get_font_height(uc: &mut Emu) {
    let v = font_h(uc);
    uc.ret(v as u32);
}

fn str_width(uc: &mut Emu, s: &[u8]) -> i32 {
    (crate::api::text::text_width(uc, s) & 0xFFFF) as i32
}

fn bounded(uc: &Emu, p: u32, n: i32) -> Vec<u8> {
    let n = n.clamp(0, 199) as usize;
    if n == 0 || p == 0 {
        return Vec::new();
    }
    let raw = uc.mem_read_as_vec(p as u64, n).unwrap_or_default();
    match raw.iter().position(|&c| c == 0) {
        Some(z) => raw[..z].to_vec(),
        None => raw,
    }
}

fn rgb888_to_565(c: u32) -> u16 {
    ((((c >> 16) & 0xF8) << 8) + 8 * ((c >> 8) & 0xFC) + ((c & 0xFF) >> 3)) as u16
}

#[allow(clippy::too_many_arguments)]
fn draw_string_ex_impl(uc: &mut Emu, img: u32, s: u32, n: i32, x: u32, y: u32, rgb: u32) {
    let bytes = bounded(uc, s, n);
    crate::api::text::draw_bytes(uc, &bytes, s16((x & 0xFFFF) as i32), s16((y & 0xFFFF) as i32), rgb888_to_565(rgb), img);
}

pub fn draw_string_ex(uc: &mut Emu) {
    let (img, s, n, x, y, c) = (uc.arg(0), uc.arg(1), a16(uc, 2), uc.arg(3), uc.arg(4), uc.arg(5));
    draw_string_ex_impl(uc, img, s, n, x, y, c);
    uc.ret(0);
}

pub fn draw_string(uc: &mut Emu) {
    let screen = uc.get_data().rt.gfx.img;
    let (s, n, x, y, c) = (uc.arg(0), a16(uc, 1), uc.arg(2), uc.arg(3), uc.arg(4));
    draw_string_ex_impl(uc, screen, s, n, x, y, c);
    uc.ret(0);
}

fn rs8(uc: &Emu, a: u32) -> i32 {
    uc.r8(a) as i8 as i32
}

pub fn init_text_box(uc: &mut Emu) {
    let tb = uc.arg(0);
    for (i, off) in [20u32, 22, 24, 26].into_iter().enumerate() {
        let v = uc.arg(2 + i as u32) as i32;
        w16(uc, tb + off, v);
    }
    let t = runtime::method(uc, "TextBox+0x20", tb_set_line_height);
    uc.w32(tb + 32, t);
    uc.w32(tb, 0);
    let rest: [(u32, &'static str, ApiFn); 6] = [
        (36, "TextBox+0x24", tb_set_text),
        (40, "TextBox+0x28", tb_draw),
        (48, "TextBox+0x30", tb_release),
        (28, "TextBox+0x1c", tb_set_rect),
        (44, "TextBox+0x2c", tb_draw_ex),
        (52, "TextBox+0x34", tb_set_style),
    ];
    for (off, name, f) in rest {
        let t = runtime::method(uc, name, f);
        uc.w32(tb + off, t);
    }
    let v = font_h(uc) + 2;
    w16(uc, tb + 6, v);
    w16(uc, tb + 4, 0);
    uc.w32(tb + 12, 0);
    uc.w32(tb + 8, 0);
    uc.ret(v as u32);
}

fn tb_set_rect(uc: &mut Emu) {
    let tb = uc.arg(0);
    for (i, off) in [20u32, 22, 24, 26].into_iter().enumerate() {
        let v = uc.arg(1 + i as u32) as i32;
        w16(uc, tb + off, v);
    }
}

fn tb_set_line_height(uc: &mut Emu) {
    let (tb, v) = (uc.arg(0), uc.arg(1) as i32);
    w16(uc, tb + 6, v);
}

fn tb_set_style(uc: &mut Emu) {
    let (tb, v) = (uc.arg(0), uc.arg(1) as i32);
    w16(uc, tb + 4, v);
}

fn tb_release(uc: &mut Emu) {
    let tb = uc.arg(0);
    free_field(uc, tb + 12);
    free_field(uc, tb + 8);
}

fn tb_set_text(uc: &mut Emu) {
    let (tb, text) = (uc.arg(0), uc.arg(1));
    let (w, h) = (rs16(uc, tb + 24), rs16(uc, tb + 26));
    if !(font_w(uc, true) <= w && font_h(uc) <= h + 2 && text != 0) {
        uc.ret(0);
        return;
    }
    let mut raw = uc.read_upto(text, 0x10000);
    if let Some(z) = raw.iter().position(|&c| c == 0) {
        raw.truncate(z + 1);
    }
    let ch = |i: i32| -> u8 { *raw.get(i as usize).unwrap_or(&0) };
    let seg_width = |uc: &mut Emu, start: i32, stop: i32| -> i32 {
        let b = bounded(uc, text + start as u32, stop - start);
        s16(str_width(uc, &b))
    };
    uc.w32(tb, text);
    let (mut lines, mut last, mut i, mut start) = (0i32, 0i32, 0i32, 0i32);
    while ch(i) != 0 {
        if ch(i) == 10 {
            last = 0;
            while ch(i) == 10 {
                i += 1;
            }
            lines += 1;
            if ch(i) == 0 {
                lines += 1;
            }
            start = i;
        } else {
            let n = if ch(i) & 0x80 != 0 { 2 } else { 1 };
            last = seg_width(uc, start, i + n);
            if last > w {
                lines += 1;
                start = i;
            }
            i += n;
        }
    }
    if last > 0 {
        lines += 1;
    }
    let lines = s16(lines);
    uc.write(tb + 16, &[lines as u8]);
    let per = cdiv(h, rs16(uc, tb + 6)) as i8 as i32;
    uc.write(tb + 17, &[per as u8]);
    let pages = cdiv(per + lines - 1, per);
    uc.write(tb + 18, &[pages as u8, 0]);
    free_field(uc, tb + 12);
    free_field(uc, tb + 8);
    let lens = alloc(uc, lines.max(0) as u32, "textbox_lens");
    uc.w32(tb + 12, lens);
    let starts = alloc(uc, (2 * lines).max(0) as u32, "textbox_starts");
    uc.w32(tb + 8, starts);
    let set_start = |uc: &mut Emu, k: i32, v: i32| w16(uc, starts + 2 * k as u32, v);
    let set_len = |uc: &mut Emu, k: i32, v: i32| uc.write(lens + k as u32, &[v as u8]);
    set_start(uc, 0, 0);
    let (mut line, mut last, mut i, mut start) = (0i32, 0i32, 0i32, 0i32);
    while ch(i) != 0 {
        if ch(i) == 10 {
            last = 0;
            let st = uc.r16(starts + 2 * line as u32) as i32;
            set_len(uc, line, i - st);
            while ch(i) == 10 {
                i += 1;
            }
            line += 1;
            set_start(uc, line, i);
            start = i;
        } else {
            let n = if ch(i) & 0x80 != 0 { 2 } else { 1 };
            last = seg_width(uc, start, i + n);
            if last > w {
                start = i;
                let st = uc.r16(starts + 2 * line as u32) as i32;
                set_len(uc, line, i - st);
                line += 1;
                set_start(uc, line, i);
            }
            i += n;
        }
    }
    if last > 0 {
        let st = uc.r16(starts + 2 * line as u32) as i32;
        set_len(uc, line, i - st);
    }
    uc.ret(0);
}

fn tb_draw_impl(uc: &mut Emu, tb: u32, img: u32, rgb: u32) {
    let (x, mut y) = (rs16(uc, tb + 20), rs16(uc, tb + 22));
    let style = uc.r16(tb + 4);
    let (per, page, lines) = (rs8(uc, tb + 17), rs8(uc, tb + 19), rs8(uc, tb + 16));
    let first = page * per;
    let (mut xoff, mut yoff) = (0, 0);
    if style & 4 != 0 {
        let mut cnt = per;
        if per + first > lines {
            cnt = s16(lines - first);
        }
        yoff = s16(rs16(uc, tb + 26) - (font_h(uc) + 2) * cnt) >> 1;
    }
    let (text, starts, lens) = (uc.r32(tb), uc.r32(tb + 8), uc.r32(tb + 12));
    let mut i = first;
    while rs8(uc, tb + 17) + first > i {
        if rs8(uc, tb + 16) > i {
            let s = (text as i64 + rs16(uc, (starts as i64 + 2 * i as i64) as u32) as i64) as u32;
            let n = rs8(uc, (lens as i64 + i as i64) as u32);
            if style & 2 != 0 {
                let b = bounded(uc, s, n);
                let sw = str_width(uc, &b);
                let bw = rs16(uc, tb + 24);
                xoff = if sw >= bw { 0 } else { s16(cdiv(bw - sw, 2)) };
            }
            let dst = if img != 0 { img } else { uc.get_data().rt.gfx.img };
            draw_string_ex_impl(uc, dst, s, n, (x + xoff) as u32, (y + yoff) as u32, rgb);
            y = s16(uc.r16(tb + 6) as i32 + y);
        }
        i = s16(i + 1);
    }
}

fn tb_draw(uc: &mut Emu) {
    let (tb, c) = (uc.arg(0), uc.arg(1));
    tb_draw_impl(uc, tb, 0, c);
}

fn tb_draw_ex(uc: &mut Emu) {
    let (tb, img, c) = (uc.arg(0), uc.arg(1), uc.arg(2));
    tb_draw_impl(uc, tb, img, c);
}

fn and_clip(uc: &mut Emu, mut x: i32, mut y: i32, mut w: i32, mut h: i32) {
    let [x0, y0, x1, y1] = uc.get_data().rt.oldlib_clip;
    let c = if x + w >= x0 && y + h >= y0 && x1 >= x && y1 >= y {
        if x0 > x {
            w = s16(w - (x0 - x));
            x = x0;
        }
        if y0 > y {
            h = s16(h - (y0 - y));
            y = y0;
        }
        if x + w > x1 {
            w = s16(x1 - x);
        }
        if y + h > y1 {
            h = s16(y1 - y);
        }
        let (w, h) = (w.max(0), h.max(0));
        [x, y, s16(x + w), s16(y + h)]
    } else {
        [0; 4]
    };
    uc.get_data_mut().rt.oldlib_clip = c;
}

fn saved_clip(uc: &Emu) -> [i32; 4] {
    let [x0, y0, x1, y1] = uc.get_data().rt.oldlib_clip;
    [x0, y0, if x1 > x0 { x1 - x0 } else { 0 }, if y1 > y0 { y1 - y0 } else { 0 }]
}

fn restore_clip(uc: &mut Emu, s: [i32; 4]) {
    uc.get_data_mut().rt.oldlib_clip = [s[0], s[1], s16(s[0] + s[2]), s16(s[1] + s[3])];
}

#[allow(clippy::too_many_arguments)]
fn cell(uc: &mut Emu, img: u32, sx: i32, sy: i32, w: i32, h: i32, dx: i32, dy: i32) {
    let screen = uc.get_data().rt.gfx.img;
    clipped_blit(uc, screen, img, s16(sx), s16(sy), s16(w), s16(h), s16(dx), s16(dy), false);
}

pub fn draw_ui(uc: &mut Emu) {
    let img = uc.arg(0);
    let (x, y, w, h, n) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4), a16(uc, 5));
    let (iw, ih) = gfx::img_wh(uc, img);
    let (cw, ch) = (s16(cdiv((iw & 0xFFFF) as i32, n)), s16(cdiv((ih & 0xFFFF) as i32, n)));
    let (cols, rows) = (s16(cdiv(w + cw - 1, cw)), s16(cdiv(h + ch - 1, ch)));
    let saved = saved_clip(uc);
    and_clip(uc, x, y, w, h);
    let k = s16(n - 2);
    let (right, last) = (s16(x + w - cw), s16(cw * (k + 1)));
    let row = |uc: &mut Emu, sy: i32, dy: i32| {
        cell(uc, img, 0, sy, cw, ch, x, dy);
        for i in 1..cols {
            cell(uc, img, (cmod(i - 1, k) + 1) * cw, sy, cw, ch, i * cw + x, dy);
        }
        cell(uc, img, last, sy, cw, ch, right, dy);
    };
    row(uc, 0, y);
    for j in 1..rows {
        row(uc, (cmod(j - 1, k) + 1) * ch, j * ch + y);
    }
    row(uc, ch * (k + 1), y + h - ch);
    restore_clip(uc, saved);
    uc.ret(0);
}

pub fn draw_ui_horizontal(uc: &mut Emu) {
    let img = uc.arg(0);
    let (x, y, w) = (a16(uc, 1), a16(uc, 2), a16(uc, 3));
    let (iw, ih) = gfx::img_wh(uc, img);
    let (cw, ch) = (s16((iw & 0xFFFF) as i32 / 3), s16(ih as i32));
    let cols = s16(cdiv(w + cw - 1, cw));
    let saved = saved_clip(uc);
    and_clip(uc, x, y, w, ch);
    cell(uc, img, 0, 0, cw, ch, x, y);
    for i in 1..cols {
        cell(uc, img, cw, 0, cw, ch, i * cw + x, y);
    }
    cell(uc, img, 2 * cw, 0, cw, ch, x + w - cw, y);
    restore_clip(uc, saved);
    uc.ret(0);
}

pub fn draw_ui_single_repeat(uc: &mut Emu) {
    let img = uc.arg(0);
    let (x, y, w, h) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4));
    let (iw, ih) = gfx::img_wh(uc, img);
    let (iw, ih) = (s16(iw as i32), s16(ih as i32));
    let saved = saved_clip(uc);
    and_clip(uc, x, y, w, h);
    let (right, bottom) = (x + w, y + h);
    let mut yy = y;
    while ih > 0 && iw > 0 && bottom >= yy {
        let mut xx = x;
        while right >= xx {
            cell(uc, img, 0, 0, iw, ih, xx, yy);
            xx = s16(xx + iw);
        }
        yy = s16(yy + ih);
    }
    restore_clip(uc, saved);
    uc.ret(0);
}

pub fn draw_ui_four_x_repeat(uc: &mut Emu) {
    let img = uc.arg(0);
    let (x, y, w, h) = (a16(uc, 1), a16(uc, 2), a16(uc, 3), a16(uc, 4));
    let (iw, ih) = gfx::img_wh(uc, img);
    let (cw, ch) = (((iw & 0xFFFF) >> 1) as i32, s16(ih as i32));
    let saved = saved_clip(uc);
    and_clip(uc, x, y, w, h);
    let (mut r, mut yy) = (0, y);
    while ch > 0 && cw > 0 && yy < h {
        let (mut c, mut xx) = (0, x);
        while xx < w {
            cell(uc, img, if (c + r) & 1 != 0 { cw } else { 0 }, 0, cw, ch, xx, yy);
            xx = s16(xx + cw);
            c = s16(c + 1);
        }
        yy = s16(yy + ch);
        r = s16(r + 1);
    }
    restore_clip(uc, saved);
    uc.ret(0);
}

#[allow(clippy::too_many_arguments)]
fn draw_number_impl(uc: &mut Emu, dst: u32, img: u32, num: u32, cw: i32, ch: i32, gap: i32, x: i32, y: i32, align: u32) {
    let num = num as i32 as i64;
    let mut v = num.abs();
    let (mut digits, mut q) = (1i32, v);
    while q / 10 != 0 {
        q /= 10;
        digits += 1;
    }
    if num < 0 {
        digits += 1;
    }
    let total = s16(digits * cw + (digits - 1) * gap);
    let mut pos = match align {
        0 => s16(x + total - cw),
        1 => s16((total >> 1) + x - cw),
        2 => s16(x - cw),
        _ => 0,
    };
    let dst = if dst != 0 { dst } else { uc.get_data().rt.gfx.img };
    if v == 0 {
        clipped_blit(uc, dst, img, 0, 0, cw, ch, pos, y, true);
        return;
    }
    loop {
        let (q, rem) = (v / 10, v % 10);
        clipped_blit(uc, dst, img, s16(rem as i32 * cw), 0, cw, ch, pos, y, true);
        pos = s16(pos - (cw + gap));
        v = q;
        if v == 0 {
            break;
        }
    }
    if num < 0 {
        clipped_blit(uc, dst, img, s16(10 * cw), 0, cw, ch, pos, y, true);
    }
}

pub fn draw_image_number_ex(uc: &mut Emu) {
    let (dst, img, num) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let (cw, ch, gap, x, y, align) = (a16(uc, 3), a16(uc, 4), a16(uc, 5), a16(uc, 6), a16(uc, 7), uc.arg(8));
    draw_number_impl(uc, dst, img, num, cw, ch, gap, x, y, align);
    uc.ret(0);
}

pub fn draw_number(uc: &mut Emu) {
    let screen = uc.get_data().rt.gfx.img;
    let (img, num) = (uc.arg(0), uc.arg(1));
    let (cw, ch, gap, x, y, align) = (a16(uc, 2), a16(uc, 3), a16(uc, 4), a16(uc, 5), a16(uc, 6), uc.arg(7));
    draw_number_impl(uc, screen, img, num, cw, ch, gap, x, y, align);
    uc.ret(0);
}

pub fn refres_screen(uc: &mut Emu) {
    let [x0, y0, x1, y1] = uc.get_data().rt.oldlib_clip;
    if x1 - 1 > x0 && y1 - 1 > y0 {
        uc.get_data_mut().rt.frames += 1;
        uc.get_data_mut().rt.gfx.frames += 1;
    }
    uc.ret(0);
}

fn time_word(uc: &mut Emu, off: u32) {
    if uc.get_data().rt.host_bufs.get("oldlib_timebuf").is_none() {
        let b = alloc(uc, 24, "oldlib_time");
        uc.get_data_mut().rt.host_bufs.insert("oldlib_timebuf", b);
    }
    let buf = uc.get_data().rt.host_bufs["oldlib_timebuf"];
    uc.setreg(0, buf);
    crate::api::misc2::current_time(uc);
    let v = uc.r32(buf + off);
    uc.ret(v);
}

pub fn time_word8(uc: &mut Emu) {
    time_word(uc, 8);
}

pub fn time_word4(uc: &mut Emu) {
    time_word(uc, 4);
}

pub fn time_word0(uc: &mut Emu) {
    time_word(uc, 0);
}

pub fn lib_abs(uc: &mut Emu) {
    let v = uc.arg(0) as i32 as i64;
    uc.ret(s16(v.abs() as i32) as u32);
}

pub fn lib_max(uc: &mut Emu) {
    let (a, b) = (uc.arg(0) as i32, uc.arg(1) as i32);
    uc.ret(s16(if a <= b { b } else { a }) as u32);
}

pub fn lib_min(uc: &mut Emu) {
    let (a, b) = (uc.arg(0) as i32, uc.arg(1) as i32);
    uc.ret(s16(if a >= b { b } else { a }) as u32);
}

pub fn lib_random(uc: &mut Emu) {
    let (lo, hi) = (uc.arg(0) as i32, uc.arg(1) as i32);
    let st = uc.get_data().rt.rand_state;
    let st = (1103515245u64.wrapping_mul(st as u64).wrapping_add(12345) & 0x7FFF_FFFF) as u32;
    uc.get_data_mut().rt.rand_state = st;
    let rem = cmod(st as i32, hi.wrapping_sub(lo).wrapping_add(1));
    uc.ret(s16(s16(rem).abs() + lo) as u32);
}

pub fn init_df_actor(uc: &mut Emu) {
    let a = uc.arg(0);
    let (x, y) = (uc.arg(1) as i32, uc.arg(2) as i32);
    w16(uc, a, x);
    w16(uc, a + 2, y);
    let slots: [(u32, &'static str, ApiFn); 7] = [
        (16, "DF_Actor+0x10", actor_load),
        (20, "DF_Actor+0x14", actor_draw),
        (24, "DF_Actor+0x18", actor_draw_at),
        (28, "DF_Actor+0x1c", actor_next),
        (32, "DF_Actor+0x20", actor_set_action),
        (36, "DF_Actor+0x24", actor_last),
        (40, "DF_Actor+0x28", actor_collide),
    ];
    for (off, name, f) in slots {
        let t = runtime::method(uc, name, f);
        uc.w32(a + off, t);
    }
    for off in [6u32, 8, 10, 4] {
        w16(uc, a + off, 0);
    }
    uc.ret(a);
}

fn call_host(uc: &mut Emu, f: ApiFn, args: &[u32]) -> u32 {
    for (i, v) in args.iter().enumerate() {
        uc.setreg(i as u32, *v);
    }
    f(uc);
    uc.reg(0)
}

fn rint(uc: &Emu, buf: u32, pos: &mut u32) -> i32 {
    let b = uc.mem_read_as_vec((buf + *pos) as u64, 4).unwrap_or_else(|_| vec![0; 4]);
    *pos += 4;
    s16(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn rstr(uc: &mut Emu, buf: u32, pos: &mut u32) -> u32 {
    let p = uc.get_data_mut().heap.alloc(4, "df_pos", false).unwrap_or(0);
    uc.w32(p, *pos);
    let r = call_host(uc, crate::api::dfio::read_string, &[buf, p]);
    *pos = uc.r32(p);
    uc.get_data_mut().heap.free(p);
    r
}

fn actor_load(uc: &mut Emu) {
    let (a, lib, name) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let res = call_host(uc, crate::api::dfpkg::df_res_by_name, &[name]);
    let buf = call_host(uc, crate::api::dfio::get_stream_data, &[res]);
    let mut pos = 0u32;
    let head = alloc(uc, 20, "actor_head");
    uc.w32(a + 12, head);
    let n = rint(uc, buf, &mut pos);
    let imgs = if n > 0 {
        let p = alloc(uc, (2 * n) as u32, "actor_imgs");
        uc.w32(head, p);
        p
    } else {
        0
    };
    let co = actor_load_images(uc, a, lib, buf, pos, head, imgs, 0, n);
    runtime::cocall(uc, co);
}

#[allow(clippy::too_many_arguments)]
fn actor_load_images(uc: &mut Emu, a: u32, lib: u32, buf: u32, mut pos: u32, head: u32, imgs: u32, i: i32, n: i32) -> Co {
    if i < n {
        let s = rstr(uc, buf, &mut pos);
        let f = uc.r32(lib + 28);
        return Co::Call(
            f,
            vec![lib, s],
            Box::new(move |uc, idx| {
                w16(uc, imgs + 2 * i as u32, idx as i32);
                actor_load_images(uc, a, lib, buf, pos, head, imgs, i + 1, n)
            }),
        );
    }
    actor_load_rest(uc, lib, buf, pos, head);
    Co::Done(0)
}

fn actor_load_rest(uc: &mut Emu, lib: u32, buf: u32, mut pos: u32, head: u32) {
    let n = rint(uc, buf, &mut pos);
    if n > 0 {
        let frames = alloc(uc, (10 * n) as u32, "actor_frames");
        uc.w32(head + 4, frames);
        for i in 0..n as u32 {
            let f = frames + 10 * i;
            let x0 = rint(uc, buf, &mut pos);
            w16(uc, f, x0);
            let y0 = rint(uc, buf, &mut pos);
            w16(uc, f + 2, y0);
            let x1 = rint(uc, buf, &mut pos);
            w16(uc, f + 4, x1 - x0);
            let y1 = rint(uc, buf, &mut pos);
            w16(uc, f + 6, y1 - y0);
            let img = rint(uc, buf, &mut pos);
            w16(uc, f + 8, img);
        }
    }
    let n = rint(uc, buf, &mut pos);
    w16(uc, head + 8, n);
    if n > 0 {
        let acts = alloc(uc, (8 * n) as u32, "actor_acts");
        uc.w32(head + 12, acts);
        for i in 0..n as u32 {
            let act = acts + 8 * i;
            let nf = rint(uc, buf, &mut pos);
            w16(uc, act, nf);
            if nf <= 0 {
                continue;
            }
            let fl = alloc(uc, (8 * nf) as u32, "actor_act_frames");
            uc.w32(act + 4, fl);
            for j in 0..nf as u32 {
                let fr = fl + 8 * j;
                let dur = rint(uc, buf, &mut pos);
                w16(uc, fr, dur);
                let np = rint(uc, buf, &mut pos);
                w16(uc, fr + 2, np);
                if np <= 0 {
                    continue;
                }
                let parts = alloc(uc, (10 * np) as u32, "actor_parts");
                uc.w32(fr + 4, parts);
                for k in 0..np as u32 {
                    for m in 0..5u32 {
                        let v = rint(uc, buf, &mut pos);
                        w16(uc, parts + 10 * k + 2 * m, v);
                    }
                }
            }
        }
    }
    uc.w32(head + 16, lib);
    call_host(uc, crate::api::mem::free_big, &[buf]);
}

fn actor_frame(uc: &Emu, a: u32) -> u32 {
    let head = uc.r32(a + 12);
    let act = (uc.r32(head + 12) as i64 + 8 * rs16(uc, a + 6) as i64) as u32;
    (uc.r32(act + 4) as i64 + 8 * rs16(uc, a + 8) as i64) as u32
}

fn actor_draw_from(uc: &mut Emu, a: u32, ox: i32, oy: i32, i: i32) -> Co {
    let fr = actor_frame(uc, a);
    if rs16(uc, fr + 2) > i {
        let part = uc.r32(fr + 4) + 10 * i as u32;
        let head = uc.r32(a + 12);
        let f = (uc.r32(head + 4) as i64 + 10 * rs16(uc, part) as i64) as u32;
        let lib = uc.r32(head + 16);
        let img = rs16(uc, (uc.r32(head) as i64 + 2 * rs16(uc, f + 8) as i64) as u32);
        let args = vec![
            lib,
            img as u32,
            s16(rs16(uc, part + 2) + uc.r16(a) as i32 - ox) as u32,
            s16(rs16(uc, part + 4) + uc.r16(a + 2) as i32 - oy) as u32,
            rs16(uc, f) as u32,
            rs16(uc, f + 2) as u32,
            rs16(uc, f + 4) as u32,
            rs16(uc, f + 6) as u32,
            (uc.r16(part + 6) & 0xFF) as u32,
            0,
        ];
        let m = uc.r32(lib + 64);
        return Co::Call(m, args, Box::new(move |uc, _| actor_draw_from(uc, a, ox, oy, s16(i + 1))));
    }
    Co::Done(0)
}

fn actor_draw(uc: &mut Emu) {
    let a = uc.arg(0);
    let co = actor_draw_from(uc, a, 0, 0, 0);
    runtime::cocall(uc, co);
}

fn actor_draw_at(uc: &mut Emu) {
    let (a, ox, oy) = (uc.arg(0), a16(uc, 1), a16(uc, 2));
    let co = actor_draw_from(uc, a, ox, oy, 0);
    runtime::cocall(uc, co);
}

fn actor_duration(uc: &Emu, a: u32) -> i32 {
    let head = uc.r32(a + 12);
    let act = (uc.r32(head + 12) as i64 + 8 * rs16(uc, a + 6) as i64) as u32;
    let (frame, last) = (rs16(uc, a + 8), rs16(uc, act) - 1);
    let fl = uc.r32(act + 4) as i64;
    if last <= frame {
        rs16(uc, (fl + 8 * frame as i64) as u32)
    } else {
        rs16(uc, (fl + 8 * frame as i64 + 8) as u32)
    }
}

fn actor_next(uc: &mut Emu) {
    let a = uc.arg(0);
    let t = uc.r16(a + 10) as i32 + 1;
    w16(uc, a + 10, t);
    let mut r = actor_duration(uc, a);
    if r <= rs16(uc, a + 10) {
        r = s16(uc.r16(a + 8) as i32 + 1);
        w16(uc, a + 8, r);
        let head = uc.r32(a + 12);
        let act = (uc.r32(head + 12) as i64 + 8 * rs16(uc, a + 6) as i64) as u32;
        if rs16(uc, act) <= r {
            w16(uc, a + 8, 0);
            w16(uc, a + 10, 0);
            r = 0;
        }
    }
    uc.ret(r as u32);
}

fn actor_set_action(uc: &mut Emu) {
    let (a, act) = (uc.arg(0), uc.arg(1) as i32);
    let head = uc.r32(a + 12);
    if rs16(uc, a + 6) != act && rs16(uc, head + 8) > act {
        w16(uc, a + 6, act);
        w16(uc, a + 8, 0);
        w16(uc, a + 10, 0);
    }
    uc.ret(a);
}

fn actor_last(uc: &mut Emu) {
    let a = uc.arg(0);
    let head = uc.r32(a + 12);
    let act = (uc.r32(head + 12) as i64 + 8 * rs16(uc, a + 6) as i64) as u32;
    let v = rs16(uc, (uc.r32(act + 4) as i64 + 8 * rs16(uc, act) as i64 - 8) as u32);
    uc.ret(v as u32);
}

fn actor_collide(uc: &mut Emu) {
    let (a, b, ta, tb) = (uc.arg(0), uc.arg(1), uc.arg(2) as i32, uc.arg(3) as i32);
    let (fa, fb) = (actor_frame(uc, a), actor_frame(uc, b));
    let (ha, hb) = (uc.r32(a + 12), uc.r32(b + 12));
    for i in 0..rs16(uc, fa + 2).max(0) as u32 {
        let pa = uc.r32(fa + 4) + 10 * i;
        for j in 0..rs16(uc, fb + 2).max(0) as u32 {
            if rs16(uc, pa + 8) != ta {
                continue;
            }
            let pb = uc.r32(fb + 4) + 10 * j;
            if rs16(uc, pb + 8) != tb {
                continue;
            }
            let ra = (uc.r32(ha + 4) as i64 + 10 * rs16(uc, pa) as i64) as u32;
            let rb = (uc.r32(hb + 4) as i64 + 10 * rs16(uc, pb) as i64) as u32;
            let (ax, ay) = (rs16(uc, pa + 2) + rs16(uc, a), rs16(uc, pa + 4) + rs16(uc, a + 2));
            let (bx, by) = (rs16(uc, pb + 2) + rs16(uc, b), rs16(uc, pb + 4) + rs16(uc, b + 2));
            let (aw, ah) = (rs16(uc, ra + 4), rs16(uc, ra + 6));
            let (bw, bh) = (rs16(uc, rb + 4), rs16(uc, rb + 6));
            if ax + aw - 1 >= bx && bx + bw - 1 >= ax && ay + ah - 1 >= by && by + bh - 1 >= ay {
                uc.ret(1);
                return;
            }
        }
    }
    uc.ret(0);
}

fn lcd_forward(uc: &mut Emu, f: ApiFn, args: &[u32]) {
    for (i, v) in args.iter().take(4).enumerate() {
        uc.setreg(i as u32, *v);
    }
    let sp = uc.reg_read(unicorn_engine::RegisterARM::SP).unwrap_or(0) as u32;
    let extra = &args[args.len().min(4)..];
    let saved = if extra.is_empty() {
        Vec::new()
    } else {
        uc.mem_read_as_vec(sp as u64, 4 * extra.len()).unwrap_or_default()
    };
    for (i, v) in extra.iter().enumerate() {
        uc.w32(sp + 4 * i as u32, *v);
    }
    f(uc);
    if !extra.is_empty() {
        uc.write(sp, &saved);
    }
}

fn u16a(uc: &Emu, i: u32) -> u32 {
    uc.arg(i) & 0xFFFF
}

pub fn draw_vline(uc: &mut Emu) {
    let (x, y, y2, c) = (u16a(uc, 1), u16a(uc, 2), u16a(uc, 4), u16a(uc, 5));
    lcd_forward(uc, crate::api::misc2::draw_line_ex, &[x, y, x, y2, c]);
}

pub fn draw_hline(uc: &mut Emu) {
    let (x, y, x2, c) = (u16a(uc, 1), u16a(uc, 2), u16a(uc, 3), u16a(uc, 5));
    lcd_forward(uc, crate::api::misc2::draw_line_ex, &[x, y, x2, y, c]);
}

pub fn draw_line(uc: &mut Emu) {
    let a: Vec<u32> = (1..6).map(|i| u16a(uc, i)).collect();
    lcd_forward(uc, crate::api::misc2::draw_line_ex, &a);
}

pub fn draw_rect(uc: &mut Emu) {
    let a: Vec<u32> = (1..6).map(|i| u16a(uc, i)).collect();
    lcd_forward(uc, crate::api::misc2::draw_rect_ex, &a);
}

pub fn draw_string_rect_old(uc: &mut Emu) {
    let (s, x, y, w, h, c) = (uc.arg(1), u16a(uc, 2), u16a(uc, 3), uc.arg(4), uc.arg(5), uc.arg(6));
    let x2 = (font_w(uc, true) as u32).wrapping_add(w).wrapping_sub(1) & 0xFFFF;
    lcd_forward(uc, crate::api::text::draw_string_rect, &[s, x, y, x2, h, c]);
}

pub fn num_to_zhifei(uc: &mut Emu) {
    const KEYS: [&str; 11] = [
        "zhifei0", "zhifei1", "zhifei2", "zhifei3", "zhifei4", "zhifei5", "zhifei6", "zhifei7", "zhifei8", "zhifei9",
        "zhifei10",
    ];
    let n = uc.arg(0);
    let key = if (1..=10).contains(&n) { n as usize } else { 0 };
    if let Some(&p) = uc.get_data().rt.host_bufs.get(KEYS[key]) {
        uc.ret(p);
        return;
    }
    let text: Vec<u8> = match key {
        0 => vec![0xA3, 0xB0, 0],
        10 => vec![0xA3, 0xB1, 0xA3, 0xB0, 0],
        k => vec![0xA3, 0xB0 + k as u8, 0],
    };
    let p = alloc(uc, text.len() as u32, "NumToZhiFei");
    uc.write(p, &text);
    uc.get_data_mut().rt.host_bufs.insert(KEYS[key], p);
    uc.ret(p);
}
