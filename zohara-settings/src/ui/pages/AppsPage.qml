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
            text: qsTr("Apps")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 4
        }

        // ── Filter bar ────────────────────────────────────────────────────────
        RowLayout {
            Layout.bottomMargin: 8
            spacing: 12

            Rectangle {
                Layout.fillWidth: true
                height: 36
                radius: Theme.radiusSmall
                color: Theme.surfaceHigh
                border.color: Theme.border

                TextInput {
                    id: appSearch
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
                        text: qsTr("Search installed apps…")
                        color: Theme.textSecondary
                        font: parent.font
                        visible: !parent.text
                    }
                }
            }

            ComboBox {
                id: sourceFilter
                model: ["All", "Native", "Flatpak", "Android"]
                implicitWidth: 110
                font.family: "Inter, Segoe UI, sans-serif"
                contentItem: Text {
                    leftPadding: 8
                    text: sourceFilter.displayText
                    font: sourceFilter.font
                    color: Theme.text
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    implicitHeight: 36
                    color: Theme.buttonBg
                    border.color: Theme.border
                    radius: Theme.radiusSmall
                }
            }

            PrimaryButton {
                text: qsTr("Refresh")
                onClicked: appsManager.rescan()
            }
        }

        // Scanning indicator
        Rectangle {
            Layout.fillWidth: true
            height: 36
            radius: Theme.radiusSmall
            color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.1)
            border.color: Theme.accent
            border.width: 1
            visible: appsManager.scanning

            RowLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10
                BusyIndicator { running: appsManager.scanning; implicitHeight: 20; implicitWidth: 20 }
                Text {
                    text: qsTr("Scanning installed apps…")
                    font.pixelSize: 12; font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                }
            }
        }

        // Count
        Text {
            text: filteredApps.length + qsTr(" apps")
            font.pixelSize: 12
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            visible: filteredApps.length > 0
        }

        // ── Computed filtered list ────────────────────────────────────────────
        property var filteredApps: {
            var q = appSearch.text.toLowerCase()
            var src = sourceFilter.currentText
            return appsManager.apps.filter(function(a) {
                var matchQ = q === "" || a.name.toLowerCase().includes(q)
                var matchS = src === "All" || a.source === src
                return matchQ && matchS
            })
        }

        Repeater {
            model: parent.filteredApps
            SettingsCard {
                title: modelData.name
                subtitle: modelData.source + " • " + modelData.size

                PrimaryButton {
                    text: qsTr("Uninstall")
                    // Disable for system libs (no install date means it's a dep)
                    enabled: modelData.source !== "Native" ||
                             (modelData.name.length > 0)
                    onClicked: console.log("Uninstall: " + modelData.name)
                }
            }
        }

        Item { height: 24 }
    }
}
