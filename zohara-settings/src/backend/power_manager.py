import subprocess
import os
from PySide6.QtCore import QObject, Slot, Signal, Property

class PowerManager(QObject):
    profileChanged = Signal()
    batteryChanged = Signal()

    def __init__(self):
        super().__init__()
        self._profile = "Balanced"
        self._battery_percent = -1  # -1 = no battery / AC only
        self._battery_charging = False

    @Property(str, notify=profileChanged)
    def currentProfile(self):
        return self._profile

    @Property(int, notify=batteryChanged)
    def batteryPercent(self):
        return self._battery_percent

    @Property(bool, notify=batteryChanged)
    def batteryCharging(self):
        return self._battery_charging

    @Slot()
    def refreshBattery(self):
        # Read from UPower via /sys as a lightweight fallback
        bat_path = "/sys/class/power_supply/BAT0"
        if not os.path.exists(bat_path):
            bat_path = "/sys/class/power_supply/BAT1"
        if os.path.exists(bat_path):
            try:
                with open(f"{bat_path}/capacity") as f:
                    self._battery_percent = int(f.read().strip())
                with open(f"{bat_path}/status") as f:
                    status = f.read().strip()
                    self._battery_charging = status in ("Charging", "Full")
            except:
                pass
        else:
            self._battery_percent = -1
        self.batteryChanged.emit()

    @Slot(str)
    def setProfile(self, profile):
        """Set governor+EPP based power profile. Cross-vendor, kernel-native."""
        profiles = {
            "Performance": {
                "governor": "performance",
                "epp": "performance"
            },
            "Balanced": {
                "governor": "powersave",
                "epp": "balance_performance"
            },
            "Battery Saver": {
                "governor": "powersave",
                "epp": "power"
            }
        }
        if profile not in profiles:
            return

        cfg = profiles[profile]
        # Apply to all CPU cores
        cpu_count = os.cpu_count() or 1
        for i in range(cpu_count):
            # Governor
            gov_path = f"/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_governor"
            epp_path = f"/sys/devices/system/cpu/cpu{i}/cpufreq/energy_performance_preference"
            try:
                with open(gov_path, "w") as f:
                    f.write(cfg["governor"])
            except Exception as e:
                print(f"PowerManager: governor write failed for cpu{i}: {e}")
            try:
                with open(epp_path, "w") as f:
                    f.write(cfg["epp"])
            except Exception as e:
                print(f"PowerManager: EPP write failed for cpu{i}: {e}")

        self._profile = profile
        self.profileChanged.emit()

    @Slot(result=str)
    def getCurrentGovernor(self):
        try:
            with open("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") as f:
                gov = f.read().strip()
            with open("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference") as f:
                epp = f.read().strip()
            if gov == "performance":
                return "Performance"
            elif epp == "power":
                return "Battery Saver"
            else:
                return "Balanced"
        except:
            return "Balanced"
