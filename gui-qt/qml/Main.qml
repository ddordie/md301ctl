import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import "theme"
import "components"
import "proto.js" as Proto

// ASUS ProArt MD301 配置工具 —— 按设计稿 UI_prototype-01（1280×720 画布）还原。
// 静态层（卡片底、鼠标插画、引线、水印、静态文案）来自 assets/ui-bg.png；
// 交互控件在这里按设计稿坐标摆放（坐标表见 qml/proto.js）。
Window {
    id: win
    width: 1280
    height: 720
    minimumWidth: 1280
    minimumHeight: 720
    maximumWidth: 1280
    maximumHeight: 720
    visible: true
    color: Theme.bg
    title: "ASUS ProArt MD301"

    // ── 设备数据 ────────────────────────────────────────────────
    function caps() { return (backend.spec && backend.spec.caps) ? backend.spec.caps : ({}) }
    function st() { return backend.state ? backend.state : ({}) }
    function hasState() { return !!(backend.state && backend.state.dpi) }
    function connected() { return hasState() && !backend.error }
    function sleepTimes() { return caps().sleep_times ? caps().sleep_times : [] }
    function pollingRates() { return caps().polling_rates ? caps().polling_rates : [] }
    function responses() { return caps().button_responses ? caps().button_responses : [] }
    function slotNames() {
        var out = []
        var bs = (backend.spec && backend.spec.buttons) ? backend.spec.buttons : []
        for (var i = 0; i < bs.length; ++i) out.push(bs[i].name)
        return out
    }
    function funcOptions() { return (backend.spec && backend.spec.functions) ? backend.spec.functions : [] }
    // 槽位名 → 设计稿坐标（CLI 名字与设计稿名字可能有空格差异，忽略空格匹配）
    function comboPos(name) {
        var L = Proto.L.combos
        if (L[name]) return L[name]
        var k = String(name).replace(/\s+/g, "")
        for (var key in L) {
            if (String(key).replace(/\s+/g, "") === k) return L[key]
        }
        return [0, 0, 108, 31]
    }
    // 功能码 → 短名（"鼠标左键 / MouseLeftButton (0x1F0)" → "鼠标左键"）
    function funcShort(code) {
        if (code === null || code === undefined) return "—"
        var fs = funcOptions()
        for (var i = 0; i < fs.length; ++i) {
            if (fs[i].code === code) {
                var s = String(fs[i].label)
                var k = s.indexOf(" / ")
                return k > 0 ? s.substring(0, k) : s
            }
        }
        return String(code)
    }

    // ── 本地草稿（只有点「应用」才写设备）────────────────────────
    property var dpiDraft: [-1, -1, -1]
    property var mapDraft: ({})
    property int lowDraft: -1
    property int profileSel: 1
    property bool confirmReset: false
    property bool mapDirty: false
    property bool dpiDirty: false
    property bool lowDirty: false
    property string statusText: ""
    property bool statusErr: false

    function syncDrafts() {
        var s = st()
        if (!hasState()) return
        // 用户没改过 → 跟随设备；改过 → 保留草稿，别冲掉正在编辑的值
        if (!dpiDirty && s.dpi && s.dpi.levels) dpiDraft = [s.dpi.levels[0], s.dpi.levels[1], s.dpi.levels[2]]
        if (!mapDirty && s.buttons) {
            var m = {}
            for (var k in s.buttons) m[k] = s.buttons[k]
            mapDraft = m
        }
        if (!lowDirty && s.power && s.power.low_power !== undefined) lowDraft = s.power.low_power
        if (s.device && s.device.current_profile) profileSel = s.device.current_profile
    }

    function setMapping(slot, code) {
        var m = {}
        for (var k in mapDraft) m[k] = mapDraft[k]
        m[slot] = code
        mapDraft = m
        mapDirty = true
    }
    function setDpi(i, v) {
        var d = dpiDraft.slice()
        d[i] = v
        dpiDraft = d
        dpiDirty = true
    }
    function showStatus(t, err) { statusText = t; statusErr = !!err; toastTimer.restart() }
    Timer { id: toastTimer; interval: 8000; onTriggered: win.statusText = "" }

    Connections {
        target: backend
        function onChanged() {
            win.syncDrafts()
            if (backend.status && backend.status !== "") {
                win.statusText = backend.status
                win.statusErr = backend.error
                toastTimer.restart()
            }
        }
    }

    // ── 静态背景层（卡片底 / 鼠标插画 / 引线 / 水印 / 静态文案）──────
    Image {
        anchors.fill: parent
        source: "../assets/ui-bg.png"
        fillMode: Image.Stretch
        smooth: true
        mipmap: true
    }

    // ══ 顶栏 ══════════════════════════════════════════════════
    Rectangle {
        x: Proto.L.header.bar[0]
        y: Proto.L.header.bar[1]
        width: Proto.L.header.bar[2]
        height: Proto.L.header.bar[3]
        radius: Theme.rPanel
        color: Theme.ctrl
        border.width: 1
        border.color: Theme.border
    }
    // 指示灯 + 状态胶囊（文案只有「已连接」「未连接」两种）
    Rectangle {
        x: Proto.L.header.dot[0]
        y: Proto.L.header.dot[1]
        width: Proto.L.header.dot[2]
        height: Proto.L.header.dot[2]
        radius: width / 2
        color: win.connected() ? Theme.ok : Theme.muted
    }
    Rectangle {
        x: Proto.L.header.connChip[0]
        y: Proto.L.header.connChip[1]
        width: Proto.L.header.connChip[2]
        height: Proto.L.header.connChip[3]
        radius: Theme.r
        color: "transparent"
        border.width: 1
        border.color: win.connected() ? Theme.border : Theme.danger
        Text {
            anchors.centerIn: parent
            text: win.connected() ? "已连接" : "未连接"
            color: win.connected() ? Theme.muted : Theme.danger
            font.family: Theme.font
            font.pixelSize: Theme.fsBody
        }
    }
    // 配置档下拉
    Item {
        id: profileCombo
        x: Proto.L.header.profileCombo[0]
        y: Proto.L.header.profileCombo[1]
        width: Proto.L.header.profileCombo[2]
        height: Proto.L.header.profileCombo[3]
        Rectangle {
            anchors.fill: parent
            radius: Theme.r
            color: profileMouse.containsMouse ? "#20242E" : Theme.ctrl
            border.width: 1
            border.color: Theme.border
        }
        Text {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            text: "配置档 " + win.profileSel
            color: Theme.text
            font.family: Theme.font
            font.pixelSize: Theme.fsBody
        }
        Shape {
            anchors.right: parent.right
            anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            width: 9
            height: 5
            ShapePath {
                strokeColor: Theme.border
                strokeWidth: 1.2
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathPolyline { path: [Qt.point(0, 0), Qt.point(4.5, 4.2), Qt.point(9, 0)] }
            }
        }
        MouseArea {
            id: profileMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: profilePop.open()
        }
        Popup {
            id: profilePop
            width: profileCombo.width
            y: profileCombo.height + 4
            padding: 6
            background: Rectangle {
                color: Theme.panel
                radius: 8
                border.width: 1
                border.color: Theme.border
            }
            contentItem: Column {
                spacing: 2
                Repeater {
                    model: (st().device && st().device.profile_count) ? st().device.profile_count : 3
                    delegate: Item {
                        required property int index
                        width: profileCombo.width - 12
                        height: 28
                        Rectangle {
                            anchors.fill: parent
                            radius: 4
                            color: (index + 1 === win.profileSel) ? Theme.ctrl : (pMouse.containsMouse ? "#20242E" : "transparent")
                        }
                        Text {
                            anchors.left: parent.left
                            anchors.leftMargin: 8
                            anchors.verticalCenter: parent.verticalCenter
                            text: "配置档 " + (index + 1)
                            color: (index + 1 === win.profileSel) ? Theme.accent : Theme.text
                            font.family: Theme.font
                            font.pixelSize: Theme.fsBody
                        }
                        MouseArea {
                            id: pMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: { win.profileSel = index + 1; profilePop.close() }
                        }
                    }
                }
            }
        }
    }
    ActionButton {
        x: Proto.L.header.switchBtn[0]
        y: Proto.L.header.switchBtn[1]
        width: Proto.L.header.switchBtn[2]
        height: Proto.L.header.switchBtn[3]
        text: "切换"
        kind: "accent"
        active: win.hasState()
        onClicked: backend.switchProfile(win.profileSel)
    }
    // 电量百分比 + 电池图标
    Text {
        x: 1167.6
        y: 43.1
        text: (st().power && st().power.battery !== undefined) ? st().power.battery + "%" : "—"
        color: Theme.text
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
        horizontalAlignment: Text.AlignRight
    }
    Item {
        x: Proto.L.header.batteryIcon[0]
        y: Proto.L.header.batteryIcon[1] + 2
        width: Proto.L.header.batteryIcon[2]
        height: Proto.L.header.batteryIcon[3] - 2
        Rectangle {
            width: parent.width - 3
            height: parent.height
            radius: 2
            color: "transparent"
            border.width: 1
            border.color: Theme.muted
        }
        Rectangle {
            x: 2
            y: 2
            height: parent.height - 4
            width: Math.max(0, (parent.width - 7) * ((st().power && st().power.battery !== undefined) ? st().power.battery / 100 : 0))
            radius: 1
            color: {
                var p = st().power || ({})
                return (p.battery !== undefined && p.battery <= 20) ? Theme.danger : Theme.ok
            }
        }
        // 正极小凸点
        Rectangle {
            x: parent.width - 2
            y: parent.height / 2 - 2
            width: 2
            height: 4
            color: Theme.muted
        }
        Shape {
            anchors.centerIn: parent
            width: 8
            height: 12
            visible: !!(st().power && st().power.charging)
            ShapePath {
                fillColor: Theme.text
                strokeColor: "transparent"
                PathPolyline { path: [Qt.point(5, 0), Qt.point(1, 7), Qt.point(4, 7), Qt.point(3, 12), Qt.point(7, 5), Qt.point(4, 5)] }
            }
        }
    }

    // ══ 左栏：10 个按键映射下拉（位置＝该控件在鼠标上的物理位置）══════
    Repeater {
        model: win.slotNames()
        delegate: SlotCombo {
            required property string modelData
            property var pos: win.comboPos(modelData)
            x: pos[0]
            y: pos[1]
            width: pos[2]
            height: pos[3]
            slotName: modelData
            funcLabel: win.funcShort(win.mapDraft[modelData])
            curCode: (win.mapDraft[modelData] !== undefined && win.mapDraft[modelData] !== null) ? win.mapDraft[modelData] : -1
            options: win.funcOptions()
            onPicked: function (code) { win.setMapping(modelData, code) }
        }
    }
    ActionButton {
        x: Proto.L.buttons.applyMap[0]
        y: Proto.L.buttons.applyMap[1]
        width: Proto.L.buttons.applyMap[2]
        height: Proto.L.buttons.applyMap[3]
        text: "应用按键映射"
        kind: "accent"
        active: win.hasState()
        onClicked: {
            var names = win.slotNames()
            var codes = []
            for (var i = 0; i < names.length; ++i) {
                var c = win.mapDraft[names[i]]
                codes.push(c === undefined ? null : c)
            }
            backend.applyButtons(codes)
            win.mapDirty = false
        }
    }

    // ══ DPI 设定 ═══════════════════════════════════════════════
    Repeater {
        model: 3
        delegate: Item {
            required property int index
            property bool isCur: !!(st().dpi && st().dpi.current_index === index + 1)
            x: 0
            y: Proto.L.dpiRows[index].y
            width: win.width
            height: 31

            Text {
                x: 768
                y: 0
                width: 22
                height: 31
                verticalAlignment: Text.AlignVCenter
                text: String(index + 1)
                color: isCur ? Theme.accent : Theme.muted
                font.family: Theme.font
                font.pixelSize: Theme.fsBody
            }
            TrackSlider {
                id: dpiSlider
                x: 800
                y: 0
                width: 267
                height: 31
                from: caps().dpi_min !== undefined ? caps().dpi_min : 100
                to: caps().dpi_max !== undefined ? caps().dpi_max : 8000
                stepSize: caps().dpi_step !== undefined ? caps().dpi_step : 50
                value: win.dpiDraft[index] > 0 ? win.dpiDraft[index] : 0
                enabled: win.hasState()
                onMoved: win.setDpi(index, Math.round(value / stepSize) * stepSize)
            }
            ValueBox {
                x: Proto.L.dpiRows[index].valBox[0]
                y: 0
                width: Proto.L.dpiRows[index].valBox[2]
                height: Proto.L.dpiRows[index].valBox[3]
                text: win.dpiDraft[index] > 0 ? String(win.dpiDraft[index]) : "—"
                accent: isCur
            }
            ActionButton {
                x: Proto.L.dpiRows[index].btn[0]
                y: 0
                width: Proto.L.dpiRows[index].btn[2]
                height: Proto.L.dpiRows[index].btn[3]
                text: "设为当前"
                kind: "accent"
                active: win.hasState() && !isCur
                onClicked: backend.switchDpi(index + 1)
            }
        }
    }
    Text {
        x: 774.7
        y: 307.4
        text: "DPI挡位范围 " + (caps().dpi_min !== undefined ? caps().dpi_min : 100) + "-" + (caps().dpi_max !== undefined ? caps().dpi_max : 8000)
        color: Theme.muted
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
    }
    ActionButton {
        x: Proto.L.applyDpiBtn[0]
        y: Proto.L.applyDpiBtn[1]
        width: Proto.L.applyDpiBtn[2]
        height: Proto.L.applyDpiBtn[3]
        text: "应用DPI"
        kind: "accent"
        active: win.hasState()
        onClicked: {
            if (win.dpiDraft[0] > 0 && win.dpiDraft[1] > 0 && win.dpiDraft[2] > 0) {
                backend.applyDpi(win.dpiDraft[0], win.dpiDraft[1], win.dpiDraft[2])
                win.dpiDirty = false
            }
        }
    }

    // ══ 回报率 / 按键响应 ═══════════════════════════════════════
    Repeater {
        model: win.pollingRates()
        delegate: Pill {
            required property int index
            required property var modelData
            x: Proto.L.polling.pills[index][0]
            y: Proto.L.polling.y
            width: Proto.L.polling.pills[index][1]
            height: 31
            text: modelData + " Hz"
            current: {
                var p = st().perf || ({})
                var n = parseInt(String(p.polling_usb_rf || ""), 10)
                return !isNaN(n) && n === modelData
            }
            onClicked: backend.setPolling(modelData)
        }
    }
    Repeater {
        model: win.responses()
        delegate: Pill {
            required property int index
            required property var modelData
            x: Proto.L.response.pills[index][0]
            y: Proto.L.response.y
            width: Proto.L.response.pills[index][1]
            height: 31
            text: modelData.ms + " ms"
            current: (st().perf && st().perf.button_response !== undefined) && st().perf.button_response === modelData.value
            onClicked: backend.setResponse(modelData.value)
        }
    }

    // ══ 电源（休眠时间 / 低电量提醒）══════════════════════════════
    Repeater {
        model: win.sleepTimes()
        delegate: Pill {
            required property int index
            required property var modelData
            x: Proto.L.sleep.pills[index][0]
            y: Proto.L.sleep.y
            width: Proto.L.sleep.pills[index][1]
            height: 31
            text: modelData.label
            // 设备给的是档位**值**（0..4，永不=255）
            current: (st().power && st().power.sleep_index !== undefined) && modelData.value === st().power.sleep_index
            onClicked: backend.setSleep(modelData.value, win.lowDraft >= 0 ? win.lowDraft : 20)
        }
    }
    TrackSlider {
        id: lowSlider
        x: 865
        y: 608
        width: 317
        height: 22
        from: caps().low_power_min !== undefined ? caps().low_power_min : 0
        to: caps().low_power_max !== undefined ? caps().low_power_max : 100
        stepSize: 5
        value: win.lowDraft >= 0 ? win.lowDraft : 0
        enabled: win.hasState()
        onMoved: { win.lowDraft = Math.round(value / stepSize) * stepSize; win.lowDirty = true }
        // 松手才写；拖动过程不刷设备
        onPressedChanged: {
            if (!pressed && win.lowDraft >= 0 && win.hasState()) {
                backend.setSleep((st().power && st().power.sleep_index !== undefined) ? st().power.sleep_index : 1, win.lowDraft)
                win.lowDirty = false
            }
        }
    }
    Text {
        x: 1199.9
        y: 609.5
        text: (win.lowDraft >= 0 ? win.lowDraft : 0) + "%"
        color: Theme.muted
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
    }


    // ══ 底部动作 ═══════════════════════════════════════════════
    ActionButton {
        x: Proto.L.buttons.saveFlash[0]
        y: Proto.L.buttons.saveFlash[1]
        width: Proto.L.buttons.saveFlash[2]
        height: Proto.L.buttons.saveFlash[3]
        text: "保存到Flash"
        kind: "accent"
        active: win.hasState()
        onClicked: backend.saveFlash()
    }
    ActionButton {
        x: Proto.L.buttons.factory[0]
        y: Proto.L.buttons.factory[1]
        width: Proto.L.buttons.factory[2]
        height: Proto.L.buttons.factory[3]
        text: "恢复出厂设置"
        kind: "danger"
        active: win.hasState()
        onClicked: win.confirmReset = true
    }

    // 底部提示（设计稿没有这一条，但休眠/错误反馈必须有地方显示）
    Text {
        x: 46
        y: 700
        width: 620
        text: win.statusText
        color: win.statusErr ? Theme.danger : Theme.muted
        font.family: Theme.font
        font.pixelSize: 13
        elide: Text.ElideRight
        opacity: win.statusText === "" ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: 150 } }
    }

    // 恢复出厂二次确认（危险操作）
    Popup {
        id: resetPop
        visible: win.confirmReset
        modal: true
        anchors.centerIn: Overlay.overlay
        width: 400
        height: 160
        padding: 16
        background: Rectangle {
            color: Theme.panel
            radius: Theme.rPanel
            border.width: 1
            border.color: Theme.border
        }
        contentItem: Column {
            spacing: 16
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "恢复出厂设置会清除键位映射、DPI、电源等全部配置，是否继续？"
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: Theme.fsBody
            }
            Row {
                spacing: 12
                anchors.right: parent.right
                ActionButton {
                    width: 90
                    height: 31
                    text: "取消"
                    kind: "ghost"
                    active: true
                    onClicked: win.confirmReset = false
                }
                ActionButton {
                    width: 96
                    height: 31
                    text: "确定恢复"
                    kind: "danger"
                    active: true
                    onClicked: { win.confirmReset = false; backend.resetFactory() }
                }
            }
        }
    }
}
