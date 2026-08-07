import subprocess
import os
import sqlite3
import threading
from datetime import datetime
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer

CACHE_DB = os.path.expanduser("~/.cache/zohara-settings/apps.db")

class AppsManager(QObject):
    appsChanged = Signal()
    scanningChanged = Signal()

    def __init__(self):
        super().__init__()
        self._apps = []
        self._scanning = False
        os.makedirs(os.path.dirname(CACHE_DB), exist_ok=True)
        self._init_db()
        # Load from cache immediately
        self._apps = self._read_cache()
        # Trigger async scan in background
        self._run_async_scan()

    def _init_db(self):
        conn = sqlite3.connect(CACHE_DB)
        conn.execute("""CREATE TABLE IF NOT EXISTS apps (
            name TEXT PRIMARY KEY,
            source TEXT,
            size TEXT,
            install_date TEXT
        )""")
        conn.commit()
        conn.close()

    def _read_cache(self):
        try:
            conn = sqlite3.connect(CACHE_DB)
            rows = conn.execute("SELECT name, source, size, install_date FROM apps ORDER BY name").fetchall()
            conn.close()
            return [{"name": r[0], "source": r[1], "size": r[2], "installDate": r[3]} for r in rows]
        except:
            return []

    def _write_cache(self, apps):
        try:
            conn = sqlite3.connect(CACHE_DB)
            conn.execute("DELETE FROM apps")
            conn.executemany(
                "INSERT INTO apps(name, source, size, install_date) VALUES(?,?,?,?)",
                [(a["name"], a["source"], a["size"], a["installDate"]) for a in apps]
            )
            conn.commit()
            conn.close()
        except Exception as e:
            print(f"AppsManager: cache write failed: {e}")

    def _scan_native(self):
        apps = []
        try:
            out = subprocess.check_output(
                ["pacman", "-Qi"], text=True, stderr=subprocess.DEVNULL
            )
            current = {}
            for line in out.splitlines():
                if line.startswith("Name"):
                    current["name"] = line.split(":", 1)[1].strip()
                elif line.startswith("Installed Size"):
                    current["size"] = line.split(":", 1)[1].strip()
                elif line.startswith("Install Date"):
                    current["installDate"] = line.split(":", 1)[1].strip()
                    apps.append({"name": current.get("name", "?"),
                                 "source": "Native",
                                 "size": current.get("size", "?"),
                                 "installDate": current.get("installDate", "?")})
                    current = {}
        except Exception as e:
            print(f"AppsManager: native scan error: {e}")
        return apps

    def _scan_flatpak(self):
        apps = []
        try:
            out = subprocess.check_output(
                ["flatpak", "list", "--columns=application,name,size,installation"],
                text=True, stderr=subprocess.DEVNULL
            )
            for line in out.strip().splitlines():
                parts = line.split("\t")
                if len(parts) >= 3:
                    apps.append({"name": parts[1] if len(parts) > 1 else parts[0],
                                 "source": "Flatpak",
                                 "size": parts[2] if len(parts) > 2 else "?",
                                 "installDate": "?"})
        except Exception as e:
            print(f"AppsManager: flatpak scan skipped: {e}")
        return apps

    def _scan_waydroid(self):
        apps = []
        try:
            out = subprocess.check_output(
                ["waydroid", "shell", "pm", "list", "packages", "-3"],
                text=True, stderr=subprocess.DEVNULL, timeout=10
            )
            for line in out.strip().splitlines():
                if line.startswith("package:"):
                    pkg = line.replace("package:", "").strip()
                    apps.append({"name": pkg, "source": "Android",
                                 "size": "?", "installDate": "?"})
        except Exception as e:
            print(f"AppsManager: waydroid scan skipped: {e}")
        return apps

    def _do_scan(self):
        self._scanning = True
        QTimer.singleShot(0, self.scanningChanged.emit)
        all_apps = []
        all_apps.extend(self._scan_native())
        all_apps.extend(self._scan_flatpak())
        all_apps.extend(self._scan_waydroid())
        # Deduplicate by name
        seen = set()
        deduped = []
        for a in all_apps:
            if a["name"] not in seen:
                seen.add(a["name"])
                deduped.append(a)
        self._write_cache(deduped)
        self._apps = deduped
        self._scanning = False
        QTimer.singleShot(0, self.scanningChanged.emit)
        QTimer.singleShot(0, self.appsChanged.emit)

    def _run_async_scan(self):
        t = threading.Thread(target=self._do_scan, daemon=True)
        t.start()

    @Property(list, notify=appsChanged)
    def apps(self):
        return self._apps

    @Property(bool, notify=scanningChanged)
    def scanning(self):
        return self._scanning

    @Slot()
    def rescan(self):
        self._run_async_scan()
