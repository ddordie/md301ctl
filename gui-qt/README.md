# md301ctl 的 Qt 前端（实验性）

用 **PySide6 + QML** 写的界面壳，替换原来那个零依赖 Win32 GUI 的观感。
目标是在不重写协议的前提下，把界面做成可以照设计稿（FigCraft 原型）复刻的样子。

## 设计原则

1. **协议只有一份实现。** 所有读写都走既有的 Rust 程序 `md301ctl.exe`，
   本目录只负责调用、解析 `--json`、把结果画出来。前端里**没有任何 HID 报文**。
2. **安全规则不重写。** 射频休眠守门、只在值变化时写入、范围校验，都在 Rust 侧；
   Qt 这层只做三件事：启动/刷新**只读**、开下拉**不写**、值没变**不写**。
3. **颜色/间距只有一个出处。** 全部在 `qml/theme/Theme.qml`；照原型换 UI 时先换它。
4. **不猜契约。** 选项列表（DPI 范围、回报率档位、按键响应档位、156 个功能码、
   按键名）全部来自 `md301ctl spec --json`。

## 前置

- Windows，Python 3.11（`uv venv` 或 `python -m venv` 都行）
- `PySide6-Essentials`（见 `requirements.txt`）
- 已构建的 `md301ctl.exe`：放在 `../target/release/`、或本目录、或桌面，
  或用环境变量 `MD301CTL` 指定绝对路径

## 运行

```bat
run.bat            :: 首次会自动建 .venv 并装依赖，然后起界面
```

或者手动：

```bash
uv venv --python 3.11 .venv
uv pip install --python .venv/Scripts/python.exe -r requirements.txt
.venv/Scripts/python.exe app.py
```

自检（**只读**，不写设备，可反复跑）：

```bash
.venv/Scripts/python.exe smoke.py
```

想看前端到底发了哪些命令（排查用）：

```bash
MD301_DEBUG=1 .venv/Scripts/python.exe app.py     # 每条命令打进 stderr
```

## 文件

| 文件 | 作用 |
| --- | --- |
| `app.py` | 入口：QML 引擎、中文默认字体、Fusion 样式 + 暗色调色板兜底 |
| `backend.py` | 后端桥：QProcess 串行执行 CLI、解析 JSON、属性/槽给 QML；含启动失败上报与 20s 看门狗 |
| `qml/Main.qml` | 主面板：设备状态 / DPI / 回报率·按键响应 / 按键映射 / 配置档 |
| `qml/theme/Theme.qml` | 设计 token（颜色、圆角、间距、字号）——**照原型改这里** |
| `qml/components/` | `Card`、`PillButton`、`StyledSlider`、`DarkCombo`（默认 Fusion 是白底，必须自己着色） |
| `smoke.py` | 无头自检：验证「界面桥 → CLI → 设备」这条链 |

## 数据契约（前端只依赖这两个命令）

```bash
md301ctl spec --json     # 静态规格，启动取一次，不碰设备
# { "caps": {...}, "sleep_times": [...], "buttons": [{name, code}...], "functions": [{code,label}...] }

md301ctl status --json   # 实时快照，一次调用一个对象
# { "device": {...}, "power": {...}, "dpi": {...}, "perf": {...}, "buttons": {...} }
```

写操作分别是 `dpi set <档> <值>`、`dpi switch <档>`、`perf polling <hz>`、
`perf response <0-5>`、`buttons map <按键名> <功能码>`、`profile set <档>`、
`perf save`、`profile reset-all --yes`。

## 怎么让它变成"你的设计"

FigCraft 画的原型 → 导出 → 我照它改：

1. **导出设计**：`.figcraft` 文件（就是普通归档，可以直接解出颜色/尺寸/文字），
   或者从 FigCraft 里导出 PNG（2x）/SVG；
2. **换 token**：颜色/圆角/间距/字号 → `qml/theme/Theme.qml`；
3. **重排布局**：`qml/Main.qml` 的卡片顺序与内部结构（组件都已经参数化）；
4. **自检 + 截图对照**：`smoke.py` 保证链路没坏，截图和原型逐屏比对。

## 已验证（真机 ASUS ProArt MD301）

- `smoke.py` 通过：读到固件 0.0.41、电量、DPI 三档、回报率、按键映射、156 个功能码；
- 界面启动与刷新**不发任何写命令**（`MD301_DEBUG=1` 日志可证）；
- 合成鼠标点击「应用 DPI」→ 日志出现 `dpi set 1/2/3` + 一次 `status` 刷新，
  设备值未被改动（写的是原值）；
- 设备射频休眠时，状态栏显示 CLI 报的原话（如「设备处于射频休眠…」），值不会被覆盖。

## 打包成单文件 exe（给不装 Python 的人用）

```bash
uv pip install --python .venv/Scripts/python.exe pyinstaller
.venv/Scripts/pyinstaller.exe --noconfirm --onefile --windowed --name md301ctl-qt \
  --add-data 'qml;qml' \
  --add-data 'assets;assets' \
  --icon assets/logo.ico \
  --add-binary '../target/release/md301ctl.exe;.' \
  --collect-data PySide6 \
  app.py
```

要点：

- `--add-data 'qml;qml'` 必须加，否则运行时找不到 QML；
- `--add-binary '...;.'` 把 Rust CLI 打进包里，`backend.py` 会优先用 `sys._MEIPASS`
  里那份，所以拷到任何机器上都能独立跑；
- 产物 `dist/md301ctl-qt.exe` 约 81 MB（Qt 运行时 + QML 模块占绝大部分），
  **首次启动约 8 秒**（单文件模式每次启动都要把包解到临时目录）；想要秒开就改成
  `--onedir`（启动 1–2 秒，但产物是一个文件夹）；
- 检查包里确实带着后端和 QML：
  `.venv/Scripts/pyi-archive_viewer.exe -l dist/md301ctl-qt.exe | grep -E 'md301ctl.exe|Main.qml'`；
- 未做代码签名，Windows 可能提示 SmartScreen（"更多信息 → 仍要运行"）。

## Logo / 图标

- `assets/logo.svg` 是**用户提供的应用图标**，由它生成：
  - `assets/logo.ico` —— 7 种尺寸（16/24/32/48/64/128/256），用于窗口图标与打包进 exe；
  - `assets/logo-256.png` —— 单张 256px，用作兜底。
- 重新生成（改了 SVG 之后）：
  `scripts/gen_assets.py assets/logo.svg assets/logo-256.png assets/logo.ico`
  （脚本在仓库根的 `scripts/`，用 QtSvg 渲染，ICO 容器自己拼，不需要 Pillow）
- 图标是白底方块，所以在暗色标题栏里放在**白色圆角徽章**中（`qml/Main.qml` 顶部）；
  QML 里用 `sourceSize: Qt.size(68, 68)` 做 2x 光栅化，边缘才不糊。
- **许可边界**：`assets/` 下的图标为用户提供，不在本项目 MIT 许可范围内；
  ProArt / ASUS 等名称与标识的权利归其所有者。

## 已知限制

- 一条 `status --json` 约 0.7–2 秒（设备 RF 链路本身慢），所以刷新时状态栏会短暂显示「读取中…」；
- 只做了主面板，没有做设置向导、按键宏、灯光等（这些官方页面也没有）；
- 用 PyInstaller 打成单个 exe 未做（要带 Qt 运行时，体积会大不少）。
