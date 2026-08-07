import subprocess
from PySide6.QtCore import QObject, Slot, Signal, Property

class TimeLanguageManager(QObject):
    timezoneChanged = Signal()
    localeChanged = Signal()

    def __init__(self):
        super().__init__()
        self._timezone = self._get_timezone()
        self._is_24h = True
        self._locale = self._get_locale()
        self._keyboard_layout = self._get_keyboard()

    def _get_timezone(self):
        try:
            out = subprocess.check_output(
                ["timedatectl", "show", "--property=Timezone", "--value"],
                text=True, stderr=subprocess.DEVNULL
            ).strip()
            return out
        except:
            return "UTC"

    def _get_locale(self):
        try:
            out = subprocess.check_output(
                ["localectl", "status"], text=True, stderr=subprocess.DEVNULL
            )
            for line in out.splitlines():
                if "System Locale" in line:
                    return line.split("=", 1)[-1].strip()
        except:
            pass
        return "en_US.UTF-8"

    def _get_keyboard(self):
        try:
            out = subprocess.check_output(
                ["localectl", "status"], text=True, stderr=subprocess.DEVNULL
            )
            for line in out.splitlines():
                if "VC Keymap" in line:
                    return line.split(":", 1)[-1].strip()
        except:
            pass
        return "us"

    @Property(str, notify=timezoneChanged)
    def timezone(self):
        return self._timezone

    @Property(str, notify=localeChanged)
    def locale(self):
        return self._locale

    @Property(str, notify=localeChanged)
    def keyboardLayout(self):
        return self._keyboard_layout

    @Slot(result=list)
    def listTimezones(self):
        try:
            out = subprocess.check_output(
                ["timedatectl", "list-timezones"], text=True, stderr=subprocess.DEVNULL
            )
            return out.strip().splitlines()
        except:
            return ["UTC"]

    @Slot(str)
    def setTimezone(self, tz):
        try:
            subprocess.run(["timedatectl", "set-timezone", tz], check=True)
            self._timezone = tz
            self.timezoneChanged.emit()
        except Exception as e:
            print(f"TimeLanguageManager: setTimezone failed: {e}")

    @Slot(str)
    def setKeyboardLayout(self, layout):
        try:
            subprocess.run(["localectl", "set-keymap", layout], check=True)
            self._keyboard_layout = layout
            self.localeChanged.emit()
        except Exception as e:
            print(f"TimeLanguageManager: setKeyboard failed: {e}")

    @Slot(bool)
    def set24Hour(self, enabled):
        self._is_24h = enabled
        # This is stored via KDE locale settings
        try:
            if enabled:
                subprocess.run(["kwriteconfig6", "--file", "kdeglobals",
                                "--group", "Locale", "--key", "TimeFormat", "%H:%M:%S"])
            else:
                subprocess.run(["kwriteconfig6", "--file", "kdeglobals",
                                "--group", "Locale", "--key", "TimeFormat", "%I:%M:%S %p"])
        except:
            pass
