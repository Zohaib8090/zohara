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
            text: qsTr("Time & language")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("Timezone")
            subtitle: timeManager.timezone
            
            control: PrimaryButton {
                text: qsTr("Change")
                onClicked: {
                    // Logic to pop open timezone selector
                    console.log("Timezone selector clicked")
                }
            }
        }
        
        SettingsCard {
            title: qsTr("System Locale")
            subtitle: timeManager.locale
            
            control: PrimaryButton {
                text: qsTr("Change")
            }
        }
        
        SettingsCard {
            title: qsTr("Keyboard Layout")
            subtitle: timeManager.keyboardLayout
            
            control: PrimaryButton {
                text: qsTr("Change")
            }
        }
    }
}
