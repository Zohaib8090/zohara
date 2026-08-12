pragma Singleton
import QtQuick

QtObject {
    id: theme

    property bool isDark: true

    // ── Foundations ─────────────────────────────────────────────────────────
    // Background of the main window. Soft off-white or deep charcoal.
    property color background:      isDark ? "#202020" : "#f3f3f3"
    
    // Background for SettingsCards. Creates depth.
    property color surface:         isDark ? "#2d2d2d" : "#ffffff"
    property color surfaceHigh:     isDark ? "#383838" : "#f9f9f9"
    property color navBackground:   "transparent" // Let window background show through

    // ── Borders ─────────────────────────────────────────────────────────────
    // Very subtle borders to define edges without heavy lines.
    property color border:          isDark ? Qt.rgba(1,1,1, 0.08) : Qt.rgba(0,0,0, 0.06)
    property color separator:       isDark ? Qt.rgba(1,1,1, 0.05) : Qt.rgba(0,0,0, 0.05)

    // ── Typography ──────────────────────────────────────────────────────────
    property color text:            isDark ? "#ffffff" : "#1a1a1a"
    property color textSecondary:   isDark ? "#a0a0a0" : "#5e5e5e"
    property color textDisabled:    isDark ? "#666666" : "#a0a0a0"

    // ── Controls ────────────────────────────────────────────────────────────
    property color buttonBg:        isDark ? Qt.rgba(1,1,1, 0.06) : Qt.rgba(0,0,0, 0.03)
    property color buttonBgHover:   isDark ? Qt.rgba(1,1,1, 0.1)  : Qt.rgba(0,0,0, 0.06)
    property color buttonBgPress:   isDark ? Qt.rgba(1,1,1, 0.03) : Qt.rgba(0,0,0, 0.08)
    property color buttonText:      Theme.text

    // ── Accent ──────────────────────────────────────────────────────────────
    // A beautiful vibrant blue, similar to Windows 11 / iOS
    property color accent:         "#0067c0"
    property color accentHover:    "#005ba3"
    property color accentText:     "#ffffff"
    
    // Semantic
    property color accentGreen:    isDark ? "#34c759" : "#28cd41"
    property color accentRed:      isDark ? "#ff3b30" : "#ff3b30"
    property color accentOrange:   isDark ? "#ff9f0a" : "#ff9500"

    // ── Nav Rail ────────────────────────────────────────────────────────────
    property color navItemHover:   isDark ? Qt.rgba(1,1,1, 0.05) : Qt.rgba(0,0,0, 0.04)
    property color navItemActive:  isDark ? Qt.rgba(1,1,1, 0.08) : Qt.rgba(0,0,0, 0.06)
    property color navItemPill:    Theme.accent

    // ── Geometry ────────────────────────────────────────────────────────────
    property int   radius:         10  // Smooth, continuous-like curve
    property int   radiusSmall:    6
    property int   cardPadding:    20
}
