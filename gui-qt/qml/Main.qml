import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "theme"
import "components"

Window {
    id: win
    width: 680
    height: 940
    minimumWidth: 560
    minimumHeight: 520
    visible: true
    color: Theme.bg
    title: "ASUS ProArt MD301"

    // ── 本地草稿值：界面可编辑，只有点「应用」才写设备 ──────────────────
    property var dpiVals: []          // 空 = 尚未读到设备值
    property var btnCodes: ({})
    property int profileSel: 1
    property int sleepSel: -1      // -1 = 还没从设备读到
    property int lowSel: -1
    property bool confirmReset: false

    function specCaps() { return backend.spec && backend.spec.caps ? backend.spec.caps : ({}) }
    function stateOf() { return backend.state ? backend.state : ({}) }
    function hasState() { return backend.state && backend.state.dpi !== undefined }
    // 已读到 → 用草稿值；未读到 → 显示占位符，绝不显示 0
    function dpiShown(i) { return win.dpiVals[i] > 0 ? win.dpiVals[i] : 0 }
    function dpiIsCurrent(i) {
        var d = stateOf().dpi
        return d !== undefined && d.current_index === i + 1
    }

    // 把设备读到的值同步到草稿（拖动滑块时不覆盖）
    function syncFromDevice() {
        var st = stateOf()
        if (st.dpi && st.dpi.levels) win.dpiVals = st.dpi.levels.slice(0, 3)
        if (st.buttons) win.btnCodes = JSON.parse(JSON.stringify(st.buttons))
        if (st.device && st.device.current_profile) win.profileSel = st.device.current_profile
        if (st.power) {
            // 只在还没读到过时填，避免自动刷新把用户没应用的改动冲掉
            if (win.sleepSel < 0) win.sleepSel = st.power.sleep_index
            if (win.lowSel < 0) win.lowSel = st.power.low_power
        }
    }
    function curSleepLabel() {
        var p = stateOf().power
        if (!p) return "—"
        var sl = specCaps().sleep_times || []
        for (var i = 0; i < sl.length; i++) if (sl[i].value === p.sleep_index) return sl[i].label
        return "未知(" + p.sleep_index + ")"
    }
    function powerDirty() {
        var p = stateOf().power
        if (!p) return false
        return (win.sleepSel >= 0 && win.sleepSel !== p.sleep_index)
            || (win.lowSel >= 0 && win.lowSel !== p.low_power)
    }

    Connections {
        target: backend
        // 自动轮询时不覆盖草稿（否则会把你正在调的值冲掉）；首次读到数据时同步一次
        function onChanged() {
            if (backend.draftSync || win.dpiVals.length === 0) win.syncFromDevice()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 顶部标题栏
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 56
            color: Theme.card
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad + 4
                anchors.rightMargin: Theme.pad
                spacing: 10
                Rectangle {
                    width: 9; height: 9; radius: 4.5
                    color: backend.error ? Theme.warn : Theme.ok
                }
                Text {
                    text: "ASUS ProArt MD301"
                    color: Theme.fg
                    font.pixelSize: 17
                    font.bold: true
                }
                Text {
                    text: backend.busy ? "通讯中…" : ""
                    color: Theme.sub
                    font.pixelSize: Theme.fontSmall
                }
                Item { Layout.fillWidth: true }
                PillButton {
                    text: backend.autoPoll ? "自动刷新：开" : "自动刷新：关"
                    primary: backend.autoPoll
                    onClicked: backend.setAutoPoll(!backend.autoPoll)
                }
                PillButton {
                    text: backend.busy ? "读取中…" : "重新读取"
                    enabled: !backend.busy
                    onClicked: backend.refresh()
                }
            }
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width; height: 1
                color: Theme.line
            }
        }

        ScrollView {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.vertical: ScrollBar {
                width: 8
                policy: ScrollBar.AsNeeded
                background: Rectangle { color: "transparent" }
                contentItem: Rectangle {
                    implicitWidth: 8
                    radius: 4
                    color: Theme.line
                    opacity: parent.pressed ? 1.0 : 0.8
                }
            }

            ColumnLayout {
                width: scroller.availableWidth
                spacing: Theme.gap + 4

                Item { implicitHeight: Theme.gap + 4 }

                // ── 设备状态
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "设备状态"
                    titleVisible: true
                    spacing: 7

                    Text {
                        Layout.fillWidth: true
                        color: Theme.sub
                        font.pixelSize: Theme.fontBody
                        wrapMode: Text.WordWrap
                        text: {
                            var st = stateOf()
                            if (!st.device) return "尚未读到设备信息"
                            var d = st.device, p = st.power || {}
                            var pwr = (p.battery !== undefined ? " · 电量 " + p.battery + "%" : "")
                                + (p.charging ? "（充电中）" : "")
                            return "固件 " + d.version + " · 接收器 " + d.dongle_version
                                + " · 配置档 " + d.current_profile + " / " + d.profile_count + pwr
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        color: Theme.sub
                        font.pixelSize: Theme.fontSmall
                        wrapMode: Text.WordWrap
                        visible: stateOf().power !== undefined
                        text: {
                            var p = stateOf().power
                            if (!p) return ""
                            var sl = specCaps().sleep_times || []
                            var lab = ""
                            for (var i = 0; i < sl.length; i++) if (sl[i].value === p.sleep_index) lab = sl[i].label
                            return "休眠 " + lab + " · 低电量提醒 " + p.low_power + "%"
                        }
                    }
                }

                // ── DPI
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "DPI"
                    titleVisible: true
                    spacing: 10

                    Repeater {
                        model: 3
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 12
                            Text {
                                text: (index + 1) + ""
                                color: win.dpiIsCurrent(index) ? Theme.accent : Theme.sub
                                font.pixelSize: Theme.fontBody
                                Layout.preferredWidth: 14
                            }
                            StyledSlider {
                                id: sl
                                Layout.fillWidth: true
                                minValue: specCaps().dpi_min !== undefined ? specCaps().dpi_min : 100
                                maxValue: specCaps().dpi_max !== undefined ? specCaps().dpi_max : 8000
                                value: win.dpiShown(index) > 0 ? win.dpiShown(index) : sl.from
                                onMoved: {
                                    var v = win.dpiVals.slice()
                                    v[index] = Math.round(value / stepSize) * stepSize
                                    win.dpiVals = v
                                }
                            }
                            Text {
                                text: win.dpiShown(index) > 0 ? win.dpiShown(index) + "" : "—"
                                color: win.dpiIsCurrent(index) ? Theme.accent : Theme.fg
                                font.pixelSize: 15
                                font.bold: win.dpiIsCurrent(index)
                                horizontalAlignment: Text.AlignRight
                                Layout.preferredWidth: 58
                            }
                            PillButton {
                                text: win.dpiIsCurrent(index) ? "当前档" : "设为当前"
                                primary: win.dpiIsCurrent(index)
                                enabled: !backend.busy && win.hasState() && !win.dpiIsCurrent(index)
                                onClicked: backend.switchDpi(index + 1)
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            color: Theme.sub
                            font.pixelSize: Theme.fontSmall
                            text: "档位数 " + (specCaps().dpi_levels !== undefined ? specCaps().dpi_levels : "?")
                                + "（该型号固定） · 范围 " + (specCaps().dpi_min !== undefined ? specCaps().dpi_min : "?")
                                + "–" + (specCaps().dpi_max !== undefined ? specCaps().dpi_max : "?")
                        }
                        Item { Layout.fillWidth: true }
                        PillButton {
                            text: "应用 DPI"
                            primary: true
                            // 没读到设备值前不允许写（否则会拿空草稿去覆盖设备）
                            enabled: !backend.busy && win.hasState()
                            onClicked: backend.applyDpi(win.dpiVals[0], win.dpiVals[1], win.dpiVals[2])
                        }
                    }
                }

                // ── 回报率 / 按键响应
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "回报率 / 按键响应"
                    titleVisible: true
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            text: "回报率"
                            color: Theme.sub
                            font.pixelSize: Theme.fontBody
                            Layout.preferredWidth: 68
                        }
                        Repeater {
                            model: specCaps().polling_rates || []
                            delegate: PillButton {
                                property int hz: modelData
                                property bool cur: stateOf().perf !== undefined
                                                   && stateOf().perf.polling_usb_rf === hz + " Hz"
                                text: hz + " Hz"
                                primary: cur
                                enabled: !backend.busy && stateOf().perf !== undefined && !cur
                                // 只在"用户真的点了另一档"时写；启动/刷新不动设备
                                onClicked: if (!cur) backend.setPolling(hz)
                            }
                        }
                        Item { Layout.fillWidth: true }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            text: "按键响应"
                            color: Theme.sub
                            font.pixelSize: Theme.fontBody
                            Layout.preferredWidth: 68
                        }
                        Repeater {
                            model: specCaps().button_responses || []
                            delegate: PillButton {
                                property var opt: modelData
                                property bool cur: stateOf().perf !== undefined
                                                   && stateOf().perf.button_response === opt.value
                                text: opt.ms + " ms"
                                primary: cur
                                enabled: !backend.busy && stateOf().perf !== undefined && !cur
                                onClicked: if (!cur) backend.setResponse(opt.value)
                            }
                        }
                        Item { Layout.fillWidth: true }
                    }
                }

                // ── 电源（休眠时间 / 低电量提醒）
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "电源（休眠 / 低电量）"
                    titleVisible: true
                    spacing: 10

                    Text {
                        Layout.fillWidth: true
                        color: Theme.sub
                        font.pixelSize: Theme.fontBody
                        wrapMode: Text.WordWrap
                        text: {
                            var p = stateOf().power
                            if (!p) return "尚未读到设备电源信息"
                            return "设备当前：休眠 " + win.curSleepLabel() + " · 低电量提醒 " + p.low_power + "%"
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            text: "休眠时间"
                            color: Theme.sub
                            font.pixelSize: Theme.fontBody
                            Layout.preferredWidth: 68
                        }
                        // 档位表来自官方 SleepTime（0/1/2/3/5/10 分钟 + 255 永不）
                        Repeater {
                            model: specCaps().sleep_times || []
                            delegate: PillButton {
                                property var opt: modelData
                                property bool cur: stateOf().power !== undefined
                                                   && stateOf().power.sleep_index === opt.value
                                text: opt.label
                                primary: cur
                                enabled: !backend.busy && win.hasState() && !cur
                                onClicked: if (!cur) backend.setSleep(opt.value, win.lowSel >= 0 ? win.lowSel : 20)
                            }
                        }
                        Item { Layout.fillWidth: true }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            text: "低电量提醒"
                            color: Theme.sub
                            font.pixelSize: Theme.fontBody
                            Layout.preferredWidth: 68
                        }
                        StyledSlider {
                            id: lowSl
                            Layout.fillWidth: true
                            minValue: specCaps().low_power_min !== undefined ? specCaps().low_power_min : 0
                            maxValue: specCaps().low_power_max !== undefined ? specCaps().low_power_max : 100
                            step: 5
                            enabled: win.hasState()
                            value: win.lowSel >= 0 ? win.lowSel : 20
                            onMoved: win.lowSel = Math.round(value / step) * step
                            // 松手才写，拖动过程中不刷设备
                            onPressedChanged: if (!pressed && win.hasState() && win.lowSel >= 0)
                                                  backend.setSleep(win.sleepSel >= 0 ? win.sleepSel : 1, win.lowSel)
                        }
                        Text {
                            text: (win.lowSel >= 0 ? win.lowSel : "—") + "%"
                            color: Theme.fg
                            font.pixelSize: 15
                            horizontalAlignment: Text.AlignRight
                            Layout.preferredWidth: 52
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        color: Theme.sub
                        font.pixelSize: Theme.fontSmall
                        wrapMode: Text.WordWrap
                        text: "点选即写入设备；改动只在点「保存到 Flash」后才断电保持（官方档位：1/2/3/5/10 分钟与「永不」）。"
                    }
                }

                // ── 按键映射
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "按键映射"
                    titleVisible: true
                    spacing: 8

                    Repeater {
                        model: backend.spec && backend.spec.buttons ? backend.spec.buttons : []
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 10
                            Text {
                                text: modelData.name
                                color: Theme.sub
                                font.pixelSize: Theme.fontBody
                                Layout.preferredWidth: 88
                            }
                            DarkCombo {
                                id: fn
                                Layout.fillWidth: true
                                    model: backend.spec && backend.spec.functions ? backend.spec.functions : []
                                textRole: "label"
                                valueRole: "code"

                                property int devCode: win.btnCodes[modelData.name] !== undefined
                                                      ? win.btnCodes[modelData.name] : 0
                                placeholder: "读取中…"

                                function syncIndex() {
                                    if (devCode === 0) { currentIndex = -1; return }
                                    for (var i = 0; i < count; i++) {
                                        if (model[i] && model[i].code === devCode) { currentIndex = i; return }
                                    }
                                    currentIndex = -1          // 对不上就别假装选中第一项
                                }
                                onDevCodeChanged: syncIndex()
                                onCountChanged: syncIndex()
                                Component.onCompleted: syncIndex()
                                // onActivated 而不是 onCurrentIndexChanged：只有用户选择才写
                                onActivated: {
                                    var v = model[currentIndex].code
                                    if (v === devCode) return          // 值没变就不写
                                    var c = win.btnCodes
                                    c[modelData.name] = v
                                    win.btnCodes = JSON.parse(JSON.stringify(c))
                                }
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        Text {
                            color: Theme.fontSmall === 0 ? Theme.fg : Theme.sub
                            font.pixelSize: Theme.fontSmall
                            text: "左键/右键/中键可自由改；改完点右侧按钮写入"
                        }
                        Item { Layout.fillWidth: true }
                        PillButton {
                            text: "应用按键映射"
                            primary: true
                            enabled: !backend.busy && win.hasState() && backend.spec.buttons !== undefined
                            onClicked: {
                                var codes = []
                                var names = backend.spec.buttons
                                for (var i = 0; i < names.length; i++) codes.push(win.btnCodes[names[i].name])
                                backend.applyButtons(codes)
                            }
                        }
                    }
                }

                // ── 配置档 / 保存
                Card {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.pad
                    Layout.rightMargin: Theme.pad
                    title: "配置档"
                    titleVisible: true
                    spacing: Theme.gap

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gap
                        DarkCombo {
                            id: prof
                            Layout.preferredWidth: 150
                            placeholder: "读取中…"
                            textRole: "label"        // 框里显示的文本：没有它，对象数组的框会是空的
                            // 用 {n,label} 对象数组：下拉项的文本走 label 角色
                            model: {
                                var n = stateOf().device ? stateOf().device.profile_count : 0
                                var a = []
                                for (var i = 1; i <= n; i++) a.push({ n: i, label: "配置档 " + i })
                                return a
                            }
                            // 设备当前档（model 后到，所以要在 model/计数变化时再同步一次）
                            property int devProfile: stateOf().device
                                                      ? stateOf().device.current_profile : 0
                            function syncIndex() {
                                if (devProfile <= 0) { currentIndex = -1; return }
                                currentIndex = devProfile - 1
                                win.profileSel = devProfile
                            }
                            onDevProfileChanged: syncIndex()
                            onCountChanged: syncIndex()
                            Component.onCompleted: syncIndex()
                            onActivated: win.profileSel = currentIndex + 1
                        }
                        PillButton {
                            text: "切换"
                            enabled: !backend.busy && win.hasState()
                            onClicked: {
                                if (win.profileSel !== (stateOf().device ? stateOf().device.current_profile : -1))
                                    backend.switchProfile(win.profileSel)
                            }
                        }
                        Item { Layout.fillWidth: true }
                        PillButton {
                            text: "保存到 Flash"
                            primary: true
                            enabled: !backend.busy && win.hasState()
                            onClicked: backend.saveFlash()
                        }
                        PillButton {
                            text: "恢复出厂"
                            danger: true
                            enabled: !backend.busy && win.hasState()
                            onClicked: win.confirmReset = true
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        color: Theme.sub
                        font.pixelSize: Theme.fontSmall
                        wrapMode: Text.WordWrap
                        text: "改动只在点「保存到 Flash」后才断电保持；恢复出厂会清空全部配置档。"
                    }
                }

                Item { implicitHeight: Theme.gap * 2 }
            }
        }

        // ── 底部状态栏
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 40
            color: Theme.card
            Rectangle {
                anchors.top: parent.top
                width: parent.width; height: 1
                color: Theme.line
            }
            Text {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad + 4
                anchors.rightMargin: Theme.pad
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
                color: backend.error ? Theme.warn : Theme.sub
                font.pixelSize: Theme.fontSmall
                text: backend.status
            }
        }
    }

    // ── 恢复出厂二次确认（不用系统对话框，样式统一）
    Rectangle {
        anchors.fill: parent
        visible: win.confirmReset
        color: Qt.rgba(0, 0, 0, 0.55)
        MouseArea { anchors.fill: parent }   // 吃掉点击

        Rectangle {
            anchors.centerIn: parent
            width: 380
            height: 170
            radius: Theme.radius
            color: Theme.card
            border.color: Theme.line
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.pad + 2
                spacing: Theme.gap
                Text {
                    Layout.fillWidth: true
                    text: "恢复出厂设置？"
                    color: Theme.fg
                    font.pixelSize: Theme.fontTitle
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: "会清空全部配置档并恢复设备出厂值，不可撤销。"
                    color: Theme.sub
                    font.pixelSize: Theme.fontBody
                    wrapMode: Text.WordWrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gap
                    Item { Layout.fillWidth: true }
                    PillButton {
                        text: "取消"
                        onClicked: win.confirmReset = false
                    }
                    PillButton {
                        text: "确认恢复"
                        danger: true
                        onClicked: {
                            win.confirmReset = false
                            backend.resetFactory()
                        }
                    }
                }
            }
        }
    }
}
