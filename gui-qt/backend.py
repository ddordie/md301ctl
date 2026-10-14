"""md301ctl 的 Qt(QML) 前端 —— 后端桥。

设计原则：
1. **协议只有一份实现**：所有读写都通过既有的 Rust 程序 `md301ctl.exe` 完成，
   本文件只负责调用它、解析 `--json`、把结果给 QML。绝不在这里再实现一遍 HID 协议。
2. **不猜契约**：选项列表（DPI 范围、回报率档位、按键响应档位、功能码表）来自
   `md301ctl spec --json`，前端不写死。
3. **安全规则由后端保证**：休眠守门、只在值变化时写入，都在 Rust 侧；本文件的职责是
   把失败信息（stderr）原样显示给用户，而不是假装成功。
4. **命令串行**：一次只跑一个子进程，避免同时读设备把设备打成休眠/应答错乱。
"""

from __future__ import annotations

import json
import os
import shutil
import sys
from typing import Any, Callable

from PySide6.QtCore import Property, QObject, QProcess, QTimer, Signal, Slot

# 兜底：找到 md301ctl.exe 的几个常见位置（仓库里、桌面、PATH）
_CANDIDATES = [
    os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "target", "release", "md301ctl.exe"),
    os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "md301ctl.exe"),
    os.path.expanduser("~/Desktop/md301ctl.exe"),
]


def find_cli() -> str | None:
    env = os.environ.get("MD301CTL")
    if env and os.path.isfile(env):
        return env
    for c in _CANDIDATES:
        if os.path.isfile(c):
            return c
    return shutil.which("md301ctl")


