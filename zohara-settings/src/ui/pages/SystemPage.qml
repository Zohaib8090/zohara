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

        // ── Header ────────────────────────────────────────────────────────────
        Text {
            text: qsTr("System")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        // ── Power & Battery ───────────────────────────────────────────────────
        Text {
            text: qsTr("Power & battery")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 4
        }

        SettingsCard {
            title: qsTr("Power mode")
            subtitle: qsTr("Choose a power profile to balance performance and energy use")

            ComboBox {
                id: profileCombo
                model: ["Performance", "Balanced", "Battery Saver"]
                currentIndex: 1
                font.family: "Inter, Segoe UI, sans-serif"
                onCurrentTextChanged: powerManager.setProfile(currentText)
                Component.onCompleted: {
                    var gov = powerManager.getCurrentGovernor()
                    currentIndex = model.indexOf(gov)
                    if (currentIndex < 0) currentIndex = 1
                }
                contentItem: Text {
                    leftPadding: 8
                    text: profileCombo.displayText
                    font: profileCombo.font
                    color: Theme.text
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    implicitWidth: 160
                    implicitHeight: 34
                    color: Theme.buttonBg
                    border.color: Theme.border
                    radius: Theme.radiusSmall
                }
            }
        }

        SettingsCard {
            title: qsTr("Battery")
            subtitle: powerManager.batteryPercent < 0
                      ? qsTr("No battery detected (Desktop / AC only)")
                      : (powerManager.batteryPercent + "%" + (powerManager.batteryCharging ? " — Charging" : " — Discharging"))
            Component.onCompleted: powerManager.refreshBattery()
        }

        // ── Display ───────────────────────────────────────────────────────────
        Text {
            text: qsTr("Display")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
        }

        SettingsCard {
            title: qsTr("Display settings")
            subtitle: qsTr("Resolution, refresh rate, and arrangement (managed by KDE)")
            PrimaryButton {
                text: qsTr("Open display settings")
                onClicked: Qt.openUrlExternally("kcm5://kcm_kscreen")
            }
        }

        SettingsCard {
            title: qsTr("Sound")
            subtitle: qsTr("Volume, output, and input devices (managed by KDE)")
            PrimaryButton {
                text: qsTr("Open sound settings")
                onClicked: Qt.openUrlExternally("kcm5://kcm_pulseaudio")
            }
        }

        // ── About ─────────────────────────────────────────────────────────────
        Text {
            text: qsTr("About")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
        }

        SettingsCard {
            title: systemManager.getOsVersion() + (systemManager.getCodename() !== "" ? " (" + systemManager.getCodename() + ")" : "")
            subtitle: qsTr("OS Version")
        }

        SettingsCard {
            title: systemManager.getKernelVersion()
            subtitle: qsTr("Kernel")
        }

        SettingsCard {
            title: systemManager.getCpuModel() + " • " + systemManager.getCpuCores() + "C / " + systemManager.getCpuThreads() + "T"
            subtitle: qsTr("Processor")
        }

        SettingsCard {
            title: systemManager.getMemoryTotal()
            subtitle: qsTr("Installed RAM")
        }

        Repeater {
            model: systemManager.getGpus()
            SettingsCard {
                title: modelData.name
                subtitle: modelData.type + " GPU"
            }
        }

        SettingsCard {
            title: qsTr("Copy specs to clipboard")
            PrimaryButton {
                text: qsTr("Copy")
                onClicked: {
                    var text = systemManager.copySpecsToClipboard()
                    // Access clipboard via Qt
                    Qt.application.clipboard ? Qt.application.clipboard.text = text : console.log(text)
                }
            }
        }

        Item { height: 24 }
    }
}
