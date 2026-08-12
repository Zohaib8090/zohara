import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."  // Theme

Rectangle {
    id: root
    Layout.fillWidth: true
    // Let the height be determined by the inner ColumnLayout
    implicitHeight: mainLayout.implicitHeight + (Theme.cardPadding * 2)
    radius: Theme.radius
    color: Theme.surface
    border.color: Theme.border
    border.width: 1

    property string title: ""
    property string subtitle: ""
    // Any child Item placed here becomes the right-side control
    default property alias control: controlSlot.data

    // Subtle hover effect to make the UI feel alive
    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        // Allow clicks to pass through to controls
        propagateComposedEvents: true
        preventStealing: false
        onClicked: (mouse) => mouse.accepted = false
        onPressed: (mouse) => mouse.accepted = false
    }

    Behavior on color { ColorAnimation { duration: 150 } }
    Behavior on border.color { ColorAnimation { duration: 150 } }

    RowLayout {
        id: mainLayout
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.cardPadding
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: root.title
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.family: "Inter, Segoe UI, sans-serif"
                color: Theme.text
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Text {
                text: root.subtitle
                font.pixelSize: 12
                font.family: "Inter, Segoe UI, sans-serif"
                color: Theme.textSecondary
                visible: text !== ""
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                lineHeight: 1.2
            }
        }

        // Right-hand control slot
        Item {
            id: controlSlot
            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
            implicitWidth: childrenRect.width
            implicitHeight: childrenRect.height
            visible: children.length > 0
        }
    }
}
