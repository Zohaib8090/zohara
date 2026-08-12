import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"
import ".."

ScrollView {
    id: root
    clip: true
    contentWidth: availableWidth

    // Load network data when page opens
    Component.onCompleted: {
        networkManager.refreshEthernet()
        if (networkManager.wifiEnabled) networkManager.refreshNetworks()
    }

    ColumnLayout {
        width: root.availableWidth
        spacing: 12

        Text {
            text: qsTr("Network & internet")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        // ── Wi-Fi ─────────────────────────────────────────────────────────────
        SettingsCard {
            title: qsTr("Wi-Fi")
            subtitle: networkManager.wifiEnabled ? qsTr("On") : qsTr("Off")

            Switch {
                checked: networkManager.wifiEnabled
                onClicked: networkManager.setWifiEnabled(checked)
            }
        }

        // ── Ethernet ──────────────────────────────────────────────────────────
        SettingsCard {
            title: qsTr("Ethernet")
            subtitle: networkManager.ethernetStatus

            PrimaryButton {
                text: qsTr("Refresh")
                onClicked: networkManager.refreshEthernet()
            }
        }

        // ── Wi-Fi Networks ────────────────────────────────────────────────────
        RowLayout {
            Layout.topMargin: 12
            visible: networkManager.wifiEnabled

            Text {
                text: qsTr("Available networks")
                font.pixelSize: 13
                font.weight: Font.DemiBold
                font.family: "Inter, Segoe UI, sans-serif"
                color: Theme.textSecondary
                Layout.fillWidth: true
            }

            PrimaryButton {
                text: qsTr("Scan")
                onClicked: networkManager.refreshNetworks()
                visible: networkManager.wifiEnabled
            }
        }

        // Password dialog
        Popup {
            id: passwordDialog
            anchors.centerIn: Overlay.overlay
            width: 340
            modal: true
            closePolicy: Popup.CloseOnEscape

            property string targetSsid: ""

            background: Rectangle {
                color: Theme.surface
                border.color: Theme.border
                radius: Theme.radius
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 12

                Text {
                    text: qsTr("Connect to ") + passwordDialog.targetSsid
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 36
                    radius: Theme.radiusSmall
                    color: Theme.surfaceHigh
                    border.color: Theme.border

                    TextInput {
                        id: passwordField
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        verticalAlignment: TextInput.AlignVCenter
                        font.pixelSize: 13
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: Theme.text
                        echoMode: TextInput.Password
                        selectionColor: Theme.accent

                        Text {
                            anchors.fill: parent
                            verticalAlignment: Text.AlignVCenter
                            text: qsTr("Password")
                            color: Theme.textSecondary
                            font: parent.font
                            visible: !parent.text
                        }
                    }
                }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 12

                    PrimaryButton {
                        text: qsTr("Cancel")
                        onClicked: passwordDialog.close()
                    }
                    PrimaryButton {
                        text: qsTr("Connect")
                        onClicked: {
                            networkManager.connectToWifi(passwordDialog.targetSsid, passwordField.text)
                            passwordDialog.close()
                        }
                    }
                }
            }
        }

        Repeater {
            model: networkManager.networks
            visible: networkManager.wifiEnabled

            SettingsCard {
                title: modelData.ssid
                subtitle: modelData.security + " • " + modelData.signal + "% signal"

                RowLayout {
                    spacing: 12

                    // Signal icon
                    Text {
                        text: "●"
                        font.pixelSize: 18
                        color: modelData.connected ? Theme.accentGreen : Theme.textSecondary
                    }

                    PrimaryButton {
                        text: modelData.connected ? qsTr("Disconnect") : qsTr("Connect")
                        onClicked: {
                            if (modelData.connected) {
                                networkManager.disconnectNetwork(modelData.ssid)
                            } else if (modelData.security !== "Open") {
                                passwordDialog.targetSsid = modelData.ssid
                                passwordField.text = ""
                                passwordDialog.open()
                            } else {
                                networkManager.connectToWifi(modelData.ssid, "")
                            }
                        }
                    }
                }
            }
        }

        Item { height: 24 }
    }
}
