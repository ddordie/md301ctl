import QtQuick
import "../theme"

// 圆角按钮（主/次/危险三态）
Rectangle {
    id: btn
    property string text: ""
    property bool primary: false
    property bool danger: false
    property bool enabled2: enabled
    signal clicked()

    implicitWidth: label.implicitWidth + 26
    implicitHeight: 30
    radius: Theme.radiusSm
    color: !enabled ? Theme.cardHover
         : danger ? Theme.danger
         : primary ? Theme.accent
         : Theme.cardHover
    border.color: primary || danger ? "transparent" : Theme.line
    border.width: 1
    opacity: enabled ? 1.0 : 0.5

    Text {
        id: label
        anchors.centerIn: parent
        text: btn.text
        font.pixelSize: Theme.fontBody
        font.bold: btn.primary
        color: btn.primary || btn.danger ? Theme.accentFg : Theme.fg
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        enabled: btn.enabled
        hoverEnabled: true
        onClicked: btn.clicked()
    }
}
