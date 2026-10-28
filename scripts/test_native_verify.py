"""Behavior checks for the native diagnostic command boundary (no desktop needed)."""

import io
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import native_verify as native


class Desktop:
    def __init__(self):
        self.root = (0, 0)
        self.focused = True
        self.valid_identity = True
        self.bounds = (1000, 500, 800, 600)
        self.mask = 0

    def verify(self):
        if not self.valid_identity or not self.focused:
            raise native.Refused("identity or focus changed")

    def focus(self):
        self.focused = True
        self.verify()

    def capture(self, path):
        path.write_bytes(b"independent client pixels")
        return {"path": str(path), "source": "test desktop"}

    def geometry(self):
        return self.bounds

    def pointer(self):
        x, y, _, _ = self.bounds
        return self.root, (self.root[0] - x, self.root[1] - y), self.mask


class Portal:
    size = (3072, 1728)

    def __init__(self, desktop):
        self.desktop = desktop
        self.events = []
        self.last_move = None
        self.lose_focus = False
        self.starts = 0
        self.closes = 0

    def start(self):
        self.starts += 1

    def close(self):
        self.closes += 1

    def move(self, x, y):
        self.last_move = (x, y)
        self.desktop.root = (round(100 + x * 1.6), round(60 + y * 1.6))
        if self.lose_focus:
            self.desktop.focused = False

    def button(self, pressed):
        self.events.append(pressed)


class NativeCommands(unittest.TestCase):
    def test_bad_identity_and_nonfinite_or_off_window_targets_never_press(self):
        desktop = Desktop()
        portal = Portal(desktop)
        controller = native.Controller(desktop, portal, pause=lambda _: None)
        controller.calibrate()
        for x, y in [(800, 250), (400, -1), (float('nan'), 0), (float('inf'), 0)]:
            with self.assertRaises(native.Refused):
                controller.click(x, y)
        desktop.valid_identity = False
        with self.assertRaises(native.Refused):
            controller.click(400, 250)
        self.assertEqual(portal.events, [])

    def test_capture_names_preserve_existing_evidence_and_stay_in_run_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'before.png').write_bytes(b'original')
            for name in ['before.png', '../outside.png', '/tmp/outside.png', 'bad.txt']:
                with self.assertRaises(native.Refused):
                    native.capture_path(root, name)
            self.assertEqual((root / 'before.png').read_bytes(), b'original')
            self.assertEqual(native.capture_path(root, 'after.png'), root / 'after.png')

    def test_one_grant_survives_refusal_and_closes_on_end_of_input(self):
        desktop = Desktop()
        portal = Portal(desktop)
        events = []
        with tempfile.TemporaryDirectory() as directory:
            native.run_session(desktop, portal, Path(directory), io.StringIO(
                '{"action":"click","x":-1,"y":250}\n'
                '{"action":"click","x":400,"y":250}\n'
            ), events.append, pause=lambda _: None)
        self.assertEqual((portal.starts, portal.closes), (1, 1))
        self.assertEqual(portal.events, [True, False])
        self.assertTrue(any(event['status'] == 'refused' for event in events))

    def test_startup_refusal_can_be_recalibrated_without_another_grant(self):
        portal = Portal(Desktop())
        portal.size = (0, 0)
        events = []

        def recovery():
            portal.size = (3072, 1728)
            yield '{"action":"calibrate"}'
            yield '{"action":"click","x":400,"y":250}'
            yield '{"action":"stop"}'

        with tempfile.TemporaryDirectory() as directory:
            native.run_session(portal.desktop, portal, Path(directory), recovery(),
                               events.append, pause=lambda _: None)
        self.assertEqual((portal.starts, portal.closes), (1, 1))
        self.assertEqual(portal.events, [True, False])
        self.assertEqual([e['status'] for e in events],
                         ['capture', 'refused', 'ready', 'clicked', 'session-closed'])

    def test_changed_mapping_geometry_or_held_button_never_press(self):
        for change in ('mapping', 'geometry', 'button'):
            with self.subTest(change=change):
                desktop = Desktop()
                portal = Portal(desktop)
                controller = native.Controller(desktop, portal, pause=lambda _: None)
                controller.calibrate()
                move = portal.move

                def changed_move(x, y):
                    move(x, y)
                    if change == 'mapping':
                        desktop.root = (desktop.root[0] + 30, desktop.root[1])
                    elif change == 'geometry':
                        desktop.bounds = (1000, 500, 801, 600)
                    else:
                        desktop.mask = 256

                portal.move = changed_move
                with self.assertRaises(native.Refused):
                    controller.click(400, 250)
                self.assertEqual(portal.events, [])

    def test_interrupted_press_releases_button_and_closes_grant(self):
        desktop = Desktop()
        portal = Portal(desktop)

        def interrupt_on_press(pressed):
            portal.events.append(pressed)
            if pressed:
                raise KeyboardInterrupt

        portal.button = interrupt_on_press
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(KeyboardInterrupt):
                native.run_session(desktop, portal, Path(directory),
                                   ['{"action":"click","x":400,"y":250}'],
                                   lambda _: None, pause=lambda _: None)
        self.assertEqual(portal.events, [True, False])
        self.assertEqual(portal.closes, 1)

    def test_partial_portal_start_is_closed(self):
        portal = Portal(Desktop())

        def failed_start():
            portal.starts += 1
            raise native.Refused('source selection failed after CreateSession')

        portal.start = failed_start
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(native.Refused):
                native.run_session(portal.desktop, portal, Path(directory), [], lambda _: None)
        self.assertEqual((portal.starts, portal.closes), (1, 1))

    def test_measured_mapping_reaches_client_target_and_refuses_lost_focus(self):
        desktop = Desktop()
        portal = Portal(desktop)
        controller = native.Controller(desktop, portal, pause=lambda _: None)
        controller.calibrate()
        controller.click(400, 250)
        self.assertAlmostEqual(desktop.pointer()[1][0], 400, delta=2)
        self.assertAlmostEqual(desktop.pointer()[1][1], 250, delta=2)
        self.assertEqual(portal.events, [True, False])

        portal.events.clear()
        portal.lose_focus = True
        with self.assertRaises(native.Refused):
            controller.click(400, 250)
        self.assertEqual(portal.events, [])


