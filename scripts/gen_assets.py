"""SVG → 多尺寸 PNG + ICO（用 QtSvg 渲染；ICO 容器纯标准库拼，PNG 载荷）。"""
import os, struct, sys, tempfile

from PySide6.QtCore import QSize, Qt
from PySide6.QtGui import QGuiApplication, QImage, QPainter
from PySide6.QtSvg import QSvgRenderer

app = QGuiApplication(sys.argv[:1])          # 离屏渲染，不显示窗口
svg, png_out, ico_out = sys.argv[1], sys.argv[2], sys.argv[3]
renderer = QSvgRenderer(svg)
print("SVG 有效:", renderer.isValid())
if not renderer.isValid():
    raise SystemExit(2)

tmpdir = tempfile.mkdtemp(prefix="ico_")
sizes = [16, 24, 32, 48, 64, 128, 256]
pngs = {}
for s in sizes:
    img = QImage(QSize(s, s), QImage.Format_ARGB32)
    img.fill(Qt.transparent)
    p = QPainter(img)
    p.setRenderHint(QPainter.RenderHint.Antialiasing, True)
    p.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform, True)
    renderer.render(p)
    p.end()
    path = os.path.join(tmpdir, f"{s}.png")
    ok = img.save(path, "PNG")
    print(f"  {s}px 渲染保存: {ok}")
    with open(path, "rb") as f:
        pngs[s] = f.read()

with open(png_out, "wb") as f:
    f.write(pngs[256])
print("PNG 写出:", png_out, len(pngs[256]), "字节")

n = len(sizes)
header = struct.pack("<HHH", 0, 1, n)
offset = 6 + 16 * n
entries, data = b"", b""
for s in sizes:
    d = pngs[s]
    dim = 0 if s == 256 else s
    entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(d), offset)
    offset += len(d)
    data += d
with open(ico_out, "wb") as f:
    f.write(header + entries + data)
print("ICO 写出:", ico_out, "尺寸", sizes, "共", 6 + 16 * n + len(data), "字节")
