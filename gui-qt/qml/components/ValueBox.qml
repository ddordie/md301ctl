import QtQuick
import "../theme"

// DPI 数值框（设计稿里是描边小方框，当前档用强调色文字）
Item {
    id: box
    property string text: "—"
    property bool accent: false
    implicitWidth: 56
    implicitHeight: 31

    Rectangle {
        anchors.fill: parent
        radius: Theme.r
        color: "transparent"
        border.width: 1
        border.color: Theme.border
    }
    Text {
        anchors.centerIn: parent
        text: box.text
        color: box.accent ? Theme.accent : Theme.muted
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
    }
}
