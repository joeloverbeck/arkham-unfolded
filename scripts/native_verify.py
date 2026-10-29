#!/usr/bin/env python3
"""Bounded Arkham GNOME/XWayland diagnostics; captures and grants stay local."""

import argparse
import ctypes as C
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import uuid

TITLE = "Arkham Unfolded — Scripted Rules Experiment"
REPOSITORY = Path(__file__).resolve().parent.parent


class Refused(RuntimeError):
    """A diagnostic precondition was not established; send no new press."""


def process_identity(pid):
    proc = Path("/proc") / str(pid)
    try:
        # comm may contain spaces or parentheses; starttime follows the final ')'.
        start = (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]
        return start, os.readlink(proc / "exe")
    except (OSError, IndexError) as error:
        raise Refused("App process is absent or not visible here") from error


def command(*args):
    try:
        return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT, timeout=5).strip()
    except (OSError, subprocess.SubprocessError) as error:
        raise Refused(f"Desktop command failed: {args[0]}") from error


class XImage(C.Structure):
    _fields_ = [
        ("width", C.c_int), ("height", C.c_int), ("xoffset", C.c_int),
        ("format", C.c_int), ("data", C.c_void_p), ("byte_order", C.c_int),
        ("bitmap_unit", C.c_int), ("bitmap_bit_order", C.c_int),
        ("bitmap_pad", C.c_int), ("depth", C.c_int), ("bytes_per_line", C.c_int),
        ("bits_per_pixel", C.c_int), ("red_mask", C.c_ulong),
        ("green_mask", C.c_ulong), ("blue_mask", C.c_ulong),
    ]


class Desktop:
    def __init__(self, pid, window):
        self.pid, self.window = pid, window
        self.identity = self.process_identity()
        self.x = C.CDLL("libX11.so.6")
        signatures = {
            "XOpenDisplay": (C.c_void_p, [C.c_char_p]),
            "XCloseDisplay": (C.c_int, [C.c_void_p]),
            "XSync": (C.c_int, [C.c_void_p, C.c_int]),
            "XSetErrorHandler": (C.c_void_p, [C.c_void_p]),
            "XGetGeometry": (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_int)] + [C.POINTER(C.c_uint)] * 4),
            "XTranslateCoordinates": (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong,
                C.c_int, C.c_int, C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_ulong)]),
            "XQueryPointer": (C.c_int, [C.c_void_p, C.c_ulong] + [C.POINTER(C.c_ulong)] * 2
                + [C.POINTER(C.c_int)] * 4 + [C.POINTER(C.c_uint)]),
            "XGetImage": (C.POINTER(XImage), [C.c_void_p, C.c_ulong, C.c_int, C.c_int,
                C.c_uint, C.c_uint, C.c_ulong, C.c_int]),
            "XDestroyImage": (C.c_int, [C.POINTER(XImage)]),
        }
        for name, (result, arguments) in signatures.items():
            function = getattr(self.x, name)
            function.restype, function.argtypes = result, arguments
        self.display = self.x.XOpenDisplay(None)
        if not self.display:
            raise Refused("No X11 display in this process context")
        self.xerror = False
        self.handler = C.CFUNCTYPE(C.c_int, C.c_void_p, C.c_void_p)(self.on_error)
        self.previous_handler = self.x.XSetErrorHandler(C.cast(self.handler, C.c_void_p))

    def on_error(self, _display, _error):
        self.xerror = True
        return 0

    def sync(self):
        self.x.XSync(self.display, 0)
        if self.xerror:
            self.xerror = False
            raise Refused("X11 client changed or its pixels are unavailable")

    def close(self):
        self.x.XCloseDisplay(self.display)
        self.x.XSetErrorHandler(self.previous_handler)

    def process_identity(self):
        return process_identity(self.pid)

    def focus(self):
        self.verify(require_focus=False)
        command("xdotool", "windowactivate", "--sync", str(self.window))
        self.verify()

    def verify(self, require_focus=True):
        window = str(self.window)
        if self.process_identity() != self.identity:
            raise Refused("App process identity changed")
        if command("xdotool", "getwindowpid", window) != str(self.pid):
            raise Refused("Window belongs to a different process")
        if command("xdotool", "getwindowname", window) != TITLE:
            raise Refused("Window is not the Arkham experiment client")
        if require_focus and (
            command("xdotool", "getactivewindow") != window
            or command("xdotool", "getwindowfocus", "-f") != window
        ):
            raise Refused("Arkham client lost focus")

    def geometry(self):
        root, child = C.c_ulong(), C.c_ulong()
        x, y = C.c_int(), C.c_int()
        width, height, border, depth = (C.c_uint() for _ in range(4))
        ok = self.x.XGetGeometry(self.display, self.window, C.byref(root), C.byref(x), C.byref(y),
                                C.byref(width), C.byref(height), C.byref(border), C.byref(depth))
        self.sync()
        if not ok:
            raise Refused("Cannot read client dimensions")
        ok = self.x.XTranslateCoordinates(self.display, self.window, root.value, 0, 0,
                                         C.byref(x), C.byref(y), C.byref(child))
        self.sync()
        if not ok or not (0 < width.value * height.value <= 25_000_000):
            raise Refused("Unsupported client geometry")
        return x.value, y.value, width.value, height.value

    def pointer(self):
        root, child = C.c_ulong(), C.c_ulong()
        rx, ry, wx, wy = (C.c_int() for _ in range(4))
        mask = C.c_uint()
        ok = self.x.XQueryPointer(self.display, self.window, C.byref(root), C.byref(child),
                                 C.byref(rx), C.byref(ry), C.byref(wx), C.byref(wy), C.byref(mask))
        self.sync()
        if not ok:
            raise Refused("Pointer is on another X11 screen")
        return (rx.value, ry.value), (wx.value, wy.value), mask.value

    def capture(self, path):
        from PIL import Image

        self.verify()
        _, _, width, height = self.geometry()
        pixels = self.x.XGetImage(self.display, self.window, 0, 0, width, height, C.c_ulong(-1).value, 2)
        try:
            self.sync()
            if not pixels:
                raise Refused("Client pixels unavailable")
            data = pixels.contents
            if (data.bits_per_pixel, data.byte_order, data.red_mask, data.green_mask, data.blue_mask) != (
                32, 0, 0xFF0000, 0xFF00, 0xFF
            ):
                raise Refused("Unsupported X11 pixel format")
            image = Image.frombytes("RGB", (width, height),
                                    C.string_at(data.data, data.bytes_per_line * height),
                                    "raw", "BGRX", data.bytes_per_line, 1)
            with path.open("xb") as output:
                image.save(output, format="PNG")
            return {"source": "XGetImage app client", "path": str(path), "size": [width, height],
                    "pixels_sha256": hashlib.sha256(image.tobytes()).hexdigest()}
        finally:
            if pixels:
                self.x.XDestroyImage(pixels)


