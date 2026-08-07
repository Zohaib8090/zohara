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
            text: qsTr("Bluetooth & devices")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        SettingsCard {
            title: qsTr("Bluetooth")
            subtitle: bluetoothManager.isAdapterOn() ? qsTr("Discoverable as Zohara") : qsTr("Off")
            
            control: Switch {
                checked: bluetoothManager.isAdapterOn()
                onCheckedChanged: bluetoothManager.setAdapterOn(checked)
            }
        }
        
        Text {
            text: qsTr("Paired Devices")
            font.pixelSize: 18
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.topMargin: 12
            visible: bluetoothManager.isAdapterOn()
        }
        
        Repeater {
            model: bluetoothManager.isAdapterOn() ? bluetoothManager.devices : []
            SettingsCard {
                title: modelData.name
                subtitle: modelData.connected ? qsTr("Connected") : qsTr("Paired")
                
                control: RowLayout {
                    spacing: 8
                    PrimaryButton {
                        text: modelData.connected ? qsTr("Disconnect") : qsTr("Connect")
                        onClicked: {
                            if (modelData.connected) {
                                bluetoothManager.disconnectDevice(modelData.mac)
                            } else {
                                bluetoothManager.connectDevice(modelData.mac)
                            }
                        }
                    }
                    PrimaryButton {
                        text: qsTr("Forget")
                        onClicked: bluetoothManager.forgetDevice(modelData.mac)
                    }
                }
            }
        }
        
        Text {
            text: qsTr("Device History")
            font.pixelSize: 18
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.topMargin: 12
        }
        
        Repeater {
            model: bluetoothManager.history
            SettingsCard {
                title: modelData.name
                subtitle: qsTr("Last action: ") + modelData.event + " (" + modelData.timestamp.substring(0, 10) + ")"
            }
        }
    }
}
