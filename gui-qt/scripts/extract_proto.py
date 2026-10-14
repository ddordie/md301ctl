#!/usr/bin/env python3
"""从设计稿 SVG 生成界面的静态背景层 assets/ui-bg.svg + ui-bg.png。

界面分两层：
  * 静态层（本脚本产物）：卡片底、鼠标插画、引线、水印、面板标题、行标签
  * 动态层（qml/Main.qml 画）：所有会随设备状态变化的胶囊、滑条、数值、指示灯、下拉箭头

所以本脚本做的事就是：把设计稿里**由 QML 负责的部分**从背景里删掉 —— 否则会出现
「设计稿画了一个手柄 + QML 又画一个」这种重影（实测踩过）。

用法：
    python scripts/extract_proto.py <设计稿.svg> [assets/ui-bg.svg]
然后渲染成 2x PNG（QML 里用 Image.Stretch 铺满 1280x720）：
    python app.py --render-only   # 见 README「重新生成背景层」

坐标口径提醒：SVG 的 <text y> 是**基线**，Qt 的 Text.y 是**顶边**，换算时大约要减 12–15px。
"""
from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET

SVG_NS = "http://www.w3.org/2000/svg"
XLINK_NS = "http://www.w3.org/1999/xlink"

# 由 QML 绘制的形状类（胶囊底/按钮/数值框/手柄/指示灯/下拉箭头）
SHAPE_DROP = {"st4", "st14", "st15", "st23", "st11", "st12", "st22", "st8", "st10"}
# 由 QML 绘制的线（滑条轨道）
LINE_DROP = {"st20", "st21"}
# 由 QML 绘制的圆（手柄、在线指示灯）
CIRCLE_DROP = {"st22", "st8"}
# 由 QML 绘制的文字（动态值 + 按钮文字 + 行号 + 说明）
TEXT_DROP = {
    "81%", "配置档1", "已连接", "1100", "2000", "3800",
    "125 Hz", "250 Hz", "500 Hz", "1000 Hz",
    "12 ms", "16 ms", "20 ms", "24 ms", "28 ms", "32 ms",
    "1 分钟", "2 分钟", "3 分钟", "5 分钟", "10 分钟", "永不", "5%",
    "设为当前", "切换", "应用DPI", "应用按键映射", "保存到Flash", "恢复出厂设置",
    "左键", "右键", "DPI切换", "侧键前进", "侧键后退", "向左滚动", "向右滚动",
    "滚轮点击", "向上滚动", "向下滚动", "1", "2", "3", "DPI挡位范围 100-8000",
}
# 电量图标（QML 按电量/充电状态自己画）所在的绝对坐标范围
BATTERY_BOX = (1198, 40, 1236, 64)


def _nums(d: str) -> list[float]:
    return [float(x) for x in re.findall(r"-?\d*\.?\d+(?:[eE]-?\d+)?", d)]


def _tf(s: str | None) -> tuple[float, ...]:
    if not s:
        return (1, 0, 0, 1, 0, 0)
    m = re.match(r"matrix\(([-\d.eE ]+)\)", s)
    if m:
        return tuple(_nums(m.group(1))[:6])  # type: ignore[return-value]
    m = re.match(r"translate\(([-\d.eE, ]+)\)", s)
    if m:
        v = _nums(m.group(1))
        return (1, 0, 0, 1, v[0], v[1] if len(v) > 1 else 0)
    return (1, 0, 0, 1, 0, 0)


def _mul(a, b):
    return (a[0] * b[0] + a[2] * b[1], a[1] * b[0] + a[3] * b[1],
            a[0] * b[2] + a[2] * b[3], a[1] * b[2] + a[3] * b[3],
            a[0] * b[4] + a[2] * b[5] + a[4], a[1] * b[4] + a[3] * b[5] + a[5])


def _apply(m, x, y):
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


