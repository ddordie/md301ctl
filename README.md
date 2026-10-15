# md301ctl —— 华硕 ProArt MD301 鼠标离线配置工具

不联网、不开浏览器，直接通过 HID 与鼠标（或 2.4G 接收器）通信，实现部分
`https://gearlink.asus.com.cn/zh-cn`（Gear Link 网页版）里对 MD301 的设备设置功能。

**不想自己编译**：到 [Releases](https://github.com/ddordie/md301ctl/releases)
下载 `md301ctl.exe`，双击即用（Windows x64，单文件零依赖，无需 Rust 工具链）。

协议是从 Gear Link 网页版的 JS 逆向出来的，并在真机上逐条读写验证过
（读到 `hidtrace5b.json` 级别的原始报文对比）。

界面有两套：默认的零依赖 Win32 窗口，以及 [gui-qt/](gui-qt/)
下用 PySide6 + QML 重写的版本

![Qt 界面](docs/screenshot.png)

*[gui-qt/](gui-qt/) 版界面（1280×720）：顶栏连接状态 + 配置档，鼠标插画四周 10 个按键映射，
右侧 DPI／回报率与按键响应／电源三张卡片。静态层由设计稿 SVG 生成，动态控件在 QML 里
按设计稿坐标摆放；自动刷新常开。*

## 编译

需要 Windows + MSVC 工具链（Rust stable）：

```bash
cd md301ctl
cargo build --release
# 产物: target/release/md301ctl.exe
```

**零第三方依赖**，不需要 crates.io 联网，也不需要 C 编译器（FFI 直接调用
`setupapi` / `hid.dll` / `kernel32`）。

> 在 git-bash 里编译若报 `LNK1104: cannot open file 'C:\c\Users\...'`，
> 是该 shell 的小写 `tmp` 环境变量被 MSYS 路径污染，用：
> `env -u tmp TMP='C:\Users\<你>\AppData\Local\Temp' cargo build --release`

## 图形界面

直接**双击 `md301ctl.exe`**，或运行 `md301ctl gui`，会开一个窗口：

- 顶栏：指示灯 + 「已连接 / 未连接」+ 配置档下拉 + 切换 + 电量（按电量变色，充电时显示闪电）
- 鼠标插画四周 10 个按键映射下拉：左键 / 右键 / 滚轮点击 / 侧键前进 / 侧键后退 / DPI 切换键 /
  向上滚动 / 向下滚动 / **向左滚动 / 向右滚动**（后两个是侧边滚轮的左右方向，走设备的扩展按键表）
- DPI 设定：3 档滑块 + 数值 + 「设为当前」（当前档用强调色，其按钮置灰）
- 回报率（125/250/500/1000 Hz）、按键响应（12/16/20/24/28/32 ms）
- 电源：休眠时间（1/2/3/5/10 分钟与「永不」）+ 低电量提醒阈值
- 应用按键映射 / 应用 DPI / 保存到 Flash / 恢复出厂（有二次确认）

## 命令行用法

```
md301ctl --help                 完整命令列表
md301ctl list                   列出检测到的 ASUS HID 接口
md301ctl status                 一次输出 info + 电量 + 性能 + 按键
```

常用操作：

```bash
md301ctl dpi                              # 看 4 档 DPI 与当前生效档
md301ctl dpi set 2 1600                   # 把第 2 档改成 1600（50 的倍数，官方上限 8000）
md301ctl perf response 5                  # 按键响应 32ms（0–5 = 12/16/20/24/28/32 ms）
md301ctl dpi switch 2                     # 立即切到第 2 档
md301ctl perf polling 1000                # 回报率 125/250/500/1000（MD301 只有这 4 档）
md301ctl buttons map 侧键后退 滚轮点击      # 改按键映射（第一个参数是设备上的按键，见 `buttons`
                                          # 输出的左键/右键/滚轮点击/侧键前进/侧键后退/DPI 切换键/
                                          # 向上滚动/向下滚动/向左滚动/向右滚动，或 0x1F0 代码；
                                          # 旧名字（中键/左侧后键/滚轮上滚…）仍然可用；
                                          # 第二个参数是功能码，见 `buttons names`）
md301ctl buttons names                    # 列出全部可用功能码
md301ctl profile set 2                    # 切换配置档
md301ctl battery                          # 电量/充电/休眠/低电量阈值
md301ctl power set --sleep 0-4|255 --low 0-100 3 --low 15     # 休眠 5 分钟，低电量提醒 15%
md301ctl perf save                        # 写入 Flash 永久保存（断电保持）
md301ctl monitor 30                       # 监听设备主动上报的事件
```

任何命令都可以加 `--json` 输出机器可读结果，方便脚本调用：

```bash
md301ctl battery --json
{"battery":63,"charging":false,"sleep_index":1,"low_power":20,...}
```

## MD301 实际支持的设置

以设备真机应答为准（网页端也是按同一套能力表显示对应面板）：

## 协议要点（逆向所得）

- 控制通道：HID 接口 **Usage Page `0xFF01` / Usage 1**，report ID `0`，
  写 **65 字节**（1 字节 report ID + 64 字节载荷），读 **64 字节**。
- 事件通道：**Usage Page `0xFFC1` / Usage 1**（仅输入）。
- 帧格式：`[命令, 键/子命令, 索引低, 索引高, 载荷...]`；应答按前 4 字节回显匹配。
- 命令：`0x12` 读、`0x51` 写、`0x50` 配置档/保存。
- 主要地址（键, 索引）：
  - 设备信息 `12 00 00`、按键映射 `12 05 00`、性能信息 `12 04 00`、
    分离 DPI `12 04 02`、电源 `12 07 00`
  - DPI 写入 `51 31 <档位-1>`、切换 `51 31 09`、回报率 `51 31 04`、
    按键映射写入 `51 21 00`、电源写入 `51 37 00`
  - 配置档切换 `50 02 <档位-1>`、保存到 Flash `50 03 00`、恢复出厂 `50 40 00`
- DPI 编码 = `DPI/50 - 1`（16 位小端），所以 DPI 必须是 50 的倍数。
- VID `0x0B05`，PID `0x1CD3`（2.4G 接收器）/ `0x1CD5`（有线）。

## 许可证

[MIT](LICENSE)。协议为独立逆向所得，与华硕（ASUS）无关联；本项目未包含任何
官方代码或二进制资源。
