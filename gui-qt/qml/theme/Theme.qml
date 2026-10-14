pragma Singleton
import QtQuick

// 设计 token —— 界面里所有颜色/圆角/间距/字号都从这里取。
// 等 FigCraft 原型出来，**只需要改这个文件**（或由原型导出的值生成它），
// 布局与逻辑不用动。
QtObject {
    // 颜色
    readonly property color bg:        "#101318"
    readonly property color card:      "#1A1E26"
    readonly property color cardHover: "#222735"
    readonly property color line:      "#2A303C"
    readonly property color track:     "#2E3542"
    readonly property color fg:        "#E9ECF2"
    readonly property color sub:       "#8D96A6"
    readonly property color accent:    "#4C8DFF"
    readonly property color accentFg:  "#0A0D13"
    readonly property color ok:        "#3DDC97"
    readonly property color warn:      "#FFB84D"
    readonly property color danger:    "#FF6B6B"

    // 形状 / 间距
    readonly property int radius:   14
    readonly property int radiusSm: 9
    readonly property int pad:      16
    readonly property int gap:      10

    // 字号
    readonly property int fontTitle: 16
    readonly property int fontBody:  13
    readonly property int fontSmall: 12
}
