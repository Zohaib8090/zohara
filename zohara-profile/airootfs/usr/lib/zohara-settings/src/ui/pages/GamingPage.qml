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
            text: qsTr("Gaming")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("GameMode")
            subtitle: gamingManager.gamemodeAvailable ? (gamingManager.gamemodeActive ? qsTr("Active") : qsTr("Available but inactive")) : qsTr("Not installed")
            
            control: Switch {
                checked: gamingManager.gamemodeActive
                enabled: gamingManager.gamemodeAvailable
                onCheckedChanged: {
                    if (enabled && checked !== gamingManager.gamemodeActive) {
                        gamingManager.setGamemode(checked)
                    }
                }
            }
        }
        
        SettingsCard {
            title: qsTr("Wine Compatibility Layer")
            subtitle: qsTr("Status: ") + gamingManager.wineVersion
        }
        
        SettingsCard {
            title: qsTr("Waydroid (Android Apps)")
            subtitle: qsTr("Status: ") + gamingManager.waydroidStatus
        }
    }
}
