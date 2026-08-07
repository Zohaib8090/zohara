import subprocess
from PySide6.QtCore import QObject, Slot, Signal, Property

class GamingManager(QObject):
    statusChanged = Signal()

    def __init__(self):
        super().__init__()
        self._wine_version = ""
        self._waydroid_status = ""
        self._gamemode_available = False
        self._gamemode_active = False

    @Property(str, notify=statusChanged)
    def wineVersion(self):
        return self._wine_version

    @Property(str, notify=statusChanged)
    def waydroidStatus(self):
        return self._waydroid_status

    @Property(bool, notify=statusChanged)
    def gamemodeAvailable(self):
        return self._gamemode_available

    @Property(bool, notify=statusChanged)
    def gamemodeActive(self):
        return self._gamemode_active

    @Slot()
    def refresh(self):
        # Wine version
        try:
            self._wine_version = subprocess.check_output(
                ["wine", "--version"], text=True, stderr=subprocess.DEVNULL
            ).strip()
        except:
            self._wine_version = "Not installed"

        # Waydroid status
        try:
            out = subprocess.check_output(
                ["waydroid", "status"], text=True, stderr=subprocess.DEVNULL
            )
            if "RUNNING" in out.upper():
                self._waydroid_status = "Running"
            elif "STOPPED" in out.upper():
                self._waydroid_status = "Stopped"
            else:
                self._waydroid_status = out.strip().splitlines()[0] if out.strip() else "Unknown"
        except:
            self._waydroid_status = "Not available"

        # GameMode: check if gamemoded is installed
        try:
            subprocess.run(["gamemoded", "--status"], check=True,
                           capture_output=True, timeout=2)
            self._gamemode_available = True
            # Check if active: gamemoded returns 0 if active
            result = subprocess.run(
                ["gamemoded", "--status"], capture_output=True, text=True, timeout=2
            )
            self._gamemode_active = "is active" in result.stdout.lower()
        except:
            self._gamemode_available = False
            self._gamemode_active = False

        self.statusChanged.emit()

    @Slot(bool)
    def setGamemode(self, enabled):
        try:
            if enabled:
                subprocess.Popen(["gamemoded"])
            else:
                subprocess.run(["gamemoded", "-r"], capture_output=True)
            self._gamemode_active = enabled
            self.statusChanged.emit()
        except Exception as e:
            print(f"GamingManager: GameMode toggle failed: {e}")
