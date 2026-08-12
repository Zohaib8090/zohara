import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"
import ".."

ScrollView {
    id: root
    clip: true
    contentWidth: availableWidth

    ColumnLayout {
        width: root.availableWidth
        spacing: 12

        Text {
            text: qsTr("Time & language")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        Text {
            text: qsTr("Date & time")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
        }

        SettingsCard {
            title: qsTr("Timezone")
            subtitle: timeManager.timezone

            PrimaryButton {
                text: qsTr("Change")
                onClicked: tzPopup.open()
            }
        }

        // ── Timezone popup ────────────────────────────────────────────────────
        Popup {
            id: tzPopup
            anchors.centerIn: Overlay.overlay
            width: 380
            height: 440
            modal: true
            closePolicy: Popup.CloseOnEscape

            background: Rectangle {
                color: Theme.surface
                border.color: Theme.border
                radius: Theme.radius
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 10

                Text {
                    text: qsTr("Select timezone")
                    font.pixelSize: 15
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
                        id: tzSearch
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        verticalAlignment: TextInput.AlignVCenter
                        font.pixelSize: 13
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: Theme.text
                        selectionColor: Theme.accent

                        Text {
                            anchors.fill: parent
                            verticalAlignment: Text.AlignVCenter
                            text: qsTr("Search timezone…")
                            color: Theme.textSecondary
                            font: parent.font
                            visible: !parent.text
                        }
                    }
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: {
                        var all = timeManager.listTimezones()
                        if (tzSearch.text === "") return all
                        return all.filter(function(t) {
                            return t.toLowerCase().includes(tzSearch.text.toLowerCase())
                        })
                    }

                    delegate: Rectangle {
                        width: ListView.view.width
                        height: 36
                        color: hoverArea.containsMouse ? Theme.navItemHover : "transparent"
                        radius: Theme.radiusSmall

                        Text {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            verticalAlignment: Text.AlignVCenter
                            text: modelData
                            font.pixelSize: 13
                            font.family: "Inter, Segoe UI, sans-serif"
                            color: Theme.text
                        }

                        MouseArea {
                            id: hoverArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                timeManager.setTimezone(modelData)
                                tzPopup.close()
                            }
                        }
                    }
                }

                PrimaryButton {
                    text: qsTr("Cancel")
                    Layout.alignment: Qt.AlignRight
                    onClicked: tzPopup.close()
                }
            }
        }

        Text {
            text: qsTr("Language & keyboard")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
        }

        SettingsCard {
            title: qsTr("System locale")
            subtitle: timeManager.locale
        }

        SettingsCard {
            title: qsTr("Keyboard layout")
            subtitle: timeManager.keyboardLayout
        }

        Item { height: 24 }
    }
}
