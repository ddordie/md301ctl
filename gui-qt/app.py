"""md301ctl 的 Qt(QML) 前端 —— 启动入口。

运行：
    python app.py
（需要 PySide6：见 requirements.txt；后端 md301ctl.exe 见 backend.py 的查找顺序，
  也可用环境变量 MD301CTL 指定）
"""

from __future__ import annotations

import os
import sys

from PySide6.QtCore import QUrl
from PySide6.QtGui import QColor, QFont, QGuiApplication, QIcon, QPalette
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuickControls2 import QQuickStyle

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from backend import Backend  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def main() -> int:
    # 必须在建 QGuiApplication 之前指定控件样式
    QQuickStyle.setStyle("Fusion")
    app = QGuiApplication(sys.argv)
    app.setApplicationName("md301ctl")
    app.setOrganizationName("md301ctl")

    # 中文界面统一字体，避免 QML 默认字体缺字
    font = QFont("Microsoft YaHei UI")
    font.setPointSize(10)
    app.setFont(font)

    # 兜底的暗色调色板：任何没在 QML 里显式着色的控件（弹出框边框、滚动条等）
    # 也不会闪出白底。QML 里的 Theme 才是颜色的唯一来源。
    pal = QPalette()
    pal.setColor(QPalette.Window, QColor("#101318"))
    pal.setColor(QPalette.WindowText, QColor("#E9ECF2"))
    pal.setColor(QPalette.Base, QColor("#1A1E26"))
    pal.setColor(QPalette.AlternateBase, QColor("#161A22"))
    pal.setColor(QPalette.Text, QColor("#E9ECF2"))
    pal.setColor(QPalette.Button, QColor("#222735"))
    pal.setColor(QPalette.ButtonText, QColor("#E9ECF2"))
    pal.setColor(QPalette.Highlight, QColor("#4C8DFF"))
    pal.setColor(QPalette.HighlightedText, QColor("#0A0D13"))
    pal.setColor(QPalette.ToolTipBase, QColor("#1A1E26"))
    pal.setColor(QPalette.ToolTipText, QColor("#E9ECF2"))
    app.setPalette(pal)

    # 窗口/任务栏图标（assets/logo.ico 里含 16–256 多种尺寸）
    icon_path = os.path.join(HERE, "assets", "logo.ico")
    if not os.path.isfile(icon_path):
        icon_path = os.path.join(HERE, "assets", "logo-256.png")
    if os.path.isfile(icon_path):
        app.setWindowIcon(QIcon(icon_path))
    else:
        print(f"图标未找到: {icon_path}", file=sys.stderr)

    backend = Backend()

    engine = QQmlApplicationEngine()
    engine.addImportPath(os.path.join(HERE, "qml"))
    engine.rootContext().setContextProperty("backend", backend)
    engine.load(QUrl.fromLocalFile(os.path.join(HERE, "qml", "Main.qml")))

    if not engine.rootObjects():
        print("QML 加载失败", file=sys.stderr)
        return 1

    # 启动只读不写：先取静态规格（一次），再取一次实时状态
    backend.loadSpec()
    backend.refresh()

    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
