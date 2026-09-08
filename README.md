# emu-core-rs

尼彩（Nieche）CoolBar `.cbe` 模块的模拟核心，Rust 实现。
**三端发布的产物就是它** —— macOS、Windows、安卓跑的都是这一份代码。

靠 Unicorn 跑 ARMv5TE 指令，宿主 API 用陷阱表在 Rust 侧实现。

```
cbelib/    .cbe 容器解析与资源解码（图片、脚本、多包）
emucore/   机器、运行时、约 280 个宿主 API、显示、音频、字库、Session
emuffi/    C ABI 导出层（libnieche）+ engine 二进制
python/    nieche.py —— ctypes 绑定，Windows 和安卓的外壳用它
```

## 构建

```
cargo build --release
```

产物：

- **`libnieche`** —— C ABI 动态库，接口见 `emuffi/nieche.h`
- **`engine`** —— 无界面引擎进程，stdin 收命令、stdout 吐二进制帧，macOS 外壳用它
- 若干比对用的二进制（`ffitest` / `bootcmp` / `layout` / `cbedump`）

安卓的 `.so` 用 [emu-tools](https://github.com/nieche-cbe-emu/emu-tools) 的
`tools/build-android.sh` 交叉编译，那个脚本里记了四个必踩的坑。

## Session 是冻结接口

`emucore/src/session.rs` 和 `python/nieche.py` 对外只有这些：
`boot / stop / step / set_keys / set_touch / soft_key / take_events / size`。

输入的语义——按键边沿判定、短按锁存、触摸排队（一帧只消化一个）、
长按自动重复、软键两段式——**全在核心里**。外壳不要自己再实现一遍，
两份实现迟早会各错一遍。

## 关于帧率

模块的动画和计时都是**按帧推进**的（每帧虚拟时钟走 `frame_ms`），
所以外壳跑多快，游戏就多快。真机上 MSW8533 跑这些游戏大概只有 10–15 fps，
而这个核心轻松跑满 60 —— **不压帧率游戏会快得没法玩**。三端外壳都有帧率控制。

## 与 Python 参照实现的关系

[emu-core-py](https://github.com/nieche-cbe-emu/emu-core-py) 是**判据**，
不是发布产物。这边的任何行为都必须逐帧对得上那边：画面、宿主调用序列、
调用次数三样都要一致。差分工具在
[emu-tools](https://github.com/nieche-cbe-emu/emu-tools)。

29 个模块语料上：60 帧逐帧差分 29/29 完全一致，300 帧 28/29。

## 说明

本仓库只有代码。游戏数据、手机固件和真机文件系统都不在这里，也不会提供。
