import QtQuick 2.0
import calamares.slideshow 1.0

Presentation {
    id: presentation

    Timer {
        interval: 10000
        running: true
        repeat: true
        onTriggered: presentation.goToNextSlide()
    }

    Slide {
        Rectangle {
            anchors.fill: parent
            color: "#1e1e2e"
            
            Image {
                anchors.centerIn: parent
                source: "logo.png"
                width: 256
                height: 256
                fillMode: Image.PreserveAspectFit
            }
            
            Text {
                anchors.top: parent.top
                anchors.topMargin: 20
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Welcome to Zohara OS"
                color: "#cdd6f4"
                font.pixelSize: 24
                font.bold: true
            }
        }
    }
}
