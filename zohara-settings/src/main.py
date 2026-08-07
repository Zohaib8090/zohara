import sys
import os
from PySide6.QtGui import QGuiApplication, QIcon
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtCore import QUrl

# Import managers
sys.path.append(os.path.join(os.path.dirname(__file__), 'backend'))
from system_manager import SystemManager
from update_manager import UpdateManager
from apps_manager import AppsManager
from network_manager import NetworkManager
from bluetooth_manager import BluetoothManager
from accounts_manager import AccountsManager
from gaming_manager import GamingManager
from power_manager import PowerManager
from time_language_manager import TimeLanguageManager

def main():
    app = QGuiApplication(sys.argv)
    app.setOrganizationName("Zohara")
    app.setOrganizationDomain("zohara.org")
    app.setApplicationName("Zohara Settings")
    app.setWindowIcon(QIcon.fromTheme("preferences-system"))
    
    engine = QQmlApplicationEngine()
    
    # Register backend managers
    sys_mgr = SystemManager()
    upd_mgr = UpdateManager()
    apps_mgr = AppsManager()
    net_mgr = NetworkManager()
    bt_mgr = BluetoothManager()
    acc_mgr = AccountsManager()
    game_mgr = GamingManager()
    pwr_mgr = PowerManager()
    time_mgr = TimeLanguageManager()
    
    engine.rootContext().setContextProperty("systemManager", sys_mgr)
    engine.rootContext().setContextProperty("updateManager", upd_mgr)
    engine.rootContext().setContextProperty("appsManager", apps_mgr)
    engine.rootContext().setContextProperty("networkManager", net_mgr)
    engine.rootContext().setContextProperty("bluetoothManager", bt_mgr)
    engine.rootContext().setContextProperty("accountsManager", acc_mgr)
    engine.rootContext().setContextProperty("gamingManager", game_mgr)
    engine.rootContext().setContextProperty("powerManager", pwr_mgr)
    engine.rootContext().setContextProperty("timeManager", time_mgr)

    # Get the directory of the current script
    current_dir = os.path.dirname(os.path.abspath(__file__))
    main_qml_path = os.path.join(current_dir, "ui", "main.qml")
    
    engine.load(QUrl.fromLocalFile(main_qml_path))

    if not engine.rootObjects():
        sys.exit(-1)

    sys.exit(app.exec())

if __name__ == "__main__":
    main()

