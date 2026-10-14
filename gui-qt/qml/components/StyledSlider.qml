import QtQuick
import QtQuick.Controls
import "../theme"

// 自己画的滑块：默认 Fusion 样式的把手对比度太低，这里统一成主题色
Slider {
    id: root
    property int minValue: 100
    property int maxValue: 8000

    from: minValue
    to: maxValue
    stepSize: 50
    implicitHeight: 26

    background: Rectangle {
        x: 0
        y: (root.height - 4) / 2
        width: root.width
        height: 4
        radius: 2
        color: Theme.track

        Rectangle {
            width: root.enabled ? root.visualPosition * parent.width : 0
            height: parent.height
            radius: 2
            color: Theme.accent
        }
    }

    handle: Rectangle {
        x: root.visualPosition * (root.width - width)
        y: (root.height - height) / 2
        width: 16
        height: 16
        radius: 8
        color: "#FFFFFF"
        border.color: Theme.accent
        border.width: 2
    }
}
