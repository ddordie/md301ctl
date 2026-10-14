"""无头自检：不显示窗口，验证「后端桥 → md301ctl.exe → 设备」这条链路。

用法：.venv/Scripts/python.exe smoke.py
只读不写，可安全反复运行。
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PySide6.QtCore import QCoreApplication, QTimer  # noqa: E402

from backend import Backend  # noqa: E402


def main() -> int:
    app = QCoreApplication(sys.argv)
    be = Backend()
    print(f"后端程序: {be.cliPath or '未找到'}")
    be.loadSpec()
    be.refresh()

    result = {"ok": False}

    def report() -> None:
        print(f"规格: {sorted(be.spec.keys()) if be.spec else '空'}")
        print(f"状态: {sorted(be.state.keys()) if be.state else '空'}")
        print(f"status='{be.status}' error={be.error} busy={be.busy}")
        st = be.state or {}
        if st.get("dpi"):
            print(f"  DPI 三档: {st['dpi'].get('levels')} 当前档 {st['dpi'].get('current_index')}")
        if st.get("perf"):
            print(f"  回报率 {st['perf'].get('polling_usb_rf')} / 按键响应 {st['perf'].get('button_response_ms')} ms")
        if st.get("power"):
            print(f"  电量 {st['power'].get('battery')}%")
        if be.spec:
            caps = be.spec.get("caps", {})
            print(f"  回报率档位 {caps.get('polling_rates')} / 响应档位 "
                  f"{[o['ms'] for o in caps.get('button_responses', [])]} / 功能码 {len(be.spec.get('functions', []))} 个")
        result["ok"] = bool(be.state) and bool(be.spec) and not be.error
        app.quit()

    QTimer.singleShot(10000, report)
    app.exec()
    print("结果:", "通过" if result["ok"] else "未通过")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