class DiagnosticBuild(unittest.TestCase):
    @unittest.skipUnless(shutil.which('cargo'), 'Cargo required for the real build-isolation check')
    def test_real_cargo_build_cannot_select_the_inherited_target_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'Cargo.toml').write_text(
                '[package]\nname="native-diagnostic-fixture"\nversion="0.1.0"\nedition="2024"\n'
                '[features]\ntabletop=[]\n[[bin]]\nname="arkham-tabletop"\npath="main.rs"\n'
            )
            (source / 'main.rs').write_text('fn main() { println!("isolated diagnostic"); }\n')
            subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path',
                            str(source / 'Cargo.toml')], check=True, capture_output=True)
            normal = root / 'normal-target'
            (normal / 'debug').mkdir(parents=True)
            reference = normal / 'debug/arkham-tabletop'
            reference.write_bytes(b'preserved original executable')
            with patch.dict(os.environ, {'CARGO_TARGET_DIR': str(normal)}):
                result = native.build_diagnostic(source, root / 'diagnostic', reference)
            self.assertEqual(reference.read_bytes(), b'preserved original executable')
            self.assertEqual(subprocess.check_output([result['binary']], text=True).strip(),
                             'isolated diagnostic')
            with self.assertRaises(native.Refused):
                native.build_diagnostic(source, source / 'target/new', reference)

            (source / 'main.rs').write_text('this is not valid Rust\n')
            with self.assertRaises(subprocess.CalledProcessError):
                native.build_diagnostic(source, root / 'failed-build', reference)
            self.assertEqual(reference.read_bytes(), b'preserved original executable')


