import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "." // Theme

Rectangle {
    id: root
    color: "transparent"
    
    signal pageSelected(string pageUrl)
    
    ListModel {
        id: navModel
        ListElement { name: "Network & internet"; url: "pages/NetworkPage.qml" }
        ListElement { name: "Bluetooth & devices"; url: "pages/BluetoothPage.qml" }
        ListElement { name: "Personalization"; url: "pages/PersonalizationPage.qml" }
        ListElement { name: "Apps"; url: "pages/AppsPage.qml" }
        ListElement { name: "Accounts"; url: "pages/AccountsPage.qml" }
        ListElement { name: "Gaming"; url: "pages/GamingPage.qml" }
        ListElement { name: "Time & language"; url: "pages/TimeLanguagePage.qml" }
        ListElement { name: "Accessibility"; url: "pages/AccessibilityPage.qml" }
        ListElement { name: "Privacy & security"; url: "pages/PrivacySecurityPage.qml" }
        ListElement { name: "System"; url: "pages/SystemPage.qml" }
        ListElement { name: "Zohara Update"; url: "pages/UpdatePage.qml" }
        ListElement { name: "Advanced (KDE)"; url: "pages/AdvancedPage.qml" }
    }
    
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 4
        
        // Search Box stub
        TextField {
            Layout.fillWidth: true
            placeholderText: qsTr("Find a setting")
            Layout.bottomMargin: 12
            color: Theme.text
            background: Rectangle {
                color: Theme.background
                border.color: Theme.border
                radius: 4
            }
        }
        
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: navModel
            clip: true
            
            delegate: ItemDelegate {
                width: ListView.view.width
                height: 40
                
                contentItem: RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 4
                        height: 24
                        radius: 2
                        color: listView.currentIndex === index ? Theme.accent : "transparent"
                    }
                    Text {
                        text: model.name
                        font.family: "Inter"
                        font.pixelSize: 14
                        color: Theme.text
                        font.weight: listView.currentIndex === index ? Font.DemiBold : Font.Normal
                    }
                }
                
                background: Rectangle {
                    radius: 4
                    color: parent.hovered ? Theme.navHover : "transparent"
                }
                
                onClicked: {
                    listView.currentIndex = index
                    root.pageSelected(model.url)
                }
            }
        }
    }
}

