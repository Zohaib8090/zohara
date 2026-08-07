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
            text: qsTr("Installed Apps")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        Repeater {
            model: appsManager.apps
            SettingsCard {
                title: modelData.name
                subtitle: modelData.source + " - " + modelData.size
                
                control: PrimaryButton {
                    text: qsTr("Uninstall")
                }
            }
        }
    }
}


