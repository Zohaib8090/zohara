import sys
import subprocess
from gi.repository import GLib, Gio
import pydbus

class ZoharaSettingsHelper:
    """
    <node>
        <interface name='org.zohara.settings.Helper'>
            <method name='UpdateSystem'>
                <arg type='s' name='action' direction='in'/>
                <arg type='s' name='result' direction='out'/>
            </method>
            <method name='ManageUser'>
                <arg type='s' name='action' direction='in'/>
                <arg type='s' name='username' direction='in'/>
                <arg type='s' name='password' direction='in'/>
                <arg type='s' name='result' direction='out'/>
            </method>
        </interface>
    </node>
    """
    def __init__(self):
        pass

    def check_polkit(self, action_id):
        # Stub for polkit check using PolicyKit1 D-Bus API
        # Normally you would extract the caller's bus name, get their UID/PID, 
        # and ask polkit if they are authorized for `action_id`.
        # For the sake of this scaffolding, we assume authorized if polkit passes.
        return True

    def UpdateSystem(self, action):
        if not self.check_polkit("org.zohara.settings.update-system"):
            return "Polkit authorization failed"
        
        if action == "check":
            res = subprocess.run(["pacman", "-Sy"], capture_output=True, text=True)
            return res.stdout
        elif action == "upgrade":
            # For a real implementation, you'd want to stream this or use a non-blocking approach
            res = subprocess.run(["pacman", "-Syu", "--noconfirm"], capture_output=True, text=True)
            return res.stdout
        return "Unknown action"

    def ManageUser(self, action, username, password):
        if not self.check_polkit("org.zohara.settings.manage-users"):
            return "Polkit authorization failed"
        
        if action == "add":
            res = subprocess.run(["useradd", "-m", username], capture_output=True, text=True)
            if password and res.returncode == 0:
                # Set password (stub, needs chpasswd)
                pass
            return "Success" if res.returncode == 0 else "Failed"
        elif action == "remove":
            res = subprocess.run(["userdel", "-r", username], capture_output=True, text=True)
            return "Success" if res.returncode == 0 else "Failed"
        return "Unknown action"

def main():
    loop = GLib.MainLoop()
    bus = pydbus.SystemBus()
    
    try:
        bus.publish("org.zohara.settings.Helper", ZoharaSettingsHelper())
        print("Zohara Settings Helper running on system bus...")
        loop.run()
    except Exception as e:
        print(f"Failed to start helper: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