class Backend(QObject):
    """QML 可见的后端对象。"""

    changed = Signal()          # 任何状态变化（state/spec/status/error/busy）
    deviceEvent = Signal(str)   # 设备侧主动上报（暂未接事件通道）

    def __init__(self, cli: str | None = None, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self.cli = cli or find_cli()
        self._state: dict[str, Any] = {}
        self._spec: dict[str, Any] = {}
        self._status = "正在连接设备…"
        self._error = False
        self._busy = False
        self._queue: list[tuple[list[str], Callable[[int, str, str], None] | None]] = []
        self._proc: QProcess | None = None

        if not self.cli:
            self._status = "找不到 md301ctl.exe（设环境变量 MD301CTL 指向它，或先 cargo build --release）"
            self._error = True

    # ---------------- 属性（QML 读） ----------------
    def _get_state(self) -> dict:
        return self._state

    def _get_spec(self) -> dict:
        return self._spec

    @Property("QVariantMap", notify=changed)
    def state(self) -> dict:
        return self._state

    @Property("QVariantMap", notify=changed)
    def spec(self) -> dict:
        return self._spec

    @Property(str, notify=changed)
    def status(self) -> str:
        return self._status

    @Property(bool, notify=changed)
    def error(self) -> bool:
        return self._error

    @Property(bool, notify=changed)
    def busy(self) -> bool:
        return self._busy

    @Property(str, notify=changed)
    def cliPath(self) -> str:
        return self.cli or ""

    # ---------------- 命令调度 ----------------
    def _run(self, args: list[str], done: Callable[[int, str, str], None] | None = None) -> None:
        if os.environ.get("MD301_DEBUG"):
            print(f"[md301ctl] {self.cli} {' '.join(args)}", file=sys.stderr, flush=True)
        self._queue.append((args, done))
        self._pump()

    def _pump(self) -> None:
        if self._proc is not None or not self._queue or not self.cli:
            return
        args, done = self._queue.pop(0)
        proc = QProcess(self)
        proc.setProgram(self.cli)
        proc.setArguments(args)
        self._proc = proc
        self._busy = True
        self.changed.emit()

        # 启动失败（路径不对、权限）必须报出来，否则界面会一直"通讯中…"
        def failed(err) -> None:
            if err == QProcess.ProcessError.FailedToStart:
                self._fail(f"启动后端失败: {proc.errorString()}（{self.cli}）", "启动后端失败")

        # 看门狗：设备卡住时不能让界面永远转圈
        watchdog = QTimer(proc)
        watchdog.setSingleShot(True)
        watchdog.setInterval(20000)
        def on_timeout() -> None:
            self._fail("后端 20 秒无响应，已放弃本次读取", "超时")
            if proc.state() != QProcess.ProcessState.NotRunning:
                proc.kill()
        watchdog.timeout.connect(on_timeout)
        proc.errorOccurred.connect(failed)
        proc.started.connect(watchdog.start)

        def finished(code: int, _exit_status) -> None:
            out = bytes(proc.readAllStandardOutput().data()).decode("utf-8", "replace")
            err = bytes(proc.readAllStandardError().data()).decode("utf-8", "replace")
            watchdog.stop()
            proc.deleteLater()
            self._proc = None
            self._busy = bool(self._queue)
            if done is not None:
                done(code, out, err)
            self.changed.emit()
            self._pump()

        proc.finished.connect(finished)
        proc.start()

    @staticmethod
    def _parse(out: str) -> dict | None:
        out = out.strip()
        if not out:
            return None
        try:
            return json.loads(out)
        except json.JSONDecodeError:
            return None

    def _fail(self, err: str, fallback: str = "操作失败") -> None:
        msg = (err or "").strip().splitlines()
        self._status = msg[0] if msg else fallback
        self._error = True

    # ---------------- 读 ----------------
    @Slot()
    def loadSpec(self) -> None:
        """静态规格：能力表 + 功能码。只在启动时取一次。"""
        if self._spec or not self.cli:
            return

        def done(code: int, out: str, err: str) -> None:
            data = self._parse(out)
            if code != 0 or data is None:
                self._fail(err, "读取规格失败")
                return
            self._spec = data
            self._error = False

        self._run(["spec", "--json"], done)

    @Slot()
    def refresh(self) -> None:
        """实时状态快照。刷新**不会**写入设备。"""
        if not self.cli:
            return
        self._status = "读取中…"
        self._error = False
        self.changed.emit()

        def done(code: int, out: str, err: str) -> None:
            data = self._parse(out)
            if code != 0 or data is None:
                # 典型情况：设备射频休眠（Rust 侧已识别占位应答）
                self._fail(err, "读取失败")
                return
            self._state = data
            dev = data.get("device", {})
            pw = data.get("power", {})
            self._status = (
                f"已连接 · 固件 {dev.get('version', '?')} · 电量 {pw.get('battery', '?')}%"
                f" · 配置档 {dev.get('current_profile', '?')}/{dev.get('profile_count', '?')}"
            )
            self._error = False

        self._run(["status", "--json"], done)

    # ---------------- 写（全部经 CLI，安全规则在 Rust 侧） ----------------
    def _write(self, args_list: list[list[str]], ok_msg: str, refresh_after: bool = True) -> None:
        """串行执行若干条写命令；全部成功后提示 ok_msg 并刷新。任一步失败即停。"""
        if not self.cli or not args_list:
            return
        self._status = "写入中…"
        self.changed.emit()
        last = len(args_list) - 1

        def make(i: int):
            def done(code: int, out: str, err: str) -> None:
                if code != 0:
                    self._fail(err, "写入失败")
                    self._queue.clear()          # 失败即停，不再继续后面的写
                    return
                if i == last:
                    self._status = ok_msg
                    self._error = False
                    if refresh_after:
                        self.refresh()
            return done

        for i, args in enumerate(args_list):
            self._run(args, make(i))

    @Slot(int, int, int)
    def applyDpi(self, l1: int, l2: int, l3: int) -> None:
        vals = [l1, l2, l3]
        # 逐档写；CLI 会校验范围（100–8000）与 50 的倍数
        self._write(
            [["dpi", "set", str(i + 1), str(v)] for i, v in enumerate(vals)],
            "已写入 DPI（点「保存到 Flash」才断电保持）",
        )

    @Slot(int)
    def switchDpi(self, index: int) -> None:
        self._write([["dpi", "switch", str(index)]], f"已切换到 DPI 档 {index}")

    @Slot(int)
    def setPolling(self, hz: int) -> None:
        self._write([["perf", "polling", str(hz)]], f"回报率已设为 {hz} Hz")

    @Slot(int)
    def setResponse(self, value: int) -> None:
        self._write([["perf", "response", str(value)]], f"按键响应已设为值 {value}")

    @Slot("QVariantList")
    def applyButtons(self, codes: list) -> None:
        """按当前下拉框的选择逐键写入。codes 顺序与 spec.buttons 一致。"""
        if not self._spec:
            return
        names = [b["name"] for b in self._spec.get("buttons", [])]
        args: list[list[str]] = []
        for name, code in zip(names, codes):
            try:
                args.append(["buttons", "map", name, str(int(code))])
            except (TypeError, ValueError):
                # 界面还没读到设备值时不许把 None/undefined 当成 0 写下去
                self._status = f"{name} 没有有效值，已取消本次写入"
                self._error = True
                self.changed.emit()
                return
        self._write(args, "按键映射已写入（点「保存到 Flash」才断电保持）")

    @Slot(int)
    def switchProfile(self, index: int) -> None:
        self._write([["profile", "set", str(index)]], f"已切换到配置档 {index}")

    @Slot()
    def saveFlash(self) -> None:
        self._write([["perf", "save"]], "已保存到 Flash（断电保持）")

    @Slot()
    def resetFactory(self) -> None:
        self._write([["profile", "reset-all", "--yes"]], "已恢复出厂设置")

    @Slot(int, int)
    def setSleep(self, index: int, low: int) -> None:
        self._write([["power", "set", "--sleep", str(index), "--low", str(low)]], "电源设置已写入")
