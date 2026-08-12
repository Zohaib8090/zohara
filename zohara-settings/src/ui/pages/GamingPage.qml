import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"
import ".."

ScrollView {
    id: root
    clip: true
    contentWidth: availableWidth

    Component.onCompleted: {
        gamingManager.refresh()
    }

    ColumnLayout {
        width: root.availableWidth
        spacing: 12

        Text {
            text: qsTr("Gaming")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        SettingsCard {
            title: qsTr("GameMode")
            subtitle: gamingManager.gamemodeAvailable
                      ? (gamingManager.gamemodeActive
                         ? qsTr("Active — CPU/GPU resources prioritised for games")
                         : qsTr("Installed — inactive"))
                      : qsTr("Not installed (install gamemode package)")

            Switch {
                checked: gamingManager.gamemodeActive
                enabled: gamingManager.gamemodeAvailable
                onClicked: gamingManager.setGamemode(checked)
            }
        }

        Text {
            text: qsTr("Compatibility layers")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
        }

        SettingsCard {
            title: qsTr("Wine")
            subtitle: gamingManager.wineVersion

            Text {
                text: gamingManager.wineVersion === "Not installed" ? "!" : "●"
                font.pixelSize: 22
                font.weight: Font.Bold
                color: gamingManager.wineVersion === "Not installed" ? Theme.accentRed : Theme.accentGreen
            }
        }

        SettingsCard {
            title: qsTr("Waydroid (Android compatibility)")
            subtitle: gamingManager.waydroidStatus

            Text {
                text: gamingManager.waydroidStatus === "Running" ? "●" :
                      gamingManager.waydroidStatus === "Stopped" ? "■" : "!"
                font.pixelSize: 22
                font.weight: Font.Bold
                color: gamingManager.waydroidStatus === "Running" ? Theme.accentGreen :
                       gamingManager.waydroidStatus === "Stopped" ? Theme.accentOrange :
                       Theme.accentRed
            }
        }

        PrimaryButton {
            text: qsTr("Refresh status")
            onClicked: gamingManager.refresh()
            Layout.alignment: Qt.AlignLeft
            Layout.topMargin: 8
        }

        Item { height: 24 }
    }
}
