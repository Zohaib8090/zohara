import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import ".." // Theme

Rectangle {
    id: root
    Layout.fillWidth: true
    implicitHeight: layout.implicitHeight + 32
    radius: 8
    color: Theme.surface
    border.color: Theme.border
    border.width: 1
    
    property string title: ""
    property string subtitle: ""
    property Item control: null
    
    RowLayout {
        id: layout
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16
        
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            
            Text {
                text: root.title
                font.pixelSize: 14
                font.weight: Font.Medium
                color: Theme.text
            }
            
            Text {
                text: root.subtitle
                font.pixelSize: 12
                color: Theme.textSecondary
                visible: text !== ""
            }
        }
        
        // Inject custom control (switch, combo box, button, etc)
        Item {
            id: controlContainer
            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
            implicitWidth: root.control ? root.control.implicitWidth : 0
            implicitHeight: root.control ? root.control.implicitHeight : 0
            
            Component.onCompleted: {
                if (root.control) {
                    root.control.parent = controlContainer
                    root.control.anchors.centerIn = controlContainer
                }
            }
        }
    }
}