def bbox(d: str, polyline: bool = False):
    """迷你 path 解析（M/L/H/V/C/S/Q/A + 相对指令），返回局部坐标包围盒。"""
    toks = re.findall(r"[MmLlHhVvCcSsQqTtAaZz]|-?\d*\.?\d+(?:[eE]-?\d+)?", d)
    cx = cy = sx = sy = 0.0
    xs: list[float] = []
    ys: list[float] = []
    i = 0
    cmd = "M" if polyline else None

    def n() -> float:
        nonlocal i
        v = float(toks[i])
        i += 1
        return v

    while i < len(toks):
        t = toks[i]
        if re.match(r"[A-Za-z]", t):
            cmd = t
            if cmd in "Zz":
                i += 1
                cx, cy = sx, sy
                continue
            i += 1
            continue
        if cmd is None:
            cmd = "M"
        rel = cmd.islower()
        c = cmd.upper()
        if c == "M":
            x, y = n(), n()
            cx, cy = (cx + x, cy + y) if rel else (x, y)
            sx, sy = cx, cy
            xs.append(cx); ys.append(cy)
            cmd = "l" if rel else "L"
        elif c == "L":
            x, y = n(), n()
            cx, cy = (cx + x, cy + y) if rel else (x, y)
            xs.append(cx); ys.append(cy)
        elif c == "H":
            x = n(); cx = cx + x if rel else x
            xs.append(cx); ys.append(cy)
        elif c == "V":
            y = n(); cy = cy + y if rel else y
            xs.append(cx); ys.append(cy)
        elif c == "C":
            p = [n() for _ in range(6)]
            if rel:
                p = [cx + p[0], cy + p[1], cx + p[2], cy + p[3], cx + p[4], cy + p[5]]
            xs += [p[0], p[2], p[4]]; ys += [p[1], p[3], p[5]]
            cx, cy = p[4], p[5]
        elif c in "SQ":
            p = [n() for _ in range(4)]
            if rel:
                p = [cx + p[0], cy + p[1], cx + p[2], cy + p[3]]
            xs += [p[0], p[2]]; ys += [p[1], p[3]]
            cx, cy = p[2], p[3]
        elif c == "A":
            v = [n() for _ in range(7)]
            cx, cy = (cx + v[5], cy + v[6]) if rel else (v[5], v[6])
            xs.append(cx); ys.append(cy)
        else:
            n()
    return (min(xs), min(ys), max(xs), max(ys)) if xs else None


def _tag(el) -> str:
    return el.tag.split("}")[-1]


def strip_for_background(src: str, dst: str) -> tuple[int, list[str]]:
    ET.register_namespace("", SVG_NS)
    ET.register_namespace("xlink", XLINK_NS)
    tree = ET.parse(src)
    root = tree.getroot()
    removed = 0

    def walk(el, m):
        nonlocal removed
        mm = _mul(m, _tf(el.get("transform")))
        for ch in list(el):
            if _tag(ch) == "g":
                walk(ch, mm)
                continue
            cls = set((ch.get("class") or "").split())
            tag = _tag(ch)
            if tag == "text" and "".join(ch.itertext()).strip() in TEXT_DROP:
                el.remove(ch); removed += 1; continue
            if tag in ("path", "polyline", "polygon", "rect") and cls and cls <= SHAPE_DROP:
                el.remove(ch); removed += 1; continue
            if tag == "line" and cls and cls <= LINE_DROP:
                el.remove(ch); removed += 1; continue
            if tag == "circle" and cls and cls <= CIRCLE_DROP:
                el.remove(ch); removed += 1; continue
            # 电量图标按位置删（它的 class 与鼠标插画共用一个）
            area = None
            d = ch.get("d") or ch.get("points")
            if d:
                b = bbox(d, tag == "polyline")
                if b:
                    cor = [_apply(mm, b[0], b[1]), _apply(mm, b[2], b[1]),
                           _apply(mm, b[2], b[3]), _apply(mm, b[0], b[3])]
                    area = (min(c[0] for c in cor), min(c[1] for c in cor),
                            max(c[0] for c in cor), max(c[1] for c in cor))
            elif ch.get("cx") is not None:
                x, y = _apply(mm, float(ch.get("cx")), float(ch.get("cy")))
                r = float(ch.get("r", "0"))
                area = (x - r, y - r, x + r, y + r)
            if area:
                cx_, cy_ = (area[0] + area[2]) / 2, (area[1] + area[3]) / 2
                if BATTERY_BOX[0] <= cx_ <= BATTERY_BOX[2] and BATTERY_BOX[1] <= cy_ <= BATTERY_BOX[3]:
                    el.remove(ch); removed += 1

    walk(root, (1, 0, 0, 1, 0, 0))
    # 设计稿的字体名是 Illustrator 的内部名（SourceHanSansCN-Medium-GBpc-EUC-H），
    # 本机没有就会被 Qt 回退成宋体（衬线），整层文字和 QML 对不上 → 统一成 Theme.font
    txt = ET.tostring(root, encoding="unicode")
    txt = re.sub(r"font-family:'[^']*SourceHan[^']*'", "font-family:'Microsoft YaHei UI'", txt)
    root = ET.fromstring(txt)
    for el in root.iter():               # 设计稿把标题写成了 AUSU，顺手修正
        if _tag(el) == "text" and el.text and "AUSU" in el.text:
            el.text = el.text.replace("AUSU", "ASUS")
    tree.write(dst, encoding="utf-8", xml_declaration=True)
    kept = [" ".join(t.itertext()).strip() for t in root.iter() if _tag(t) == "text"]
    return removed, kept


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    src = sys.argv[1]
    dst = sys.argv[2] if len(sys.argv) > 2 else "assets/ui-bg.svg"
    removed, kept = strip_for_background(src, dst)
    print(f"删掉 {removed} 个动态元素 → {dst}")
    print("保留的静态文字：", " / ".join(kept))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
