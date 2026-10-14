import QtQuick
import QtQuick.Controls
import "../theme"

// 暗色下拉框：Qt 默认 Fusion 样式是白底，在暗色卡片上非常突兀，
// 这里统一背景/文字/弹出列表/滚动条。
ComboBox {
    id: combo
    implicitHeight: 30
    // 还没读到设备值时，不要显示第一项（会假装成 Escape），显示占位文字
    property string placeholder: ""

    contentItem: Text {
        leftPadding: 11
        rightPadding: combo.indicator.width + 8
        text: combo.currentIndex >= 0 ? combo.displayText : combo.placeholder
        color: combo.currentIndex >= 0 ? Theme.fg : Theme.sub
        font.pixelSize: Theme.fontBody
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    indicator: Text {
        x: combo.width - width - 11
        y: combo.topPadding + (combo.availableHeight - height) / 2
        text: "\u2304"
        color: Theme.sub
        font.pixelSize: 13
    }

    background: Rectangle {
        radius: Theme.radiusSm
        color: combo.hovered ? Theme.cardHover : Theme.bg
        border.color: combo.activeFocus ? Theme.accent : Theme.line
        border.width: 1
    }

    delegate: ItemDelegate {
        id: row
        required property var model
        required property int index
        width: combo.width
        height: 30
        highlighted: combo.highlightedIndex === index

        contentItem: Text {
            leftPadding: 11
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
            // 兼容两种 model：对象数组取 label，纯值数组取 modelData
            text: (combo.currentIndex === index ? "✓ " : "") +
                  (model && model.label !== undefined ? model.label
                   : (modelData !== undefined ? modelData : ""))
            // 悬停/键盘高亮 = 亮白；当前选中项 = 强调色（否则看不出哪项是当前值）
            color: row.highlighted ? Theme.fg
                 : (combo.currentIndex === index ? Theme.accent : Theme.sub)
            font.pixelSize: Theme.fontBody
        }
        background: Rectangle {
            color: row.highlighted ? Theme.cardHover : Theme.card
        }
    }

    popup: Popup {
        y: combo.height + 2
        width: combo.width
        implicitHeight: Math.min(contentItem.implicitHeight + 6, 340)
        padding: 3

        background: Rectangle {
            radius: Theme.radiusSm
            color: Theme.card
            border.color: Theme.line
            border.width: 1
        }
        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: combo.popup.visible ? combo.delegateModel : null
            currentIndex: combo.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
            ScrollIndicator.vertical: ScrollIndicator { }
        }
    }
}
