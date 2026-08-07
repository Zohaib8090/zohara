import QtQuick
import QtQuick.Controls
import ".." // Theme

Button {
    id: control
    
    contentItem: Text {
        text: control.text
        font: control.font
        opacity: enabled ? 1.0 : 0.3
        color: Theme.buttonText
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        implicitWidth: 100
        implicitHeight: 32
        opacity: enabled ? 1 : 0.3
        color: control.down ? Theme.accent : (control.hovered ? Theme.buttonHover : Theme.buttonBackground)
        radius: 4
        border.color: Theme.border
        border.width: 1
    }
}
