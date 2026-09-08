# emu-core-rs

CoolBar `.cbe` 模块的模拟核心（Rust）。尼彩等国产功能机上的 CoolBar 游戏是
ARMv5TE 机器码 + 私有容器格式，原机与配套服务器均已停止运行；本仓库在宿主上
用 Unicorn 执行模块代码、以陷阱表实现宿主 API，使这些模块可在现代系统上运行。

macOS、Windows、Android 三个外壳跑的都是这一份代码。

## 特性

- 容器与资源解码：两种头（`magic` 4 / 8）、多包容器、图片（原始 RGB565 / GIF 变体 / PNG）、9 位距离 LZ77 脚本流
- 两代模块入口 ABI：新 SDK 的 screen 模型与老 SDK 的数字系统调用
- 大端（BE-32）模块；模块自声明屏幕尺寸时自动认领
- 约 280 个宿主 API，按固件符号名注册
- 输入整形集中在核心：按键边沿、短按锁存、触摸排队、长按自动重复、软键两段式
- 三种对外形态：C ABI 动态库、无界面 `engine` 进程、Python ctypes 绑定

## 环境要求

- Rust 1.75 及以上（edition 2021）
- CMake 与 Ninja（`unicorn-engine` 构建依赖）

```bash
cargo build --release
```

产物位于 `target/release/`：

| 产物 | 说明 |
|---|---|
| `libnieche.dylib` / `nieche.dll` / `libnieche.so` | C ABI 动态库，接口见 `emuffi/nieche.h` |
| `engine` | 无界面引擎进程，stdin 收命令、stdout 吐二进制帧 |
| `ffitest` / `bootcmp` / `layout` / `cbedump` | 与 Python 参照实现比对用 |

## 快速开始

### engine 进程

```bash
# 以 30fps 运行，stdout 输出二进制帧分组
./target/release/engine game.cbe --fps 30
```

stdout 分组（小端）：

| 标签 | 载荷 |
|---|---|
| `FRM0` | `u32` 帧号 + `u16` 宽 + `u16` 高 + `u32` 长度 + RGB565 帧缓冲 |
| `LOG0` | `u32` 长度 + UTF-8 文本 |
| `AUD0` | `u32` 长度 + UTF-8 JSON |
| `EXT0` | `u32` 长度 + `module` |

stdin 每行一个 JSON：`{"keys": 掩码}`、`{"touch": [x, y, "down|move|up"]}`、
`{"soft": "left|right"}`、`{"fps": 30}`、`{"quit": true}`。

### Python 绑定

```python
import nieche                    # python/nieche.py

s = nieche.NiecheSession("game.cbe").boot()
s.set_keys(1 << 0)               # 按键是位掩码
px = s.step()                    # 返回小端 RGB565 帧缓冲
w, h = s.size
s.stop()                         # 必须调用：模块在 AppStop 里落存档
```

### C ABI

```c
#include "nieche.h"

NiecheSession *s = nieche_open("game.cbe");
nieche_boot(s);
size_t n = nieche_step(s, buf, sizeof buf);   /* 小端 RGB565 */
nieche_close(s);
```

## 命令行参数

`engine <module.cbe> [选项]`

| 参数 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `--fps` | 整数 | `30` | 目标帧率，取值夹到 1–240 |
| `--vclock` | 开关 | 关 | 长按连发改用「帧号 / fps」计时，供逐帧比对使用 |

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `NIECHE_HOME` | `~/.nieche-emu` | 数据根：存档与模块虚拟文件系统 |
| `NIECHE_LIB` | 自动查找 | `nieche.py` 加载的动态库路径 |
| `NIECHE_FSBASE` | `assets/fatfs` | 虚拟文件系统的只读底层 |

## 帧率与游戏速度

模块的动画与计时按帧推进，每帧虚拟时钟前进 `frame_ms`（默认 40ms，
`VmSetFPS` 可改）。外壳的实际帧率直接决定游戏快慢。原机运行这些模块约
10–15 fps，本核心通常远高于此，外壳需要限制帧率。

## 测试

```bash
cargo test --release
```

逐帧差分工具在 [emu-tools](https://github.com/nieche-cbe-emu/emu-tools)，
参照实现是 [emu-core-py](https://github.com/nieche-cbe-emu/emu-core-py)。
29 个模块语料上：60 帧完全一致 29/29，300 帧 28/29。

## 说明

本仓库只包含代码。游戏数据、手机固件与真机文件系统不在此处，也不提供。
