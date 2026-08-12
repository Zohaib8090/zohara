import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."  // Theme

ApplicationWindow {
    id: window
    width: 1060
    height: 740
    minimumWidth: 800
    minimumHeight: 560
    visible: true
    title: qsTr("Settings")
    color: Theme.background

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // ── Navigation Rail ───────────────────────────────────────────────────
        NavRail {
            id: navRail
            Layout.preferredWidth: 260
            Layout.fillHeight: true

            onPageSelected: function(url) {
                pageLoader.setSource(url)
            }
        }

        // ── Content Area ──────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.background
            
            // Add a subtle border rounding for the content area to mimic Win11 / modern macOS
            // (Only visible in light mode for depth)
            Rectangle {
                anchors.fill: parent
                anchors.margins: Theme.isDark ? 0 : 8
                color: Theme.background
                radius: Theme.isDark ? 0 : Theme.radius
                border.color: Theme.isDark ? "transparent" : Theme.border
                border.width: Theme.isDark ? 0 : 1

                Loader {
                    id: pageLoader
                    anchors.fill: parent
                    anchors.margins: 24  // Beautiful generous padding around the pages
                    source: "pages/SystemPage.qml"

                    onSourceChanged: {
                        fadeIn.restart()
                    }
                }
            }

            OpacityAnimator {
                id: fadeIn
                target: pageLoader
                from: 0; to: 1
                duration: 200
                easing.type: Easing.OutCubic
            }
        }
    }
}
