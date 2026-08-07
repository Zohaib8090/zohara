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
            text: qsTr("Zohara Update")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("System Updates")
            subtitle: updateManager.status
            
            control: PrimaryButton {
                text: qsTr("Check for updates")
                onClicked: {
                    updateManager.checkForUpdates()
                }
            }
        }
    }
}


