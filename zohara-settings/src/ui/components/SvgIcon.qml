import QtQuick
import QtQuick.Controls

Item {
    id: root
    implicitWidth: 20
    implicitHeight: 20

    property string path: ""  // The SVG path data
    property color color: "#000000"

    Image {
        anchors.fill: parent
        // Use an inline SVG wrapper
        source: {
            if (root.path === "") return ""
            var svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
                          <path fill="${root.color.toString()}" d="${root.path}" />
                       </svg>`
            // In QML, encodeURIComponent is safe for data URIs
            return "data:image/svg+xml;utf8," + encodeURIComponent(svg)
        }
        fillMode: Image.PreserveAspectFit
        smooth: true
        antialiasing: true
    }
}
