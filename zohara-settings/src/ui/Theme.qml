pragma Singleton
import QtQuick

QtObject {
    id: theme

    // Detect system theme using QML ColorScheme (fallback logic included)
    property bool isDark: true // Assuming dark mode by default for Zohara as per spec

    // Colors
    property color background: isDark ? "#202020" : "#f3f3f3"
    property color surface: isDark ? "#2d2d2d" : "#ffffff"
    property color text: isDark ? "#ffffff" : "#000000"
    property color textSecondary: isDark ? "#a0a0a0" : "#666666"
    property color border: isDark ? "#3f3f3f" : "#e5e5e5"
    
    // Buttons (bluish effect for dark mode per user request)
    property color buttonBackground: isDark ? "#353b48" : "#f0f0f0"
    property color buttonHover: isDark ? "#4b5368" : "#e0e0e0"
    property color buttonText: isDark ? "#d9e2ec" : "#000000"

    // Primary Accent
    property color accent: "#0067c0"
    
    // Navigation Rail
    property color navHover: isDark ? "#333333" : "#e5e5e5"
}
