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
            text: qsTr("Network & internet")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("Wi-Fi")
            subtitle: networkManager.isWifiEnabled() ? qsTr("Connected / Available") : qsTr("Off")
            
            control: Switch {
                checked: networkManager.isWifiEnabled()
                onCheckedChanged: networkManager.setWifiEnabled(checked)
            }
        }
        
        SettingsCard {
            title: qsTr("Ethernet")
            subtitle: networkManager.ethernetStatus
        }
        
        Text {
            text: qsTr("Available Networks")
            font.pixelSize: 18
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.topMargin: 12
            visible: networkManager.isWifiEnabled()
        }
        
        Repeater {
            model: networkManager.isWifiEnabled() ? networkManager.networks : []
            SettingsCard {
                title: modelData.ssid
                subtitle: modelData.security + " • Signal: " + modelData.signal + "%"
                
                control: PrimaryButton {
                    text: modelData.connected ? qsTr("Disconnect") : qsTr("Connect")
                    onClicked: {
                        if (modelData.connected) {
                            networkManager.disconnectNetwork(modelData.ssid)
                        } else {
                            // Stub for connecting (no password prompt yet)
                            networkManager.connectToWifi(modelData.ssid, "")
                        }
                    }
                }
            }
        }
    }
}