class Controller:
    def __init__(self, desktop, portal, pause=time.sleep):
        self.desktop = desktop
        self.portal = portal
        self.pause = pause
        self.mapping = None

    def sample(self, x, y):
        self.desktop.verify()
        if self.desktop.pointer()[2]:
            raise Refused("Release held buttons and modifiers before moving")
        self.portal.move(x, y)
        self.pause(0.08)
        previous = self.desktop.pointer()[0]
        for _ in range(20):
            self.pause(0.02)
            current = self.desktop.pointer()[0]
            if current == previous:
                return current
            previous = current
        raise Refused("Pointer did not settle")

    def calibrate(self):
        self.mapping = None
        width, height = self.portal.size
        if width < 100 or height < 100:
            raise Refused("Selected monitor has no usable coordinate extent")
        # Keep probes near the monitor center, covered by the Arkham client.
        p0 = (width * 0.45, height * 0.45)
        p1 = (width * 0.55, height * 0.45)
        p2 = (width * 0.45, height * 0.55)
        r0, r1, r2 = (self.sample(*p) for p in (p0, p1, p2))
        sx = (r1[0] - r0[0]) / (p1[0] - p0[0])
        sy = (r2[1] - r0[1]) / (p2[1] - p0[1])
        if sx <= 0 or sy <= 0 or abs(r1[1] - r0[1]) > 2 or abs(r2[0] - r0[0]) > 2:
            raise Refused(f"Monitor mapping is not an aligned, positive scale: {r0}, {r1}, {r2}")
        ox, oy = r0[0] - sx * p0[0], r0[1] - sy * p0[1]
        check = (width * 0.5, height * 0.5)
        actual = self.sample(*check)
        if abs(actual[0] - (ox + sx * check[0])) > 2 or abs(actual[1] - (oy + sy * check[1])) > 2:
            raise Refused("Independent pointer calibration check failed")
        self.mapping = (sx, sy, ox, oy)
        return self.mapping

    def click(self, x, y):
        if self.mapping is None:
            raise Refused("Calibrate before clicking")
        self.desktop.verify()
        geometry = self.desktop.geometry()
        left, top, width, height = geometry
        if not all(math.isfinite(v) for v in (x, y)) or not (0 <= x < width and 0 <= y < height):
            raise Refused("Target lies outside the app client")
        sx, sy, ox, oy = self.mapping
        px, py = (left + x - ox) / sx, (top + y - oy) / sy
        if not (0 <= px < self.portal.size[0] and 0 <= py < self.portal.size[1]):
            raise Refused("Target lies outside the selected monitor mapping")
        self.sample(px, py)
        self.desktop.verify()
        _, client, mask = self.desktop.pointer()
        if self.desktop.geometry() != geometry or mask or abs(client[0] - x) > 2 or abs(client[1] - y) > 2:
            raise Refused("Window, pointer, or button state changed before press")
        try:
            self.portal.button(True)
            self.pause(0.1)
        finally:
            self.portal.button(False)

    def right_drag(self, x, y, dx, dy, duration_seconds=0.6):
        values = (x, y, dx, dy, duration_seconds)
        try:
            if any(isinstance(value, bool) or not isinstance(value, (int, float)) for value in values):
                raise ValueError
            x, y, dx, dy, duration_seconds = (float(value) for value in values)
            if not all(math.isfinite(value) for value in (x, y, dx, dy, duration_seconds)):
                raise ValueError
        except (ValueError, OverflowError) as error:
            raise Refused("Drag coordinates and duration must be finite numbers") from error
        if not 0.2 <= duration_seconds <= 2 or not 0 < abs(dx) + abs(dy) <= 600:
            raise Refused("Drag pacing must be 0.2–2 seconds with nonzero travel of at most 600 client pixels")
        if self.mapping is None:
            raise Refused("Calibrate before dragging")
        self.desktop.verify()
        geometry = self.desktop.geometry()
        left, top, width, height = geometry
        sx, sy, ox, oy = self.mapping
        points = [(x + dx * step / 20, y + dy * step / 20) for step in range(21)]
        mapped = [((left + cx - ox) / sx, (top + cy - oy) / sy) for cx, cy in points]
        if any(not (0 <= cx < width and 0 <= cy < height) for cx, cy in points):
            raise Refused("Drag leaves the app client")
        if any(not (0 <= px < self.portal.size[0] and 0 <= py < self.portal.size[1]) for px, py in mapped):
            raise Refused("Drag leaves the selected monitor mapping")

        def verify_pointer(expected, expected_mask):
            self.desktop.verify()
            _, actual, mask = self.desktop.pointer()
            if (self.desktop.geometry() != geometry or mask != expected_mask
                    or any(abs(a - b) > 2 for a, b in zip(actual, expected))):
                raise Refused("Window, pointer, or button state changed during right-drag")

        self.sample(*mapped[0])
        verify_pointer(points[0], 0)
        try:
            self.portal.button(True, 273)  # Linux BTN_RIGHT, X11 Button3Mask below.
            self.pause(0.1)
            previous = points[0]
            for expected, target in zip(points[1:], mapped[1:]):
                verify_pointer(previous, 1 << 10)
                self.portal.move(*target)
                self.pause(duration_seconds / 20)
                verify_pointer(expected, 1 << 10)
                previous = expected
        finally:
            try:
                self.portal.button(False, 273)
            except KeyboardInterrupt:
                # A second release is safe if interruption obscured delivery.
                self.portal.button(False, 273)
                raise


