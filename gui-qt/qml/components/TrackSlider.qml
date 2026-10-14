import QtQuick
import QtQuick.Controls
import "../theme"

// 滑条：细轨道 + 紫色手柄（白描边），与设计稿的 st20/st22 一致
Slider {
    id: s
    implicitHeight: 22

    background: Item {
        x: s.leftPadding
        y: s.topPadding + s.availableHeight / 2 - height / 2
        width: s.availableWidth
        height: 3

        Rectangle {
            width: parent.width
            height: parent.height
            color: Theme.border
        }
        Rectangle {
            width: s.visualPosition * parent.width
            height: parent.height
            color: Theme.accent
        }
    }

    handle: Rectangle {
        x: s.leftPadding + s.visualPosition * (s.availableWidth - width)
        y: s.topPadding + s.availableHeight / 2 - height / 2
        width: 12
        height: 12
        radius: 6
        color: Theme.accent
        border.width: 2
        border.color: Theme.text
    }
}
