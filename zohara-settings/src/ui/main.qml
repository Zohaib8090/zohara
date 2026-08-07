import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "." // Imports Theme from qmldir

ApplicationWindow {
    id: window
    width: 1000
    height: 700
    visible: true
    title: qsTr("Settings")
    
    // Windows 11 style translucent background would go here
    color: Theme.background
    
    RowLayout {
        anchors.fill: parent
        spacing: 0
        
        // Navigation Rail
        NavRail {
            id: navRail
            Layout.preferredWidth: 260
            Layout.fillHeight: true
            
            onPageSelected: (pageUrl) => {
                pageLoader.source = pageUrl
            }
        }
        
        // Main Content Area
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.surface
            
            Loader {
                id: pageLoader
                anchors.fill: parent
                anchors.margins: 24
                source: "pages/SystemPage.qml" // Default page
            }
        }
    }
}

