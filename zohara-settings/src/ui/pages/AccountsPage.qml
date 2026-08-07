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
            text: qsTr("Accounts")
            font.pixelSize: 28
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.bottomMargin: 12
        }
        
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 100
            radius: 8
            color: Theme.surface
            border.color: Theme.border
            border.width: 1
            
            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 16
                
                Rectangle {
                    width: 60
                    height: 60
                    radius: 30
                    color: Theme.buttonBackground
                    
                    Text {
                        anchors.centerIn: parent
                        text: "?"
                        font.pixelSize: 24
                        color: Theme.text
                    }
                }
                
                ColumnLayout {
                    Layout.fillWidth: true
                    
                    Text {
                        text: qsTr("Sign in with a Zohara Account")
                        font.pixelSize: 16
                        font.weight: Font.DemiBold
                        color: Theme.text
                    }
                    Text {
                        text: qsTr("Sync settings, access the Zohara Store, and integrate email seamlessly.")
                        font.pixelSize: 12
                        color: Theme.textSecondary
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
                
                PrimaryButton {
                    text: qsTr("Sign in")
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }
        
        Text {
            text: qsTr("Other users")
            font.pixelSize: 18
            font.weight: Font.DemiBold
            color: Theme.text
            Layout.topMargin: 12
        }
        
        Repeater {
            model: accountsManager.users
            SettingsCard {
                title: modelData.fullname + " (" + modelData.username + ")"
                subtitle: (modelData.admin ? qsTr("Administrator") : qsTr("Standard user")) + " • " + modelData.shell
                
                control: PrimaryButton {
                    text: qsTr("Remove")
                    onClicked: accountsManager.removeUser(modelData.username)
                    enabled: modelData.username !== "root" && modelData.uid >= 1000
                }
            }
        }
        
        PrimaryButton {
            text: qsTr("Add account")
            Layout.alignment: Qt.AlignRight
            onClicked: {
                // Stub for user creation modal
                console.log("Add user clicked")
            }
        }
    }
}
