import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import "../theme"

// 按键映射下拉：胶囊里显示**当前功能名**，展开后是可搜索的功能列表（156 项）
Item {
    id: sc
    property string slotName: ""
    property string funcLabel: "—"
    property var options: []          // [{code,label}]
    property int curCode: -1
    signal picked(int code)

    implicitWidth: 108
    implicitHeight: 31

    Rectangle {
        anchors.fill: parent
        radius: Theme.r
        color: hoverArea.containsMouse ? "#20242E" : Theme.ctrl
        border.width: 1
        border.color: Theme.border
    }
    Text {
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.right: caret.left
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        text: sc.funcLabel
        color: Theme.text
        font.family: Theme.font
        font.pixelSize: Theme.fsBody
        elide: Text.ElideRight
    }
    Shape {
        id: caret
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
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            sc.filterText = ""
            pop.open()
        }
    }

    property string filterText: ""

    function rebuild() {
        var src = sc.options || []
        var q = sc.filterText.trim().toLowerCase()
        var out = []
        for (var i = 0; i < src.length; ++i) {
            var it = src[i]
            if (q === "" || String(it.label).toLowerCase().indexOf(q) >= 0) out.push(it)
        }
        listModel.clear()
        for (var j = 0; j < out.length; ++j) {
            listModel.append({ code: out[j].code, label: out[j].label })
        }
    }
    onOptionsChanged: rebuild()
    onFilterTextChanged: rebuild()

    ListModel { id: listModel }

    Popup {
        id: pop
        width: 250
        height: 320
        x: Math.max(8, Math.min(sc.width - width, 0))
        y: sc.height + 4
        padding: 8
        // 贴近窗口底部时向上弹
        onAboutToShow: {
            if (sc.mapToItem(null, 0, 0).y + sc.height + height > 700) y = -height - 4
            else y = sc.height + 4
            sc.rebuild()
        }
        background: Rectangle {
            color: Theme.panel
            radius: 8
            border.width: 1
            border.color: Theme.border
        }
        contentItem: Column {
            spacing: 6
            TextField {
                id: filterField
                width: parent.width
                height: 28
                text: sc.filterText
                placeholderText: "搜索功能…"
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: Theme.fsBody
                selectByMouse: true
                onTextChanged: sc.filterText = text
                background: Rectangle {
                    color: Theme.ctrl
                    radius: Theme.r
                    border.width: 1
                    border.color: Theme.border
                }
            }
            ListView {
                id: view
                width: parent.width
                height: parent.height - 34
                clip: true
                model: listModel
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Item {
                    width: view.width
                    height: 28
                    Rectangle {
                        anchors.fill: parent
                        radius: 4
                        color: (model.code === sc.curCode) ? Theme.ctrl : (rowHover.containsMouse ? "#20242E" : "transparent")
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 8
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        text: model.label
                        color: (model.code === sc.curCode) ? Theme.accent : Theme.text
                        font.family: Theme.font
                        font.pixelSize: Theme.fsBody
                        elide: Text.ElideRight
                    }
                    MouseArea {
                        id: rowHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            sc.picked(model.code)
                            pop.close()
                        }
                    }
                }
            }
        }
        onOpened: {
            filterField.forceActiveFocus()
            // 定位到当前项
            for (var i = 0; i < listModel.count; ++i) {
                if (listModel.get(i).code === sc.curCode) { view.positionViewAtIndex(i, ListView.Center); break }
            }
        }
    }
}
