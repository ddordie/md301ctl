pragma Singleton
import QtQuick

// 设计 token —— 数值全部取自设计稿 UI_prototype-01 的 SVG（CSS 类 st0/st1/st3/st4/st8/st12/st14/st15/st20/st22）。
// 换设计稿只需要改这个文件。
QtObject {
    // ── 颜色（st0 页面底 / st1 卡片底 / st4 控件底+描边 / st14 强调 / st15 危险 / st8 在线 / st12 灰字 / st3 正文）
    readonly property color bg:          "#101318"
    readonly property color panel:       "#141921"
    readonly property color ctrl:        "#1A1E26"
    readonly property color border:      "#454F60"
    readonly property color accent:      "#674DFF"
    readonly property color accentHover: "#7C66FF"
    readonly property color danger:      "#FF6B6B"
    readonly property color ok:          "#3DDC97"
    readonly property color text:        "#FFFFFF"
    readonly property color muted:       "#454F60"
    readonly property color trackRest:   "#2B3243"

    // ── 形状
    readonly property int r:      5      // 胶囊圆角
    readonly property int rPanel: 10     // 卡片圆角

    // ── 字号（设计稿 20px 卡片标题 / 15px 正文）
    readonly property int fsTitle: 20
    readonly property int fsBody:  15

    // 设计稿用 思源黑体 Medium（SourceHanSansCN-Medium）；本机未装思源 → 雅黑回退（字宽接近，布局不变）
    readonly property string font: "Microsoft YaHei UI"

    // ── 旧名字的别名（旧组件文件里还在用，避免连带改动）
    readonly property color card:      panel
    readonly property color cardHover: ctrl
    readonly property color line:      trackRest
    readonly property color track:     trackRest
    readonly property color fg:        text
    readonly property color sub:       muted
    readonly property color accentFg:  "#0A0D13"
    readonly property color warn:      "#FFB84D"
    readonly property int radius:       rPanel
    readonly property int radiusSm:     r
    readonly property int pad:          16
    readonly property int gap:          10
    readonly property int fontTitle:    fsTitle
    readonly property int fontBody:     fsBody
    readonly property int fontSmall:    12
}
