import subprocess
import os
from PySide6.QtCore import QObject, Slot, Signal, Property

class AccountsManager(QObject):
    usersChanged = Signal()

    def __init__(self):
        super().__init__()
        self._users = []
        try:
            import pydbus
            self.bus = pydbus.SystemBus()
            self.helper = self.bus.get("org.zohara.settings.Helper")
        except Exception as e:
            print(f"AccountsManager: DBus unavailable: {e}")
            self.helper = None
        self.refreshUsers()

    @Property(list, notify=usersChanged)
    def users(self):
        return self._users

    @Slot()
    def refreshUsers(self):
        try:
            out = subprocess.check_output(
                ["getent", "passwd"], text=True
            )
            users = []
            for line in out.strip().splitlines():
                parts = line.split(":")
                if len(parts) < 7:
                    continue
                uid = int(parts[2])
                # Only show real human users (UID 1000+) and root
                if uid >= 1000 or uid == 0:
                    groups_out = subprocess.check_output(
                        ["groups", parts[0]], text=True
                    ).strip()
                    is_admin = "wheel" in groups_out or "sudo" in groups_out
                    users.append({
                        "username": parts[0],
                        "fullname": parts[4].split(",")[0] if parts[4] else parts[0],
                        "uid": uid,
                        "shell": parts[6],
                        "admin": is_admin
                    })
            self._users = users
        except Exception as e:
            print(f"AccountsManager: refreshUsers error: {e}")
            self._users = []
        self.usersChanged.emit()

    @Slot(str, str, bool)
    def addUser(self, username, password, is_admin):
        if not self.helper:
            return
        try:
            self.helper.ManageUser("add", username, password)
            if is_admin:
                subprocess.run(["usermod", "-aG", "wheel", username])
            self.refreshUsers()
        except Exception as e:
            print(f"AccountsManager: addUser error: {e}")

    @Slot(str)
    def removeUser(self, username):
        if not self.helper:
            return
        try:
            self.helper.ManageUser("remove", username, "")
            self.refreshUsers()
        except Exception as e:
            print(f"AccountsManager: removeUser error: {e}")
