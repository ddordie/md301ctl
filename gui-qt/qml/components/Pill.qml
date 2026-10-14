import QtQuick
import "../theme"

// 选项胶囊（回报率 / 按键响应 / 休眠档）。
// 设计稿语义：**当前值** = 无填充 + 灰描边 + 灰字（不可点）；其余 = 强调色实心。
Item {
    id: pill
    property string text: ""
    property bool current: false
    property bool hovered: hoverArea.containsMouse
    signal clicked()

    implicitWidth: 60
    implicitHeight: 31

    Rectangle {
        anchors.fill: parent
        radius: Theme.r
        color: pill.current ? "transparent" : (pill.hovered ? Theme.accentHover : Theme.accent)
        border.width: pill.current ? 1 : 0
        border.color: Theme.border
        Behavior on color { ColorAnimation { duration: 90 } }
    }
    Text {
        anchors.centerIn: parent
        text: pill.text
        color: pill.current ? Theme.muted : Theme.text
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
    }
    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: pill.current ? Qt.ArrowCursor : Qt.PointingHandCursor
        onClicked: if (!pill.current) pill.clicked()
    }
}
