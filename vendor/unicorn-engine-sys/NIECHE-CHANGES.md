# Changes to unicorn-engine-sys 2.1.5

This directory is the crates.io package `unicorn-engine-sys` 2.1.5, modified by nieche-cbe-emu on 2026-10-23.

unicorn stubs `cpu_physical_memory_is_clean()` to always return true, so every TLB fill marks the page
TLB_NOTDIRTY and stores to it keep going through the slow `notdirty_write` path. The change marks a page
TLB_NOTDIRTY only when a translation block actually lives on it; self-modifying code is still detected.

Changed files:

- `qemu/accel/tcg/translate-all.c`: new function `uc_page_has_tb()`
- `qemu/accel/tcg/translate-all.h`: its declaration
- `qemu/accel/tcg/cputlb.c`: `tlb_set_page_with_attrs()` calls it instead of `cpu_physical_memory_is_clean()`
- `qemu/*.h` (per-architecture symbol renames): one `#define uc_page_has_tb uc_page_has_tb_<arch>` line each
