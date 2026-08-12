import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"
import ".."

ScrollView {
    id: root
    clip: true
    contentWidth: availableWidth

    Component.onCompleted: bluetoothManager.refreshDevices()

    ColumnLayout {
        width: root.availableWidth
        spacing: 12

        Text {
            text: qsTr("Bluetooth & devices")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        SettingsCard {
            title: qsTr("Bluetooth")
            subtitle: bluetoothManager.adapterOn ? qsTr("Discoverable as Zohara") : qsTr("Off")

            Switch {
                checked: bluetoothManager.adapterOn
                onClicked: bluetoothManager.setAdapterOn(checked)
            }
        }

        RowLayout {
            Layout.topMargin: 12
            visible: bluetoothManager.adapterOn

            Text {
                text: qsTr("Paired devices")
                font.pixelSize: 13
                font.weight: Font.DemiBold
                font.family: "Inter, Segoe UI, sans-serif"
                color: Theme.textSecondary
                Layout.fillWidth: true
            }

            PrimaryButton {
                text: qsTr("Refresh")
                onClicked: bluetoothManager.refreshDevices()
            }
        }

        Repeater {
            model: bluetoothManager.adapterOn ? bluetoothManager.devices : []
            SettingsCard {
                title: modelData.name
                subtitle: modelData.connected ? qsTr("● Connected") : qsTr("Paired, not connected")

                RowLayout {
                    spacing: 12
                    PrimaryButton {
                        text: modelData.connected ? qsTr("Disconnect") : qsTr("Connect")
                        onClicked: modelData.connected
                                   ? bluetoothManager.disconnectDevice(modelData.mac)
                                   : bluetoothManager.connectDevice(modelData.mac)
                    }
                    PrimaryButton {
                        text: qsTr("Forget")
                        onClicked: bluetoothManager.forgetDevice(modelData.mac)
                    }
                }
            }
        }

        Text {
            text: qsTr("No paired devices")
            font.pixelSize: 13
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
            Layout.topMargin: 8
            visible: bluetoothManager.adapterOn && bluetoothManager.devices.length === 0
        }

        Text {
            text: qsTr("Device history")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 16
            visible: bluetoothManager.history.length > 0
        }

        Repeater {
            model: bluetoothManager.history.slice().reverse()
            SettingsCard {
                title: modelData.name
                subtitle: modelData.event.charAt(0).toUpperCase() + modelData.event.slice(1) +
                          " — " + modelData.timestamp.substring(0, 10)
            }
        }

        Item { height: 24 }
    }
}
