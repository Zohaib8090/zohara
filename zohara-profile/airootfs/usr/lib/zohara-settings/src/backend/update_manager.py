import subprocess
import json
import re
import threading
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer

class UpdateManager(QObject):
    statusChanged = Signal()
    updatesChanged = Signal()
    progressChanged = Signal()
    logChanged = Signal()

    def __init__(self):
        super().__init__()
        self._status = "Ready"
        self._updates = []          # [{name, current, new}]
        self._progress = 0          # 0-100
        self._log = []              # parsed /var/log/pacman.log
        self._restart_required = False

        try:
            import pydbus
            self.bus = pydbus.SystemBus()
            self.helper = self.bus.get("org.zohara.settings.Helper")
        except Exception as e:
            print(f"UpdateManager: DBus unavailable: {e}")
            self.helper = None

        # Detect localrepo
        self._has_localrepo = self._detect_localrepo()

    def _detect_localrepo(self):
        try:
            with open("/etc/pacman.conf") as f:
                return "[localrepo]" in f.read()
        except:
            return False

    @Property(str, notify=statusChanged)
    def status(self):
        return self._status

    @Property(list, notify=updatesChanged)
    def updates(self):
        return self._updates

    @Property(int, notify=progressChanged)
    def progress(self):
        return self._progress

    @Property(list, notify=logChanged)
    def pacmanLog(self):
        return self._log

    @Property(bool, notify=statusChanged)
    def restartRequired(self):
        return self._restart_required

    @Property(bool, notify=statusChanged)
    def hasLocalrepo(self):
        return self._has_localrepo

    @Slot()
    def checkForUpdates(self):
        def _check():
            self._status = "Syncing databases..."
            QTimer.singleShot(0, self.statusChanged.emit)
            try:
                if self.helper:
                    self.helper.UpdateSystem("check")
                # Parse available updates
                out = subprocess.check_output(
                    ["pacman", "-Qu"], text=True, stderr=subprocess.DEVNULL
                ).strip()
                updates = []
                for line in out.splitlines():
                    parts = line.split()
                    if len(parts) >= 4:
                        updates.append({
                            "name": parts[0],
                            "current": parts[1],
                            "new": parts[3]
                        })
                self._updates = updates
                count = len(updates)
                self._status = f"{count} update{'s' if count != 1 else ''} available" if count > 0 else "Your system is up to date"
            except subprocess.CalledProcessError:
                self._status = "Your system is up to date"
                self._updates = []
            except Exception as e:
                self._status = f"Error: {e}"
                self._updates = []
            QTimer.singleShot(0, self.updatesChanged.emit)
            QTimer.singleShot(0, self.statusChanged.emit)
        threading.Thread(target=_check, daemon=True).start()

    @Slot()
    def installUpdates(self):
        def _install():
            self._status = "Downloading packages..."
            self._progress = 5
            QTimer.singleShot(0, self.statusChanged.emit)
            QTimer.singleShot(0, self.progressChanged.emit)
            try:
                if self.helper:
                    self.helper.UpdateSystem("upgrade")
                # Check if kernel was updated (restart required)
                log_out = subprocess.check_output(
                    ["grep", "-i", "linux-zen", "/var/log/pacman.log"],
                    text=True, stderr=subprocess.DEVNULL
                )
                self._restart_required = len(log_out.strip()) > 0
                self._progress = 100
                self._status = "Update complete!" + (" Restart required." if self._restart_required else "")
                self._updates = []
            except Exception as e:
                self._status = f"Update failed: {e}"
                self._progress = 0
            QTimer.singleShot(0, self.progressChanged.emit)
            QTimer.singleShot(0, self.statusChanged.emit)
            QTimer.singleShot(0, self.updatesChanged.emit)
        threading.Thread(target=_install, daemon=True).start()

    @Slot()
    def loadHistory(self):
        try:
            out = subprocess.check_output(
                ["tail", "-n", "500", "/var/log/pacman.log"], text=True
            )
            entries = []
            for line in out.splitlines():
                # Format: [2026-07-15T10:00:00+0500] [ALPM] upgraded package (old -> new)
                m = re.match(r'\[(.+?)\] \[ALPM\] (upgraded|installed|removed) (.+)', line)
                if m:
                    entries.append({
                        "date": m.group(1),
                        "action": m.group(2),
                        "package": m.group(3)
                    })
            self._log = list(reversed(entries))  # Most recent first
        except Exception as e:
            self._log = []
        self.logChanged.emit()
