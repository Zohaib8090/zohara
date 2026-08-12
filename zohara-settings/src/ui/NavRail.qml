import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "." // Theme
import "components"

Rectangle {
    id: root
    color: Theme.navBackground
    width: 260

    signal pageSelected(string pageUrl)

    // Standardized SVG Paths
    property var icons: ({
        "System":              "M20 18c1.1 0 1.99-.9 1.99-2L22 6c0-1.1-.9-2-2-2H4c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2H0v2h24v-2h-4zM4 6h16v10H4V6z",
        "Network & internet":  "M1 9l2 2c5-5 13-5 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3a4.237 4.237 0 00-6 0zm-4-4l2 2a9.905 9.905 0 0110 0l2-2a12.733 12.733 0 00-14 0z",
        "Bluetooth & devices": "M17.71 7.71L12 2h-1v7.59L6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 11 14.41V22h1l5.71-5.71-4.3-4.29 4.3-4.29zM13 5.83l1.88 1.88L13 9.59V5.83zm1.88 10.46L13 18.17v-3.76l1.88 1.88z",
        "Personalization":     "M12 3a9 9 0 0 0 0 18c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-.99 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 9 6.5 9 8 9.67 8 10.5 7.33 12 6.5 12zm3-4C8.67 8 8 7.33 8 6.5S8.67 5 9.5 5s1.5.67 1.5 1.5S10.33 8 9.5 8zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 5 14.5 5s1.5.67 1.5 1.5S15.33 8 14.5 8zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 9 17.5 9s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z",
        "Apps":                "M4 8h4V4H4v4zm6 12h4v-4h-4v4zm-6 0h4v-4H4v4zm0-6h4v-4H4v4zm6 0h4v-4h-4v4zm6-10v4h4V4h-4zm-6 4h4V4h-4v4zm6 6h4v-4h-4v4zm0 6h4v-4h-4v4z",
        "Accounts":            "M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z",
        "Gaming":              "M21.58 16.09l-1.09-7.66C20.18 6.53 18.6 5 16.65 5H7.35C5.4 5 3.82 6.53 3.51 8.43l-1.09 7.66C2.2 17.63 3.39 19 4.94 19h1.58c.71 0 1.36-.37 1.72-.97l2.15-3.53h3.22l2.15 3.53c.36.6 1.01.97 1.72.97h1.58c1.55 0 2.74-1.37 2.52-2.91zM9 11H7v2H5v-2H3V9h2V7h2v2h2v2zm9.5 0c-.83 0-1.5-.67-1.5-1.5S17.67 8 18.5 8s1.5.67 1.5 1.5-.67 1.5-1.5 1.5zm-3-3c-.83 0-1.5-.67-1.5-1.5S14.67 5 15.5 5s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z",
        "Time & language":     "M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z M12.5 7H11v6l5.25 3.15.75-1.23-4.5-2.67z",
        "Accessibility":       "M12 2c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2zm9 7h-6v13h-2v-6h-2v6H9V9H3V7h18v2z",
        "Privacy & security":  "M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm0 10.99h7c-.53 4.12-3.28 7.79-7 8.94V12H5V6.3l7-3.11v8.8z",
        "Zohara Update":       "M17 1.01L7 1c-1.1 0-2 .9-2 2v18c0 1.1.9 2 2 2h10c1.1 0 2-.9 2-2V3c0-1.1-.9-1.99-2-1.99zM17 19H7V5h10v14zm-1-6h-3V8h-2v5H8l4 4 4-4z",
        "Advanced (KDE)":      "M19.43 12.98c.04-.32.07-.64.07-.98 0-.34-.03-.66-.07-.98l2.11-1.65c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.3-.61-.22l-2.49 1c-.52-.4-1.08-.73-1.69-.98l-.38-2.65C14.46 2.18 14.25 2 14 2h-4c-.25 0-.46.18-.49.42l-.38 2.65c-.61.25-1.17.59-1.69.98l-2.49-1c-.23-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64l2.11 1.65c-.04.32-.07.65-.07.98 0 .33.03.66.07.98l-2.11 1.65c-.19.15-.24.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1c.52.4 1.08.73 1.69.98l.38 2.65c.03.24.24.42.49.42h4c.25 0 .46-.18.49-.42l.38-2.65c.61-.25 1.17-.59 1.69-.98l2.49 1c.23.09.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.65zM12 15.5c-1.93 0-3.5-1.57-3.5-3.5s1.57-3.5 3.5-3.5 3.5 1.57 3.5 3.5-1.57 3.5-3.5 3.5z"
    })

    ListModel {
        id: navModel
        ListElement { label: "System";              url: "pages/SystemPage.qml"           }
        ListElement { label: "Network & internet";  url: "pages/NetworkPage.qml"          }
        ListElement { label: "Bluetooth & devices"; url: "pages/BluetoothPage.qml"        }
        ListElement { label: "Personalization";     url: "pages/PersonalizationPage.qml"  }
        ListElement { label: "Apps";                url: "pages/AppsPage.qml"             }
        ListElement { label: "Accounts";            url: "pages/AccountsPage.qml"         }
        ListElement { label: "Gaming";              url: "pages/GamingPage.qml"           }
        ListElement { label: "Time & language";     url: "pages/TimeLanguagePage.qml"     }
        ListElement { label: "Accessibility";       url: "pages/AccessibilityPage.qml"    }
        ListElement { label: "Privacy & security";  url: "pages/PrivacySecurityPage.qml"  }
        ListElement { label: "Zohara Update";       url: "pages/UpdatePage.qml"           }
        ListElement { label: "Advanced (KDE)";      url: "pages/AdvancedPage.qml"         }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 0

        // User Profile Area (Top Left corner)
        RowLayout {
            Layout.fillWidth: true
            Layout.bottomMargin: 16
            spacing: 12

            Rectangle {
                width: 36
                height: 36
                radius: 18
                color: Theme.accent
                Text {
                    anchors.centerIn: parent
                    text: "Z"
                    color: "white"
                    font.pixelSize: 18
                    font.weight: Font.Bold
                    font.family: "Inter, Segoe UI, sans-serif"
                }
            }

            ColumnLayout {
                spacing: 2
                Text {
                    text: qsTr("Zohara OS")
                    font.pixelSize: 14
                    font.weight: Font.Bold
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                }
                Text {
                    text: qsTr("Local Account")
                    font.pixelSize: 12
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.textSecondary
                }
            }
        }

        // ── Search ───────────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 36
            radius: Theme.radiusSmall
            color: Theme.surfaceHigh
            border.color: Theme.border
            Layout.bottomMargin: 16

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: 8

                SvgIcon {
                    width: 16; height: 16
                    path: "M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"
                    color: Theme.textSecondary
                }

                TextInput {
                    id: searchInput
                    Layout.fillWidth: true
                    font.pixelSize: 13
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.text
                    selectionColor: Theme.accent
                    clip: true
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        anchors.fill: parent
                        text: qsTr("Find a setting")
                        color: Theme.textSecondary
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        // ── Nav Items ─────────────────────────────────────────────────────────
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: navModel
            clip: true
            currentIndex: 0
            spacing: 2
            boundsBehavior: Flickable.StopAtBounds

            // filter logic
            property string filterText: searchInput.text.toLowerCase()

            delegate: Item {
                width: ListView.view.width
                height: visible ? 40 : 0
                visible: searchInput.text === "" ||
                         model.label.toLowerCase().includes(listView.filterText)

                // Active indicator pill
                Rectangle {
                    x: 0
                    y: (parent.height - height) / 2
                    width: 4
                    height: 18
                    radius: 2
                    color: Theme.navItemPill
                    visible: listView.currentIndex === index
                }

                // Row item
                Rectangle {
                    id: itemBg
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 0
                    radius: Theme.radiusSmall
                    color: listView.currentIndex === index
                           ? Theme.navItemActive
                           : (hoverArea.containsMouse ? Theme.navItemHover : "transparent")

                    Behavior on color { ColorAnimation { duration: 150 } }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 12

                        SvgIcon {
                            width: 18; height: 18
                            path: root.icons[model.label] || ""
                            color: listView.currentIndex === index ? Theme.accent : Theme.textSecondary
                        }

                        Text {
                            Layout.fillWidth: true
                            text: model.label
                            font.pixelSize: 13
                            font.family: "Inter, Segoe UI, sans-serif"
                            font.weight: listView.currentIndex === index ? Font.DemiBold : Font.Normal
                            color: listView.currentIndex === index ? Theme.text : Theme.textSecondary
                            elide: Text.ElideRight
                        }
                    }
                }

                MouseArea {
                    id: hoverArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        listView.currentIndex = index
                        root.pageSelected(model.url)
                    }
                }
            }
        }

        // ── Footer: theme toggle ──────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 44
            color: "transparent"
            Layout.topMargin: 8

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 8
                spacing: 12

                SvgIcon {
                    width: 18; height: 18
                    path: "M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36a5.389 5.389 0 0 1-4.4 2.26 5.403 5.403 0 0 1-3.14-9.8c-.44-.06-.9-.1-1.36-.1z"
                    color: Theme.textSecondary
                }

                Text {
                    text: Theme.isDark ? qsTr("Dark mode") : qsTr("Light mode")
                    font.pixelSize: 13
                    font.family: "Inter, Segoe UI, sans-serif"
                    color: Theme.textSecondary
                    Layout.fillWidth: true
                }

                Switch {
                    checked: Theme.isDark
                    onCheckedChanged: Theme.isDark = checked
                }
            }
        }
    }
}
