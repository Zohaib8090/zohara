import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"
import ".." // Theme

ScrollView {
    clip: true
    
    ColumnLayout {
        width: parent.width - 20
        spacing: 16
        
        Text {
            text: qsTr("Personalization")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("Skin system")
            subtitle: qsTr("Coming soon")
            
            control: ComboBox {
                model: ["Win11", "Win10", "macOS", "Liquid Glass", "KDE Native"]
                enabled: false // Stubbed per spec
                
                contentItem: Text {
                    text: parent.currentText
                    color: Theme.text
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
                background: Rectangle {
                    implicitWidth: 140
                    implicitHeight: 32
                    color: Theme.buttonBackground
                    border.color: Theme.border
                    radius: 4
                }
            }
        }
        
        SettingsCard {
            title: qsTr("Dark Mode Override")
            subtitle: qsTr("Force application dark mode (Requires restart)")
            
            control: Switch {
                checked: Theme.isDark
                onCheckedChanged: Theme.isDark = checked
            }
        }
    }
}