class DiagnosticLaunch(unittest.TestCase):
    def binary(self, root, body):
        binary = root / 'candidate'
        binary.write_text(f'#!{sys.executable}\n' + body)
        binary.chmod(0o755)
        return binary

    def command(self, binary, directory, *options):
        return [sys.executable, '-B', str(native.REPOSITORY / 'scripts/native_verify.py'),
                'launch', '--binary', str(binary), '--output-dir', str(directory), *options]

    def test_cli_stages_fixed_binary_and_only_autosaves_to_its_isolated_copy(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = self.binary(root, '''import json, os, sys, time
from pathlib import Path
args = sys.argv[1:]
save = Path(args[args.index('--load') + 1])
save.write_text('autosaved copy')
print(json.dumps({'argv': args, 'cwd': os.getcwd(),
    'config': os.environ['XDG_CONFIG_HOME'], 'data': os.environ['XDG_DATA_HOME']}), flush=True)
while not Path('finish').exists():
    time.sleep(0.01)
print('original executable', flush=True)
''')
            original_binary = binary.read_bytes()
            original_save = root / 'owner.session.json'
            original_save.write_bytes(b'owner pending decision')
            directory = root / 'run'
            with subprocess.Popen(self.command(binary, directory, '--experiment', 'tony',
                    '--load', str(original_save), '--art-root', str(root / 'art'),
                    '--screenshot', 'render.png', '--quit-after-screenshot'),
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as process:
                try:
                    self.assertTrue(select.select([process.stdout], [], [], 5)[0])
                    running = json.loads(process.stdout.readline())
                    self.assertEqual(running['status'], 'running')
                    self.assertEqual(native.process_identity(running['pid']),
                                     (running['start_ticks'], running['executable']))
                    # Cargo normally replaces the output inode on rebuild.
                    replacement = root / 'rebuilt'
                    replacement.write_bytes(b'new build bytes')
                    replacement.replace(binary)
                    (directory / 'finish').touch()
                    output, error = process.communicate(timeout=5)
                    self.assertEqual(process.returncode, 0, error)
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.communicate(timeout=5)
            completed = json.loads(output)
            self.assertEqual(completed['status'], 'exited')
            self.assertEqual((directory / 'arkham-tabletop').read_bytes(), original_binary)
            self.assertEqual(completed['sha256'], hashlib.sha256(original_binary).hexdigest())
            self.assertEqual(original_save.read_bytes(), b'owner pending decision')
            self.assertEqual((directory / 'session.json').read_text(), 'autosaved copy')
            log = (directory / 'app.log').read_text().splitlines()
            facts = json.loads(log[0])
            self.assertEqual(log[1], 'original executable')
            self.assertEqual(facts['cwd'], str(directory))
            self.assertEqual(facts['config'], str(directory / 'config'))
            self.assertEqual(facts['data'], str(directory / 'data'))
            self.assertEqual(facts['argv'], [
                '--experiment', 'tony', '--load', str(directory / 'session.json'),
                '--art-root', str(root / 'art'), '--screenshot', str(directory / 'render.png'),
                '--quit-after-screenshot'])
            self.assertEqual(json.loads((directory / 'run.json').read_text()), completed)

    def test_cli_preserves_existing_run_and_refuses_escaping_capture(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = self.binary(root, "raise RuntimeError('must never launch')\n")
            existing = root / 'existing'
            existing.mkdir()
            marker = existing / 'app.log'
            marker.write_bytes(b'prior evidence')
            result = subprocess.run(self.command(binary, existing), capture_output=True,
                                    text=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertIn('fresh diagnostic directory', result.stderr)
            self.assertEqual(marker.read_bytes(), b'prior evidence')
            fresh = root / 'fresh'
            result = subprocess.run(self.command(binary, fresh, '--screenshot', '../escape.png'),
                                    capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertFalse(fresh.exists())

    def test_cli_timeout_kills_its_child_and_new_save_stays_in_run(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = self.binary(root, '''import signal, sys, time
from pathlib import Path
signal.signal(signal.SIGTERM, signal.SIG_IGN)
Path(sys.argv[sys.argv.index('--save') + 1]).write_text('new diagnostic save')
while True:
    time.sleep(1)
''')
            directory = root / 'run'
            result = subprocess.run(self.command(binary, directory, '--timeout-seconds', '1',
                                                '--no-art'),
                                    capture_output=True, text=True, timeout=6)
            self.assertEqual(result.returncode, 124, result.stderr)
            completed = json.loads(result.stdout.splitlines()[-1])
            self.assertEqual(completed['status'], 'timed-out')
            self.assertFalse((Path('/proc') / str(completed['pid'])).exists())
            self.assertEqual((directory / 'session.json').read_text(), 'new diagnostic save')
            self.assertEqual(completed['argv'], [str(directory / 'arkham-tabletop'),
                             '--save', str(directory / 'session.json'), '--no-art'])

    def test_cli_interruption_cleans_up_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = self.binary(root, 'import time\ntime.sleep(60)\n')
            directory = root / 'run'
            with subprocess.Popen(self.command(binary, directory), stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, text=True) as process:
                try:
                    self.assertTrue(select.select([process.stdout], [], [], 5)[0])
                    running = json.loads(process.stdout.readline())
                    process.terminate()
                    process.communicate(timeout=5)
                    self.assertEqual(process.returncode, 130)
                    self.assertFalse((Path('/proc') / str(running['pid'])).exists())
                    self.assertEqual(json.loads((directory / 'run.json').read_text())['status'],
                                     'interrupted-or-failed')
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate(timeout=5)


if __name__ == "__main__":
    unittest.main()
