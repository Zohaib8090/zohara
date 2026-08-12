import subprocess
import json
import os
import threading
from datetime import datetime
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer

HISTORY_FILE = os.path.expanduser("~/.local/share/zohara-settings/device-history.json")

class BluetoothManager(QObject):
    devicesChanged = Signal()
    historyChanged = Signal()
    adapterStateChanged = Signal()

    def __init__(self):
        super().__init__()
        self._devices = []
        self._history = []
        self._adapter_on = False
        os.makedirs(os.path.dirname(HISTORY_FILE), exist_ok=True)
        self._load_history()
        # Async startup check — never block init
        self._refresh_adapter_state_async()

    # ── persistence ──────────────────────────────────────────────────────────

    def _load_history(self):
        try:
            with open(HISTORY_FILE, "r") as f:
                self._history = json.load(f)
        except Exception:
            self._history = []

    def _save_history(self):
        try:
            with open(HISTORY_FILE, "w") as f:
                json.dump(self._history, f, indent=2)
        except Exception as e:
            print(f"BluetoothManager._save_history: {e}")

    def _add_to_history(self, mac, name, event):
        # Keep last 20 entries across all devices
        entry = {"mac": mac, "name": name, "event": event,
                 "timestamp": datetime.now().isoformat()}
        self._history = [h for h in self._history if h["mac"] != mac]
        self._history.append(entry)
        self._history = self._history[-20:]
        self._save_history()
        QTimer.singleShot(0, self.historyChanged.emit)

    # ── async helpers ─────────────────────────────────────────────────────────

    def _refresh_adapter_state_async(self):
        def _go():
            try:
                out = subprocess.check_output(
                    ["bluetoothctl", "show"], text=True,
                    stderr=subprocess.DEVNULL, timeout=5
                )
                self._adapter_on = "Powered: yes" in out
            except Exception:
                self._adapter_on = False
            QTimer.singleShot(0, self.adapterStateChanged.emit)
            if self._adapter_on:
                self._do_refresh_devices()
        threading.Thread(target=_go, daemon=True).start()

    def _do_refresh_devices(self):
        def _go():
            try:
                paired_out = subprocess.check_output(
                    ["bluetoothctl", "devices", "Paired"],
                    text=True, stderr=subprocess.DEVNULL, timeout=5
                ).strip()
                connected_out = subprocess.check_output(
                    ["bluetoothctl", "devices", "Connected"],
                    text=True, stderr=subprocess.DEVNULL, timeout=5
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
                        })
                self._devices = devices
            except Exception:
                self._devices = []
            QTimer.singleShot(0, self.devicesChanged.emit)
        threading.Thread(target=_go, daemon=True).start()

    # ── properties ───────────────────────────────────────────────────────────

    @Property(list, notify=devicesChanged)
    def devices(self):
        return self._devices

    @Property(list, notify=historyChanged)
    def history(self):
        return self._history

    @Property(bool, notify=adapterStateChanged)
    def adapterOn(self):
        return self._adapter_on

    # ── slots ────────────────────────────────────────────────────────────────

    @Slot()
    def refreshDevices(self):
        self._do_refresh_devices()

    @Slot(str)
    def connectDevice(self, mac):
        def _go():
            try:
                subprocess.run(["bluetoothctl", "connect", mac],
                               check=True, capture_output=True, timeout=15)
                self._add_to_history(mac, mac, "connect")
            except Exception as e:
                print(f"BluetoothManager.connectDevice: {e}")
            self._do_refresh_devices()
        threading.Thread(target=_go, daemon=True).start()

    @Slot(str)
    def disconnectDevice(self, mac):
        def _go():
            try:
                subprocess.run(["bluetoothctl", "disconnect", mac],
                               check=True, capture_output=True, timeout=10)
                self._add_to_history(mac, mac, "disconnect")
            except Exception as e:
                print(f"BluetoothManager.disconnectDevice: {e}")
            self._do_refresh_devices()
        threading.Thread(target=_go, daemon=True).start()

    @Slot(str)
    def forgetDevice(self, mac):
        def _go():
            try:
                subprocess.run(["bluetoothctl", "remove", mac],
                               check=True, capture_output=True, timeout=10)
                self._add_to_history(mac, mac, "forget")
            except Exception as e:
                print(f"BluetoothManager.forgetDevice: {e}")
            self._do_refresh_devices()
        threading.Thread(target=_go, daemon=True).start()

    @Slot(bool)
    def setAdapterOn(self, enabled):
        def _go():
            try:
                subprocess.run(
                    ["bluetoothctl", "power", "on" if enabled else "off"],
                    check=True, capture_output=True, timeout=8
                )
                self._adapter_on = enabled
                if not enabled:
                    self._devices = []
                    QTimer.singleShot(0, self.devicesChanged.emit)
            except Exception as e:
                print(f"BluetoothManager.setAdapterOn: {e}")
            QTimer.singleShot(0, self.adapterStateChanged.emit)
            if enabled:
                self._do_refresh_devices()
        threading.Thread(target=_go, daemon=True).start()
