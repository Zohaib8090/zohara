import QtQuick
import QtQuick.Controls
import "."  // Theme

Button {
    id: control
    implicitWidth: Math.max(contentItem.implicitWidth + 32, 100)
    implicitHeight: 32

    contentItem: Text {
        text: control.text
        font.pixelSize: 13
        font.family: "Inter, Segoe UI, sans-serif"
        font.weight: Font.Medium
        opacity: enabled ? 1.0 : 0.4
        // Use a stark contrast for text depending on if it's pressed
        color: control.down ? Theme.accentText : Theme.buttonText
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.radiusSmall
        color: control.down    ? Theme.accent :
               control.hovered ? Theme.buttonBgHover :
                                 Theme.buttonBg
        border.color: control.down ? Theme.accentHover : Theme.border
        border.width: 1
        opacity: enabled ? 1 : 0.5

        Behavior on color { ColorAnimation { duration: 150 } }
    }
}
