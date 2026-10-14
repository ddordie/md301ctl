import QtQuick
import QtQuick.Layouts
import "../theme"

// 一张卡片：可选标题 + 内容（默认属性就是内部 ColumnLayout 的内容）
Rectangle {
    id: card
    default property alias content: holder.data
    property alias spacing: holder.spacing
    property alias title: titleText.text
    property alias titleVisible: titleText.visible

    radius: Theme.radius
    color: Theme.card
    border.color: Theme.line
    border.width: 1
    implicitHeight: holder.implicitHeight + Theme.pad * 2
    implicitWidth: 200
    spacing: Theme.gap

    ColumnLayout {
        id: holder
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.pad
        spacing: card.spacing

        Text {
            id: titleText
            visible: false
            color: Theme.fg
            font.pixelSize: Theme.fontTitle
            font.bold: true
            Layout.fillWidth: true
        }
    }
}
