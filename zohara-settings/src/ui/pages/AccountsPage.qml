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
            text: qsTr("Accounts")
            font.pixelSize: 26
            font.weight: Font.Bold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.text
            Layout.bottomMargin: 8
        }

        // ── Zohara Account CTA ────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 100
            radius: Theme.radius
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: "#0067c0" }
                GradientStop { position: 1.0; color: "#0052a3" }
            }

            RowLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 16

                Rectangle {
                    width: 56
                    height: 56
                    radius: 28
                    color: Qt.rgba(1, 1, 1, 0.15)

                    Text {
                        anchors.centerIn: parent
                        text: "Z"
                        font.pixelSize: 30
                        font.weight: Font.Bold
                        color: "white"
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: qsTr("Sign in with a Zohara Account")
                        font.pixelSize: 16
                        font.weight: Font.DemiBold
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: "white"
                    }
                    Text {
                        text: qsTr("Sync settings and preferences across devices")
                        font.pixelSize: 12
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: Qt.rgba(1, 1, 1, 0.75)
                    }
                }

                Rectangle {
                    height: 34
                    width: 90
                    radius: Theme.radiusSmall
                    color: "white"

                    Text {
                        anchors.centerIn: parent
                        text: qsTr("Sign in")
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        font.family: "Inter, Segoe UI, sans-serif"
                        color: "#0067c0"
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: console.log("Zohara Account sign-in — coming soon")
                    }
                }
            }
        }

        // ── Local accounts ────────────────────────────────────────────────────
        Text {
            text: qsTr("Local accounts")
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Segoe UI, sans-serif"
            color: Theme.textSecondary
            Layout.topMargin: 12
        }

        Repeater {
            model: accountsManager.users
            SettingsCard {
                title: (modelData.fullname !== "" && modelData.fullname !== modelData.username
                        ? modelData.fullname + " (" + modelData.username + ")"
                        : modelData.username)
                subtitle: (modelData.admin ? qsTr("Administrator") : qsTr("Standard user")) +
                          " • uid " + modelData.uid + " • " + modelData.shell.split("/").pop()

                RowLayout {
                    spacing: 6

                    Text {
                        text: modelData.admin ? "A" : "U"
                        font.pixelSize: 20
                        font.weight: Font.Bold
                        color: modelData.admin ? Theme.accentOrange : Theme.textSecondary
                    }

                    PrimaryButton {
                        text: qsTr("Remove")
                        enabled: modelData.uid >= 1000 && modelData.username !== "root"
                        onClicked: accountsManager.removeUser(modelData.username)
                    }
                }
            }
        }

        PrimaryButton {
            text: qsTr("+ Add account")
            Layout.topMargin: 8
            Layout.alignment: Qt.AlignRight
            onClicked: addUserDialog.open()
        }

        // ── Add user popup ────────────────────────────────────────────────────
        Popup {
            id: addUserDialog
            anchors.centerIn: Overlay.overlay
            width: 340
            modal: true
            closePolicy: Popup.CloseOnEscape

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
                    text: qsTr("Add a local account")
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                }

                function fieldRect(hint) {
                    return null // placeholder; real fields below
                }

                Rectangle {
                    Layout.fillWidth: true; height: 36
                    radius: Theme.radiusSmall; color: Theme.surfaceHigh; border.color: Theme.border
                    TextInput {
                        id: newUsername
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                        verticalAlignment: TextInput.AlignVCenter
                        font.pixelSize: 13; font.family: "Inter, Segoe UI, sans-serif"
                        color: Theme.text; selectionColor: Theme.accent
                        Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter
                               text: qsTr("Username"); color: Theme.textSecondary
                               font: parent.font; visible: !parent.text }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true; height: 36
                    radius: Theme.radiusSmall; color: Theme.surfaceHigh; border.color: Theme.border
                    TextInput {
                        id: newPassword
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                        verticalAlignment: TextInput.AlignVCenter
                        font.pixelSize: 13; font.family: "Inter, Segoe UI, sans-serif"
                        color: Theme.text; selectionColor: Theme.accent
                        echoMode: TextInput.Password
                        Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter
                               text: qsTr("Password"); color: Theme.textSecondary
                               font: parent.font; visible: !parent.text }
                    }
                }

                RowLayout {
                    spacing: 6

                    CheckBox {
                        id: makeAdmin
                        text: qsTr("Administrator")
                        contentItem: Text {
                            leftPadding: parent.indicator ? parent.indicator.width + parent.spacing : 0
                            text: parent.text
                            font.pixelSize: 13; font.family: "Inter, Segoe UI, sans-serif"
                            color: Theme.text
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 12
                    PrimaryButton { text: qsTr("Cancel"); onClicked: addUserDialog.close() }
                    PrimaryButton {
                        text: qsTr("Create")
                        enabled: newUsername.text.length > 0
                        onClicked: {
                            accountsManager.addUser(newUsername.text, newPassword.text, makeAdmin.checked)
                            addUserDialog.close()
                        }
                    }
                }
            }
        }

        Item { height: 24 }
    }
}
