import QtQuick
import "../theme"

// 动作按钮：强调色（应用/保存/切换）、危险色（恢复出厂）、灰描边（禁用态/次要）
Item {
    id: btn
    property string text: ""
    property string kind: "accent"      // accent | danger | ghost
    property bool hovered: hoverArea.containsMouse
    property bool active: true          // false → 灰描边 + 灰字
    signal clicked()

    implicitWidth: 100
    implicitHeight: 31

    Rectangle {
        anchors.fill: parent
        radius: Theme.r
        color: {
            if (!btn.active || btn.kind === "ghost") return "transparent"
            if (!btn.hovered) return btn.kind === "danger" ? Theme.danger : Theme.accent
            return btn.kind === "danger" ? Qt.lighter(Theme.danger, 1.15) : Theme.accentHover
        }
        border.width: (!btn.active || btn.kind === "ghost") ? 1 : 0
        border.color: Theme.border
        Behavior on color { ColorAnimation { duration: 90 } }
    }
    Text {
        anchors.centerIn: parent
        text: btn.text
        color: btn.active ? Theme.text : Theme.muted
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
    }
    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: (btn.active && btn.kind !== "ghost") ? Qt.PointingHandCursor : Qt.ArrowCursor
        enabled: btn.active && btn.kind !== "ghost"
        onClicked: btn.clicked()
    }
}