class Portal:
    def __init__(self, emit):
        import dbus
        from dbus.mainloop.glib import DBusGMainLoop
        from gi.repository import GLib

        self.dbus, self.glib, self.emit = dbus, GLib, emit
        DBusGMainLoop(set_as_default=True)
        self.bus = dbus.SessionBus()
        self.object = self.bus.get_object("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop")
        self.remote = dbus.Interface(self.object, "org.freedesktop.portal.RemoteDesktop")
        self.cast = dbus.Interface(self.object, "org.freedesktop.portal.ScreenCast")
        self.session = None

    def request(self, method, *args, **options):
        token = "arkham_" + uuid.uuid4().hex
        sender = self.bus.get_unique_name()[1:].replace(".", "_")
        path = f"/org/freedesktop/portal/desktop/request/{sender}/{token}"
        result = {}
        loop = self.glib.MainLoop()

        def response(code, values):
            result.update(code=int(code), values=values)
            loop.quit()

        def timeout():
            response(-1, {})
            return False

        match = self.bus.add_signal_receiver(response, signal_name="Response",
                                            dbus_interface="org.freedesktop.portal.Request", path=path)
        timer = self.glib.timeout_add_seconds(600, timeout)
        try:
            options["handle_token"] = token
            handle = method(*args, self.dbus.Dictionary(options, signature="sv"))
            if str(handle) != path:
                raise Refused("Portal returned an unexpected request identity")
            loop.run()
            if result.get("code") != 0:
                raise Refused(f"Portal request denied, canceled, or timed out: {result.get('code')}")
            return result["values"]
        finally:
            if result.get("code") != -1:
                self.glib.source_remove(timer)
            match.remove()
            if "code" not in result or result["code"] == -1:
                # An interrupted request can still own a visible consent dialog.
                try:
                    self.dbus.Interface(self.bus.get_object("org.freedesktop.portal.Desktop", path),
                                        "org.freedesktop.portal.Request").Close()
                except self.dbus.DBusException:
                    pass  # The service may already have closed it; Session.Close follows.

    def start(self):
        self.session = self.request(self.remote.CreateSession,
                                    session_handle_token="arkham_" + uuid.uuid4().hex)["session_handle"]
        self.request(self.remote.SelectDevices, self.session,
                     types=self.dbus.UInt32(2), persist_mode=self.dbus.UInt32(0))
        self.request(self.cast.SelectSources, self.session,
                     types=self.dbus.UInt32(1), multiple=self.dbus.Boolean(False))
        self.emit({"status": "awaiting-desktop-consent"})
        grant = self.request(self.remote.Start, self.session, "")
        streams = grant.get("streams", [])
        if not int(grant.get("devices", 0)) & 2 or len(streams) != 1:
            raise Refused("Grant must include pointer access and exactly one monitor")
        self.stream, properties = streams[0]
        if int(properties.get("source_type", 1)) != 1:
            raise Refused("Selected source is not a monitor")
        self.size = tuple(int(v) for v in properties.get("logical_size", properties.get("size", ())))
        if len(self.size) != 2:
            raise Refused("Portal supplied no monitor extent")
        self.emit({"status": "granted", "stream_size": self.size})

    def move(self, x, y):
        self.remote.NotifyPointerMotionAbsolute(self.session, {}, self.dbus.UInt32(self.stream),
                                                self.dbus.Double(x), self.dbus.Double(y))

    def button(self, pressed, button=272):
        self.remote.NotifyPointerButton(self.session, {}, self.dbus.Int32(button), self.dbus.UInt32(pressed))

    def close(self):
        if self.session is not None:
            try:
                self.dbus.Interface(self.bus.get_object("org.freedesktop.portal.Desktop", self.session),
                                    "org.freedesktop.portal.Session").Close()
            except self.dbus.DBusException as error:
                if error.get_dbus_name() != "org.freedesktop.DBus.Error.UnknownObject":
                    raise
            finally:
                self.session = None


