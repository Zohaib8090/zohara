import sys
import subprocess
from gi.repository import GLib, Gio

INTROSPECTION_XML = """
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

POLKIT_BUS_NAME = 'org.freedesktop.PolicyKit1'
POLKIT_OBJECT_PATH = '/org/freedesktop/PolicyKit1/Authority'
POLKIT_IFACE = 'org.freedesktop.PolicyKit1.Authority'
POLKIT_ALLOW_USER_INTERACTION = 1


class ZoharaSettingsHelper:
    def __init__(self, connection):
        self._polkit = Gio.DBusProxy.new_sync(
            connection, Gio.DBusProxyFlags.NONE, None,
            POLKIT_BUS_NAME, POLKIT_OBJECT_PATH, POLKIT_IFACE, None)

    def check_polkit(self, sender, action_id):
        # `sender` is the caller's unique D-Bus name, provided by the bus itself
        # (GDBusMethodInvocation.get_sender()) -- it cannot be spoofed by the
        # caller, unlike a PID/UID the caller might claim in the method args.
        # We hand that name to polkitd as a 'system-bus-name' subject and let
        # it resolve the real UID/PID and evaluate the action's .policy rules.
        subject = ('system-bus-name', {'name': GLib.Variant('s', sender)})
        try:
            result = self._polkit.call_sync(
                'CheckAuthorization',
                GLib.Variant('((sa{sv})sa{ss}us)', (
                    subject, action_id, {}, POLKIT_ALLOW_USER_INTERACTION, '',
                )),
                Gio.DBusCallFlags.NONE, -1, None)
        except GLib.Error as e:
            print(f"polkit CheckAuthorization failed: {e}", file=sys.stderr)
            return False
        is_authorized, _is_challenge, _details = result.unpack()
        return is_authorized

    def handle_method_call(self, connection, sender, object_path, interface_name,
                            method_name, parameters, invocation):
        args = parameters.unpack()

        if method_name == 'UpdateSystem':
            (action,) = args
            invocation.return_value(GLib.Variant('(s)', (self.update_system(sender, action),)))
        elif method_name == 'ManageUser':
            action, username, password = args
            invocation.return_value(
                GLib.Variant('(s)', (self.manage_user(sender, action, username, password),)))
        else:
            invocation.return_error_literal(
                Gio.dbus_error_quark(), Gio.DBusError.UNKNOWN_METHOD, 'No such method')

    def update_system(self, sender, action):
        if not self.check_polkit(sender, "org.zohara.settings.update-system"):
            return "Polkit authorization failed"

        if action == "check":
            res = subprocess.run(["pacman", "-Sy"], capture_output=True, text=True)
            return res.stdout
        elif action == "upgrade":
            # For a real implementation, you'd want to stream this or use a non-blocking approach
            res = subprocess.run(["pacman", "-Syu", "--noconfirm"], capture_output=True, text=True)
            return res.stdout
        return "Unknown action"

    def manage_user(self, sender, action, username, password):
        if not self.check_polkit(sender, "org.zohara.settings.manage-users"):
            return "Polkit authorization failed"

        if action == "add":
            res = subprocess.run(["useradd", "-m", username], capture_output=True, text=True)
            if password and res.returncode == 0:
                chpasswd = subprocess.run(
                    ["chpasswd"], input=f"{username}:{password}",
                    capture_output=True, text=True)
                if chpasswd.returncode != 0:
                    return "Failed"
            return "Success" if res.returncode == 0 else "Failed"
        elif action == "remove":
            res = subprocess.run(["userdel", "-r", username], capture_output=True, text=True)
            return "Success" if res.returncode == 0 else "Failed"
        return "Unknown action"


def main():
    loop = GLib.MainLoop()

    def on_bus_acquired(connection, name):
        node_info = Gio.DBusNodeInfo.new_for_xml(INTROSPECTION_XML)
        helper = ZoharaSettingsHelper(connection)
        connection.register_object(
            '/org/zohara/settings/Helper',
            node_info.interfaces[0],
            helper.handle_method_call,
            None, None)
        print("Zohara Settings Helper running on system bus...")

    def on_name_lost(connection, name):
        print(f"Failed to acquire bus name: {name}", file=sys.stderr)
        sys.exit(1)

    Gio.bus_own_name(
        Gio.BusType.SYSTEM, 'org.zohara.settings.Helper',
        Gio.BusNameOwnerFlags.NONE,
        None, on_bus_acquired, None, on_name_lost)

    try:
        loop.run()
    except Exception as e:
        print(f"Failed to start helper: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
