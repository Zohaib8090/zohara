import subprocess
import threading
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer

class NetworkManager(QObject):
    networksChanged = Signal()
    statusChanged = Signal()
    wifiEnabledChanged = Signal()

    def __init__(self):
        super().__init__()
        self._networks = []
        self._ethernet_status = "Not checked"
        self._wifi_enabled = False
        self._scanning = False

        # Auto-refresh on startup (async, non-blocking)
        self._refresh_wifi_state_async()
        QTimer.singleShot(300, self.refreshEthernet)

    # ── internal helpers ─────────────────────────────────────────────────────

    def _run(self, cmd, timeout=6):
        """Run a subprocess synchronously with a timeout. Safe to call from background threads."""
        return subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL, timeout=timeout).strip()

    def _refresh_wifi_state_async(self):
        def _go():
            try:
                out = self._run(["nmcli", "radio", "wifi"])
                self._wifi_enabled = "enabled" in out
            except Exception:
                self._wifi_enabled = False
            QTimer.singleShot(0, self.wifiEnabledChanged.emit)
            QTimer.singleShot(0, self.statusChanged.emit)
            if self._wifi_enabled:
                self._scan_networks()
        threading.Thread(target=_go, daemon=True).start()

    def _scan_networks(self):
        """Background Wi-Fi scan — never blocks the UI thread."""
        def _go():
            try:
                out = self._run(
                    ["nmcli", "-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"],
                    timeout=8
                )
                networks = []
                for line in out.splitlines():
                    # nmcli -t uses colons; SSID can itself contain colons, so split from right
                    parts = line.rsplit(":", 3)
                    if len(parts) == 4:
                        ssid_raw, signal_raw, security, in_use = parts
                        try:
                            signal = int(signal_raw)
                        except ValueError:
                            signal = 0
                        networks.append({
                            "ssid": ssid_raw.strip() or "(Hidden)",
                            "signal": signal,
                            "security": security.strip() if security.strip() else "Open",
                            "connected": in_use.strip() == "*"
                        })
                # Sort: connected first, then by signal strength
                networks.sort(key=lambda n: (not n["connected"], -n["signal"]))
                self._networks = networks
            except Exception:
                self._networks = []
            QTimer.singleShot(0, self.networksChanged.emit)
        threading.Thread(target=_go, daemon=True).start()

    # ── properties ───────────────────────────────────────────────────────────

    @Property(list, notify=networksChanged)
    def networks(self):
        return self._networks

    @Property(str, notify=statusChanged)
    def ethernetStatus(self):
        return self._ethernet_status

    @Property(bool, notify=wifiEnabledChanged)
    def wifiEnabled(self):
        return self._wifi_enabled

    # ── slots (called from QML) ───────────────────────────────────────────────

    @Slot()
    def refreshNetworks(self):
        self._scan_networks()

    @Slot()
    def refreshEthernet(self):
        def _go():
            try:
                out = self._run(["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"])
                for line in out.splitlines():
                    parts = line.split(":", 2)
                    if len(parts) >= 3 and parts[0].lower() == "ethernet":
                        state = parts[1].strip()
                        conn = parts[2].strip() or "Not connected"
                        self._ethernet_status = (
                            f"Connected — {conn}" if state == "connected" else "Disconnected"
                        )
                        QTimer.singleShot(0, self.statusChanged.emit)
                        return
                self._ethernet_status = "No ethernet adapter"
            except Exception:
                self._ethernet_status = "Unavailable"
            QTimer.singleShot(0, self.statusChanged.emit)
        threading.Thread(target=_go, daemon=True).start()

    @Slot(str, str)
    def connectToWifi(self, ssid, password):
        def _go():
            try:
                cmd = ["nmcli", "dev", "wifi", "connect", ssid]
                if password:
                    cmd += ["password", password]
                subprocess.run(cmd, check=True, capture_output=True, timeout=20)
            except Exception as e:
                print(f"NetworkManager.connectToWifi: {e}")
            # Always refresh after attempt
            self._refresh_wifi_state_async()
        threading.Thread(target=_go, daemon=True).start()

    @Slot(str)
    def disconnectNetwork(self, connection_name):
        def _go():
            try:
                subprocess.run(
                    ["nmcli", "connection", "down", connection_name],
                    check=True, capture_output=True, timeout=10
                )
            except Exception as e:
                print(f"NetworkManager.disconnectNetwork: {e}")
            self._scan_networks()
        threading.Thread(target=_go, daemon=True).start()

    @Slot(bool)
    def setWifiEnabled(self, enabled):
        def _go():
            try:
                subprocess.run(
                    ["nmcli", "radio", "wifi", "on" if enabled else "off"],
                    check=True, capture_output=True, timeout=8
                )
                self._wifi_enabled = enabled
                if enabled:
                    self._scan_networks()
                else:
                    self._networks = []
                    QTimer.singleShot(0, self.networksChanged.emit)
            except Exception as e:
                print(f"NetworkManager.setWifiEnabled: {e}")
            QTimer.singleShot(0, self.wifiEnabledChanged.emit)
            QTimer.singleShot(0, self.statusChanged.emit)
        threading.Thread(target=_go, daemon=True).start()
