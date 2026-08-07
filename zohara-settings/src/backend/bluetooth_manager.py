import subprocess
import json
import os
from datetime import datetime
from PySide6.QtCore import QObject, Slot, Signal, Property

HISTORY_FILE = os.path.expanduser("~/.local/share/zohara-settings/device-history.json")

class BluetoothManager(QObject):
    devicesChanged = Signal()
    historyChanged = Signal()
    adapterStateChanged = Signal()

    def __init__(self):
        super().__init__()
        self._devices = []
        self._history = []
        self._adapter_on = True
        os.makedirs(os.path.dirname(HISTORY_FILE), exist_ok=True)
        self._load_history()

    def _load_history(self):
        try:
            with open(HISTORY_FILE, "r") as f:
                self._history = json.load(f)
        except:
            self._history = []

    def _save_history(self):
        try:
            with open(HISTORY_FILE, "w") as f:
                json.dump(self._history, f, indent=2)
        except Exception as e:
            print(f"BluetoothManager: failed to save history: {e}")

    def _add_to_history(self, mac, name, event):
        entry = {"mac": mac, "name": name, "event": event,
                 "timestamp": datetime.now().isoformat()}
        # Remove old entries for this device, keep last 10 events
        self._history = [h for h in self._history if h["mac"] != mac][-9:]
        self._history.append(entry)
        self._save_history()
        self.historyChanged.emit()

    @Property(list, notify=devicesChanged)
    def devices(self):
        return self._devices

    @Property(list, notify=historyChanged)
    def history(self):
        return self._history

    @Slot()
    def refreshDevices(self):
        try:
            # Get all paired devices
            paired_out = subprocess.check_output(
                ["bluetoothctl", "devices", "Paired"],
                text=True, stderr=subprocess.DEVNULL
            ).strip()
            # Get connected devices
            connected_out = subprocess.check_output(
                ["bluetoothctl", "devices", "Connected"],
                text=True, stderr=subprocess.DEVNULL
            ).strip()
            connected_macs = set()
            for line in connected_out.splitlines():
                parts = line.split()
                if len(parts) >= 2:
                    connected_macs.add(parts[1])

            devices = []
            for line in paired_out.splitlines():
                parts = line.split(None, 2)
                if len(parts) >= 3:
                    mac = parts[1]
                    name = parts[2]
                    devices.append({
                        "mac": mac,
                        "name": name,
                        "connected": mac in connected_macs,
                        "icon": "audio-headphones" if "headphone" in name.lower() else "phone"
                    })
            self._devices = devices
        except:
            self._devices = []
        self.devicesChanged.emit()

    @Slot(str)
    def connectDevice(self, mac):
        try:
            subprocess.run(["bluetoothctl", "connect", mac], check=True, capture_output=True)
            self._add_to_history(mac, mac, "connect")
            self.refreshDevices()
        except Exception as e:
            print(f"BluetoothManager: connect failed: {e}")

    @Slot(str)
    def disconnectDevice(self, mac):
        try:
            subprocess.run(["bluetoothctl", "disconnect", mac], check=True, capture_output=True)
            self._add_to_history(mac, mac, "disconnect")
            self.refreshDevices()
        except Exception as e:
            print(f"BluetoothManager: disconnect failed: {e}")

    @Slot(str)
    def forgetDevice(self, mac):
        try:
            subprocess.run(["bluetoothctl", "remove", mac], check=True, capture_output=True)
            self._add_to_history(mac, mac, "forget")
            self.refreshDevices()
        except Exception as e:
            print(f"BluetoothManager: forget failed: {e}")

    @Slot(result=bool)
    def isAdapterOn(self):
        try:
            out = subprocess.check_output(
                ["bluetoothctl", "show"], text=True, stderr=subprocess.DEVNULL
            )
            return "Powered: yes" in out
        except:
            return False

    @Slot(bool)
    def setAdapterOn(self, enabled):
        try:
            subprocess.run(["bluetoothctl", "power", "on" if enabled else "off"])
            self.adapterStateChanged.emit()
        except:
            pass
