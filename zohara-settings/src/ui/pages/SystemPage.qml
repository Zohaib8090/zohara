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
            text: qsTr("System")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("Display")
            subtitle: qsTr("Monitors, brightness, and scale")
        }
        
        SettingsCard {
            title: qsTr("Sound")
            subtitle: qsTr("Volume levels, output, input")
        }
        
        SettingsCard {
            title: qsTr("Power & battery")
            subtitle: qsTr("Sleep, battery usage, profiles")
            
            control: ComboBox {
                model: ["Performance", "Balanced", "Battery Saver"]
                currentIndex: 1
            }
        }
        
        SettingsCard {
            title: qsTr("About")
            subtitle: qsTr("Device specifications")
            
            control: ColumnLayout {
                spacing: 4
                Text { color: Theme.textSecondary; text: "OS: " + systemManager.getOsVersion() }
                Text { color: Theme.textSecondary; text: "Kernel: " + systemManager.getKernelVersion() }
                Text { color: Theme.textSecondary; text: "CPU: " + systemManager.getCpuModel() }
                Text { color: Theme.textSecondary; text: "RAM: " + systemManager.getMemoryTotal() }
            }
        }
    }
}

