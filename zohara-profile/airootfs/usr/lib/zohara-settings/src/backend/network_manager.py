import subprocess
import json
import os
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer, QThread

class NetworkManager(QObject):
    networksChanged = Signal()
    statusChanged = Signal()

    def __init__(self):
        super().__init__()
        self._networks = []
        self._ethernet_status = "Checking..."
        self._wifi_enabled = True

    @Property(list, notify=networksChanged)
    def networks(self):
        return self._networks

    @Property(str, notify=statusChanged)
    def ethernetStatus(self):
        return self._ethernet_status

    @Slot()
    def refreshNetworks(self):
        try:
            out = subprocess.check_output(
                ["nmcli", "-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"],
                text=True, stderr=subprocess.DEVNULL
            ).strip()
            networks = []
            for line in out.splitlines():
                parts = line.split(":")
                if len(parts) >= 4:
                    in_use = parts[3].strip() == "*"
                    networks.append({
                        "ssid": parts[0].strip() or "(Hidden)",
                        "signal": int(parts[1]) if parts[1].isdigit() else 0,
                        "security": parts[2].strip() if parts[2].strip() else "Open",
                        "connected": in_use
                    })
            # Sort: connected first, then by signal
            self._networks = sorted(networks, key=lambda n: (not n["connected"], -n["signal"]))
        except Exception as e:
            self._networks = []
        self.networksChanged.emit()

    @Slot()
    def refreshEthernet(self):
        try:
            out = subprocess.check_output(
                ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"],
                text=True, stderr=subprocess.DEVNULL
            )
            for line in out.splitlines():
                parts = line.split(":")
                if len(parts) >= 3 and parts[0].lower() == "ethernet":
                    state = parts[1].strip()
                    conn = parts[2].strip() if parts[2].strip() else "Not connected"
                    self._ethernet_status = f"{state.capitalize()} — {conn}" if state == "connected" else "Disconnected"
                    self.statusChanged.emit()
                    return
            self._ethernet_status = "No ethernet adapter found"
        except:
            self._ethernet_status = "Unavailable"
        self.statusChanged.emit()

    @Slot(str, str)
    def connectToWifi(self, ssid, password):
        """Connect to a Wi-Fi network. Password can be empty for open networks."""
        try:
            if password:
                subprocess.run(["nmcli", "dev", "wifi", "connect", ssid, "password", password],
                               check=True, capture_output=True)
            else:
                subprocess.run(["nmcli", "dev", "wifi", "connect", ssid],
                               check=True, capture_output=True)
            self.refreshNetworks()
        except subprocess.CalledProcessError as e:
            print(f"NetworkManager: connect failed: {e.stderr}")

    @Slot(str)
    def disconnectNetwork(self, ssid):
        try:
            subprocess.run(["nmcli", "connection", "down", ssid],
                           check=True, capture_output=True)
            self.refreshNetworks()
        except Exception as e:
            print(f"NetworkManager: disconnect failed: {e}")

    @Slot(result=bool)
    def isWifiEnabled(self):
        try:
            out = subprocess.check_output(["nmcli", "radio", "wifi"], text=True).strip()
            return "enabled" in out
        except:
            return False

    @Slot(bool)
    def setWifiEnabled(self, enabled):
        try:
            subprocess.run(["nmcli", "radio", "wifi", "on" if enabled else "off"])
            self.statusChanged.emit()
        except:
            pass
