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
            text: qsTr("Zohara Update")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        // ── Status card ───────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 80
            radius: Theme.radius
            color: Theme.surface
            border.color: Theme.border
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 16

                Text {
                    text: updateManager.updates.length === 0 ? "●" : "↑"
                    font.pixelSize: 36
                    font.weight: Font.Bold
                    color: updateManager.updates.length === 0 ? Theme.accentGreen : Theme.accentOrange
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: updateManager.status
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: Theme.text
                    }

                    ProgressBar {
                        Layout.fillWidth: true
                        from: 0; to: 100
                        value: updateManager.progress
                        visible: updateManager.progress > 0 && updateManager.progress < 100
                    }
                }

                RowLayout {
                    spacing: 12
                    PrimaryButton {
                        text: qsTr("Check")
                        onClicked: updateManager.checkForUpdates()
                    }
                    PrimaryButton {
                        text: qsTr("Install all")
                        enabled: updateManager.updates.length > 0
                        onClicked: updateManager.installUpdates()
                    }
                }
            }
        }

        // ── Restart banner ────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 44
            radius: Theme.radiusSmall
            color: Qt.rgba(Theme.accentOrange.r, Theme.accentOrange.g, Theme.accentOrange.b, 0.15)
            border.color: Theme.accentOrange
            border.width: 1
            visible: updateManager.restartRequired

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                Text {
                    text: "↻"
                    font.pixelSize: 22
                    font.weight: Font.Bold
                    color: Theme.accentOrange
                }
                Text {
                    text: qsTr("A restart is required to complete the update.")
                    font.pixelSize: 13
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                    Layout.fillWidth: true
                }
            }
        }

        // ── Available updates list ────────────────────────────────────────────
        Text {
            text: qsTr("Available updates (%1)").arg(updateManager.updates.length)
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
            visible: updateManager.updates.length > 0
        }

        Repeater {
            model: updateManager.updates
            SettingsCard {
                title: modelData.name
                subtitle: modelData.current + "  →  " + modelData.new
            }
        }

        // ── History ───────────────────────────────────────────────────────────
        RowLayout {
            Layout.topMargin: 16

            Text {
                text: qsTr("Recent changes")
                font.pixelSize: 13
                font.weight: Font.DemiBold
                font.family: "Inter, Segoe UI, sans-serif"
                color: Theme.textSecondary
                Layout.fillWidth: true
            }
            PrimaryButton {
                text: qsTr("Load history")
                onClicked: updateManager.loadHistory()
            }
        }

        Repeater {
            model: updateManager.pacmanLog.slice(0, 30)
            SettingsCard {
                title: modelData.package
                subtitle: modelData.action.charAt(0).toUpperCase() + modelData.action.slice(1) +
                          " — " + modelData.date.substring(0, 16).replace("T", " ")
            }
        }

        Item { height: 24 }
    }
}
