"""Profiling orchestration checks; no sudo or perf execution."""

from contextlib import redirect_stderr, redirect_stdout
import io
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import profile_native as profile


class ProcessIdentity(unittest.TestCase):
    def test_live_executable_and_start_identity_reject_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'sleep'
            shutil.copy2(shutil.which('sleep'), binary)
            process = subprocess.Popen([str(binary), '60'])
            try:
                target = profile.Target(process.pid, binary)
                target.verify()
                with patch.object(profile, 'process_identity',
                                  return_value=('another-start', str(binary))):
                    with self.assertRaisesRegex(profile.Refused, 'identity changed'):
                        target.verify()
                replacement = Path(directory) / 'replacement'
                shutil.copy2(shutil.which('sleep'), replacement)
                os.replace(replacement, binary)
                with self.assertRaisesRegex(profile.Refused, 'selected executable'):
                    target.verify()
            finally:
                process.terminate()
                process.wait(timeout=5)

    def test_wrong_executable_is_refused(self):
        with self.assertRaisesRegex(profile.Refused, 'selected executable'):
            profile.Target(os.getpid(), Path(shutil.which('sleep')))


class CaptureCommands(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name) / 'capture'
        self.argv = ['--pid', '12345', '--executable', '/example/arkham-tabletop',
                     '--output-dir', str(self.output), '--seconds', '10', '--countdown', '2']
        self.stdout, self.stderr = io.StringIO(), io.StringIO()
        self.target_type = self.enterContext(patch.object(profile, 'Target'))
        self.target = self.target_type.return_value
        self.target.executable = Path('/example/arkham-tabletop')
        self.enterContext(patch.object(profile.shutil, 'which', side_effect=lambda name: '/bin/' + name))
        self.pause = self.enterContext(patch.object(profile.time, 'sleep'))
        self.run = self.enterContext(patch.object(profile.subprocess, 'run', side_effect=self.success))

    def success(self, command, **kwargs):
        if '-v' not in command:
            self.output.joinpath('perf.data').write_bytes(b'captured samples')
            kwargs['stdout'].write('perf statistics\n')
        return subprocess.CompletedProcess(command, 0)

    def invoke(self):
        with redirect_stdout(self.stdout), redirect_stderr(self.stderr):
            return profile.main(self.argv)

    def test_interaction_precedes_auth_and_countdown_and_capture_is_bounded(self):
        events = []

        def run(command, **kwargs):
            if '-v' in command:
                self.assertIn('right mouse button', self.stdout.getvalue())
                self.assertIn('mouse wheel', self.stdout.getvalue())
                self.assertIn('10 seconds', self.stdout.getvalue())
                events.append('authenticate')
            else:
                events.append('record')
                self.assertIn('RECORDING', self.stdout.getvalue())
                self.assertEqual(command, [
                    '/bin/sudo', '-n', '/bin/timeout', '--signal=INT', '--kill-after=3',
                    '15', '/bin/perf', 'record', '-F', '99', '-g', '-p', '12345',
                    '-o', str(self.output / 'perf.data'), '--', '/bin/sleep', '10'])
                self.assertEqual(kwargs['timeout'], 22)
            return self.success(command, **kwargs)

        self.run.side_effect = run
        self.pause.side_effect = lambda _: events.append('countdown')
        self.target.verify.side_effect = lambda: events.append('verify')
        self.assertEqual(self.invoke(), 0)
        self.assertEqual(events, ['authenticate', 'countdown', 'countdown', 'verify',
                                  'record', 'verify'])
        self.assertIn('Capture complete:', self.stdout.getvalue())
        self.assertIn('(16 bytes)', self.stdout.getvalue())
        self.assertEqual(self.output.joinpath('perf.log').read_text(), 'perf statistics\n')
        self.assertEqual(self.output.stat().st_mode & 0o777, 0o700)

    def test_existing_directory_and_capture_are_preserved_without_sudo(self):
        self.output.mkdir()
        data = self.output / 'perf.data'
        data.write_bytes(b'previous evidence')
        self.assertEqual(self.invoke(), 1)
        self.assertEqual(data.read_bytes(), b'previous evidence')
        self.run.assert_not_called()
        self.assertIn('fresh path', self.stderr.getvalue())

    def test_initial_target_refusal_precedes_output_creation_and_sudo(self):
        self.target_type.side_effect = profile.Refused('wrong executable')
        self.assertEqual(self.invoke(), 1)
        self.assertFalse(self.output.exists())
        self.run.assert_not_called()

    def test_target_change_during_countdown_prevents_recording(self):
        self.target.verify.side_effect = profile.Refused('identity changed')
        self.assertEqual(self.invoke(), 1)
        self.assertEqual(self.run.call_count, 1)
        self.assertFalse((self.output / 'perf.data').exists())
        self.assertNotIn('Capture complete:', self.stdout.getvalue())

    def test_failed_authentication_never_starts_countdown_or_capture(self):
        self.run.side_effect = subprocess.CalledProcessError(1, ['sudo', '-v'])
        self.assertEqual(self.invoke(), 1)
        self.pause.assert_not_called()
        self.assertEqual(self.run.call_count, 1)

    def test_perf_failure_preserves_partial_data_and_log(self):
        def failure(command, **kwargs):
            result = self.success(command, **kwargs)
            result.returncode = 0 if '-v' in command else 124
            return result

        self.run.side_effect = failure
        self.assertEqual(self.invoke(), 1)
        self.assertTrue((self.output / 'perf.data').exists())
        self.assertTrue((self.output / 'perf.log').exists())
        self.assertIn('status 124', self.stderr.getvalue())
        self.assertNotIn('Capture complete:', self.stdout.getvalue())

    def test_interrupt_or_timeout_never_claims_completion(self):
        for error, expected in [(KeyboardInterrupt(), 130),
                                (subprocess.TimeoutExpired('perf', 22), 1)]:
            with self.subTest(error=type(error).__name__):
                self.output = Path(self.temporary.name) / type(error).__name__
                self.argv[self.argv.index('--output-dir') + 1] = str(self.output)
                self.run.side_effect = [subprocess.CompletedProcess(['sudo'], 0), error]
                self.assertEqual(self.invoke(), expected)
                self.assertNotIn('Capture complete:', self.stdout.getvalue())

    def test_app_exit_after_capture_retains_data_and_reports_failure(self):
        self.target.verify.side_effect = [None, profile.Refused('App process is absent')]
        self.assertEqual(self.invoke(), 1)
        self.assertTrue((self.output / 'perf.data').exists())
        self.assertNotIn('Capture complete:', self.stdout.getvalue())

    def test_missing_or_empty_capture_is_not_success(self):
        for content in [None, b'']:
            with self.subTest(content=content):
                self.output = Path(self.temporary.name) / ('missing' if content is None else 'empty')
                self.argv[self.argv.index('--output-dir') + 1] = str(self.output)

                def run(command, **_kwargs):
                    if '-v' not in command and content is not None:
                        (self.output / 'perf.data').write_bytes(content)
                    return subprocess.CompletedProcess(command, 0)

                self.run.side_effect = run
                self.assertEqual(self.invoke(), 1)
                self.assertNotIn('Capture complete:', self.stdout.getvalue())

    def test_cli_limits_refuse_unbounded_recording(self):
        for flag, value in [('--seconds', '0'), ('--seconds', '61'),
                            ('--countdown', '0'), ('--countdown', '11'), ('--pid', '0')]:
            with self.subTest(flag=flag, value=value), redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    profile.arguments(self.argv + [flag, value])
                self.assertEqual(error.exception.code, 2)
        self.run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
