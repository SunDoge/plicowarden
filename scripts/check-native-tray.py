#!/usr/bin/python3
"""Linux smoke: isolated X11 + D-Bus tray/notification services, synthetic probe only."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True, help='prebuilt desktop_probe example')
    parser.add_argument('--inside', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not args.inside:
        return subprocess.call(['dbus-run-session', '--', '/usr/bin/python3', __file__, '--inside', '--binary', str(args.binary.resolve())])
    import dbus
    import dbus.service
    from dbus.mainloop.glib import DBusGMainLoop
    from gi.repository import GLib
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    items = []
    notifications = []

    class Watcher(dbus.service.Object):
        @dbus.service.method('org.kde.StatusNotifierWatcher', in_signature='s', sender_keyword='sender')
        def RegisterStatusNotifierItem(self, service, sender=None):
            items.append((sender, service) if str(service).startswith('/') else (str(service), '/StatusNotifierItem'))

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='ss', out_signature='v')
        def Get(self, interface, prop):
            return dbus.Boolean(True) if prop == 'IsStatusNotifierHostRegistered' else dbus.Int32(0)

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='s', out_signature='a{sv}')
        def GetAll(self, interface):
            return {'IsStatusNotifierHostRegistered': dbus.Boolean(True)}

    class Notifications(dbus.service.Object):
        @dbus.service.method('org.freedesktop.Notifications', in_signature='susssasa{sv}i', out_signature='u')
        def Notify(self, app, replaces, icon, summary, body, actions, hints, timeout):
            notifications.append((str(summary), str(body), list(actions)))
            return dbus.UInt32(1)

        @dbus.service.method('org.freedesktop.Notifications', out_signature='as')
        def GetCapabilities(self):
            return ['actions', 'body']

        @dbus.service.method('org.freedesktop.Notifications', out_signature='ssss')
        def GetServerInformation(self):
            return ('Plicowarden synthetic test', 'Plicowarden', '1', '1.2')

        @dbus.service.method('org.freedesktop.Notifications', in_signature='u')
        def CloseNotification(self, notification_id):
            self.NotificationClosed(notification_id, 3)

        @dbus.service.signal('org.freedesktop.Notifications', signature='us')
        def ActionInvoked(self, notification_id, action):
            pass

        @dbus.service.signal('org.freedesktop.Notifications', signature='uu')
        def NotificationClosed(self, notification_id, reason):
            pass

    watcher_name = dbus.service.BusName('org.kde.StatusNotifierWatcher', bus)
    notification_name = dbus.service.BusName('org.freedesktop.Notifications', bus)
    watcher = Watcher(watcher_name, '/StatusNotifierWatcher')
    notification = Notifications(notification_name, '/org/freedesktop/Notifications')
    context = GLib.MainContext.default()

    def wait(check, label, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            while context.pending():
                context.iteration(False)
            value = check()
            if value:
                return value
            time.sleep(.01)
        raise AssertionError(label)

    def visible():
        return subprocess.run(['xdotool', 'search', '--onlyvisible', '--name', '^Plicowarden$'], capture_output=True, text=True).stdout.strip().splitlines()

    with tempfile.TemporaryDirectory(prefix='plicowarden-native-') as temporary:
        read_fd, write_fd = os.pipe()
        xvfb = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1600x1100x24', '-nolisten', 'tcp'], pass_fds=[write_fd], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        os.close(write_fd)
        with os.fdopen(read_fd) as read:
            display = read.readline().strip()
        assert display, 'Xvfb failed to start'
        os.environ['DISPLAY'] = ':' + display
        os.environ.pop('WAYLAND_DISPLAY', None)
        os.environ['WINIT_UNIX_BACKEND'] = 'x11'
        os.environ['XDG_CONFIG_HOME'] = temporary
        os.environ['XDG_DATA_HOME'] = temporary
        os.environ['XDG_CACHE_HOME'] = temporary
        log = Path(temporary) / 'probe.log'
        try:
            with log.open('w') as output:
                app = subprocess.Popen([str(args.binary.resolve())], stdout=output, stderr=output)
                try:
                    wait(lambda: items and notifications and visible(), 'tray, notification or window did not initialize')
                    summary, body, actions = notifications[0]
                    assert summary == 'SSH signing approval'
                    assert body == 'Open Plicowarden to review a pending request.'
                    assert 'default' in actions
                    service, path = items[0]
                    item = bus.get_object(service, path)
                    properties = dbus.Interface(item, 'org.freedesktop.DBus.Properties')
                    menu_path = properties.Get('org.kde.StatusNotifierItem', 'Menu')
                    menu = dbus.Interface(bus.get_object(service, menu_path), 'com.canonical.dbusmenu')
                    _, layout = menu.GetLayout(0, -1, [])
                    entries = {str(child[1].get('label', '')): int(child[0]) for child in layout[2]}
                    for label in ['Open Plicowarden', 'Lock vault', 'Quit Plicowarden', 'Keep running when closed', 'SSH approval notifications']:
                        assert label in entries, entries
                    # Give the watcher probe time to report availability before closing.
                    wait(lambda: time.monotonic() - started > 4, 'watcher check')

                    def choose(label):
                        menu.Event(entries[label], 'clicked', dbus.Int32(0, variant_level=1), dbus.UInt32(0))

                    def close():
                        window = visible()[0]
                        subprocess.run(['xdotool', 'windowfocus', window, 'mousemove', '--window', window, '36', '40', 'click', '1'], check=True)
                        wait(lambda: not visible(), 'close did not hide window')
                        assert app.poll() is None, 'close exited while tray was available'

                    close()
                    choose('Open Plicowarden')
                    wait(visible, 'tray open did not restore window')
                    close()
                    notification.ActionInvoked(1, 'default')
                    wait(visible, 'notification activation did not restore window')
                    choose('Lock vault')
                    assert app.poll() is None
                    choose('SSH approval notifications')
                    wait(lambda: any(Path(temporary).rglob('desktop.json')), 'preferences were not persisted')
                    saved = next(Path(temporary).rglob('desktop.json')).read_text()
                    assert '"sshNotifications": false' in saved
                    close()
                    watcher.remove_from_connection()
                    bus.release_name('org.kde.StatusNotifierWatcher')
                    wait(visible, 'lost tray host left the window hidden')
                    choose('Quit Plicowarden')
                    wait(lambda: app.poll() is not None, 'quit did not exit')
                    assert app.returncode == 0, log.read_text()
                    print('PASS native tray menu, close/hide, restore, notification activation, preferences, host loss, quit')
                    # A desktop without a StatusNotifier host must still have a reachable exit.
                    fresh = str(Path(temporary) / 'without-tray-host')
                    for variable in ['XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']:
                        os.environ[variable] = fresh
                    app = subprocess.Popen([str(args.binary.resolve())], stdout=output, stderr=output)
                    wait(visible, 'fallback window did not initialize')
                    fallback_start = time.monotonic()
                    wait(lambda: time.monotonic() - fallback_start > 4, 'fallback watcher check')
                    window = visible()[0]
                    subprocess.run(['xdotool', 'windowfocus', window, 'mousemove', '--window', window, '36', '40', 'click', '1'], check=True)
                    wait(lambda: app.poll() is not None, 'close without tray host failed to exit')
                    assert app.returncode == 0, log.read_text()
                    print('PASS close without tray host exits normally')
                finally:
                    if app.poll() is None:
                        app.terminate()
                        app.wait(timeout=10)
        except Exception:
            print(log.read_text(), file=sys.stderr)
            raise
        finally:
            xvfb.terminate()
            xvfb.wait(timeout=10)
    return 0


if __name__ == '__main__':
    started = time.monotonic()
    raise SystemExit(main())