def capture_path(directory, name):
    if not isinstance(name, str) or Path(name).name != name or Path(name).suffix != ".png":
        raise Refused("Use a plain PNG filename inside this run's evidence directory")
    path = directory / name
    if path.exists():
        raise Refused("Capture already exists; choose a new filename")
    return path


def run_session(desktop, portal, directory, commands, emit, pause=time.sleep):
    try:
        desktop.focus()
        emit({"status": "capture", **desktop.capture(capture_path(directory, "initial.png"))})
        portal.start()
        controller = Controller(desktop, portal, pause)
        try:
            desktop.focus()
            emit({"status": "ready", "mapping": controller.calibrate()})
        except Refused as error:
            emit({"status": "refused", "reason": str(error)})
        for line in commands:
            try:
                request = json.loads(line)
                action = request["action"]
                if action == "stop":
                    break
                if action == "capture":
                    emit({"status": "capture", **desktop.capture(capture_path(directory, request["name"]))})
                elif action == "click":
                    controller.click(float(request["x"]), float(request["y"]))
                    emit({"status": "clicked", "client": [request["x"], request["y"]]})
                elif action == "right-drag":
                    duration = request.get("duration_seconds", 0.6)
                    controller.right_drag(request["x"], request["y"], request["dx"], request["dy"], duration)
                    emit({"status": "right-dragged", "client": [request["x"], request["y"]],
                          "delta": [request["dx"], request["dy"]], "duration_seconds": duration})
                elif action == "focus":
                    desktop.focus()
                    emit({"status": "focused"})
                elif action == "calibrate":
                    emit({"status": "ready", "mapping": controller.calibrate()})
                else:
                    raise Refused("Unknown action: use capture, click, right-drag, focus, calibrate, or stop")
            except (Refused, ValueError, TypeError, KeyError) as error:
                emit({"status": "refused", "reason": str(error)})
    finally:
        portal.close()
        emit({"status": "session-closed"})


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def launch_diagnostic(args):
    directory = args.output_dir.resolve()
    binary = args.binary.resolve(strict=True)
    source_save = args.load.resolve(strict=True) if args.load else None
    if directory.is_relative_to(REPOSITORY):
        raise Refused("Diagnostic run must be outside the source checkout")
    if os.path.lexists(args.output_dir):
        raise Refused("Use a fresh diagnostic directory; existing runs are preserved")
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise Refused("Selected binary must be an executable file")
    if source_save and not source_save.is_file():
        raise Refused("Selected save must be a file")
    if not math.isfinite(args.timeout_seconds) or not 0 < args.timeout_seconds <= 3600:
        raise Refused("Launch timeout must be greater than zero and at most 3600 seconds")
    if args.quit_after_screenshot and not args.screenshot:
        raise Refused("--quit-after-screenshot requires --screenshot NAME.png")
    screenshot = capture_path(directory, args.screenshot) if args.screenshot else None
    directory.mkdir(parents=True)
    staged = directory / 'arkham-tabletop'
    before = digest(binary)
    shutil.copyfile(binary, staged)
    if digest(staged) != before or digest(binary) != before:
        raise Refused("Selected binary changed while staging; retry in a fresh directory")
    staged.chmod(0o555)
    save = directory / 'session.json'
    save_digest = None
    if source_save:
        save_digest = digest(source_save)
        shutil.copyfile(source_save, save)
        if digest(save) != save_digest or digest(source_save) != save_digest:
            raise Refused("Selected save changed while copying; retry in a fresh directory")
    isolated = {name: str(directory / child) for name, child in (
        ('XDG_CONFIG_HOME', 'config'), ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache'))}
    for path in isolated.values():
        Path(path).mkdir()
    argv = [str(staged)]
    if args.experiment == 'tony':
        argv += ['--experiment', 'tony']
    argv += ['--load' if source_save else '--save', str(save)]
    if args.art_root:
        argv += ['--art-root', str(args.art_root.resolve())]
    elif args.no_art:
        argv += ['--no-art']
    if screenshot:
        argv += ['--screenshot', str(screenshot)]
    if args.quit_after_screenshot:
        argv += ['--quit-after-screenshot']
    metadata = {
        'status': 'staged', 'source_binary': str(binary), 'binary': str(staged),
        'sha256': before, 'argv': argv, 'cwd': str(directory), 'environment': isolated,
        'save': str(save), 'source_save': str(source_save) if source_save else None,
        'source_save_sha256': save_digest, 'log': str(directory / 'app.log'),
        'timeout_seconds': args.timeout_seconds,
    }

    def record():
        temporary = directory / 'run.json.tmp'
        temporary.write_text(json.dumps(metadata, indent=2) + '\n')
        temporary.replace(directory / 'run.json')

    record()
    with (directory / 'app.log').open('xb') as log:
        process = subprocess.Popen(argv, cwd=directory, env={**os.environ, **isolated},
                                   stdin=subprocess.DEVNULL, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            metadata['pid'] = process.pid
            start, executable = process_identity(process.pid)
            metadata.update(status='running', start_ticks=start, executable=executable,
                            executable_sha256=digest(Path('/proc') / str(process.pid) / 'exe'))
            record()
            print(json.dumps(metadata), flush=True)
            try:
                code = process.wait(timeout=args.timeout_seconds)
                metadata['status'] = 'exited'
            except subprocess.TimeoutExpired:
                code = 124
                metadata['status'] = 'timed-out'
        except BaseException:
            metadata['status'] = 'interrupted-or-failed'
            raise
        finally:
            # Only this launch owns the new session/process group. Leave no app
            # running when the supervisor times out, is interrupted, or fails.
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
            finally:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=2)
            metadata['returncode'] = process.returncode
            record()
    print(json.dumps(metadata), flush=True)
    return code if code >= 0 else 128 - code


def build_diagnostic(source, directory, reference):
    source, directory, reference = (Path(p).resolve() for p in (source, directory, reference))
    if directory.is_relative_to(source) or directory.is_relative_to(REPOSITORY):
        raise Refused("Diagnostic output must be outside both source checkouts")
    if reference.is_relative_to(directory) or directory.exists():
        raise Refused("Use a fresh diagnostic directory separate from the preserved executable")
    if not (source / 'Cargo.toml').is_file():
        raise Refused("Source has no Cargo.toml")
    before = digest(reference)
    directory.mkdir(parents=True)
    target = directory / 'target'
    try:
        with (directory / 'build.log').open('xb') as log:
            subprocess.run(['cargo', 'build', '--locked', '--offline', '--manifest-path',
                            str(source / 'Cargo.toml'), '--features', 'tabletop', '--bin',
                            'arkham-tabletop', '--target-dir', str(target)], cwd=source,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
    finally:
        if digest(reference) != before:
            raise Refused("Preserved executable changed during the diagnostic build")
    binary = target / 'debug/arkham-tabletop'
    return {'binary': str(binary), 'sha256': digest(binary), 'reference_sha256': before,
            'log': str(directory / 'build.log')}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest='mode', required=True)
    for name in ('capture', 'session'):
        mode = modes.add_parser(name)
        mode.add_argument('--pid', type=int, required=True)
        mode.add_argument('--window', type=lambda value: int(value, 0), required=True)
        mode.add_argument('--output' if name == 'capture' else '--output-dir', type=Path, required=True)
    build = modes.add_parser('build', help='build a diagnostic source copy in a fresh external target')
    build.add_argument('--source', type=Path, default=REPOSITORY)
    build.add_argument('--output-dir', type=Path, required=True)
    build.add_argument('--reference', type=Path, default=REPOSITORY / 'target/debug/arkham-tabletop')
    launch = modes.add_parser('launch', help='stage a fixed binary and supervise an isolated native run')
    launch.add_argument('--binary', type=Path, default=REPOSITORY / 'target/debug/arkham-tabletop')
    launch.add_argument('--output-dir', type=Path, required=True)
    launch.add_argument('--experiment', choices=('jacqueline', 'tony'), default='jacqueline')
    launch.add_argument('--load', type=Path, help='copy this save before opening it; never autosave to the original')
    art = launch.add_mutually_exclusive_group()
    art.add_argument('--art-root', type=Path)
    art.add_argument('--no-art', action='store_true')
    launch.add_argument('--screenshot', help='plain PNG filename inside the fresh run directory')
    launch.add_argument('--quit-after-screenshot', action='store_true')
    launch.add_argument('--timeout-seconds', type=float, default=300)
    args = parser.parse_args()

    def interrupted(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, interrupted)
    if args.mode == 'launch':
        return launch_diagnostic(args)
    if args.mode == 'build':
        print(json.dumps(build_diagnostic(args.source, args.output_dir, args.reference)), flush=True)
        return
    desktop = Desktop(args.pid, args.window)
    try:
        if args.mode == 'capture':
            desktop.focus()
            args.output.parent.mkdir(parents=True, exist_ok=True)
            print(json.dumps(desktop.capture(args.output)), flush=True)
        else:
            args.output_dir.mkdir(parents=True, exist_ok=False)
            with (args.output_dir / 'events.jsonl').open('x') as log:
                def emit(event):
                    line = json.dumps({'time': time.time(), 'pid': args.pid, 'window': args.window, **event})
                    print(line, flush=True)
                    log.write(line + '\n')
                    log.flush()

                run_session(desktop, Portal(emit), args.output_dir, sys.stdin, emit)
    finally:
        desktop.close()


if __name__ == '__main__':
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
    except (Refused, OSError, ImportError, subprocess.CalledProcessError) as error:
        print(json.dumps({'status': 'failed', 'reason': str(error)}), file=sys.stderr)
        sys.exit(1)
