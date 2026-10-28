#!/usr/bin/env python3
"""Owner-run, bounded CPU capture for an already running Arkham client."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import time

from native_verify import Refused, process_identity


class Target:
    def __init__(self, pid, executable):
        self.pid = pid
        self.executable = executable.resolve(strict=True)
        self.identity = self.read_identity()

    def read_identity(self):
        identity = process_identity(self.pid)
        running = (Path('/proc') / str(self.pid) / 'exe').stat()
        expected = self.executable.stat()
        running_file = (running.st_dev, running.st_ino)
        expected_file = (expected.st_dev, expected.st_ino)
        if identity[1] != str(self.executable) or running_file != expected_file:
            raise Refused('PID is not running the selected executable; check the PID and build path')
        return identity, running_file

    def verify(self):
        if self.read_identity() != self.identity:
            raise Refused('App process identity changed; no further capture is authorized')


def bounded_integer(minimum, maximum):
    def parse(value):
        number = int(value)
        if not minimum <= number <= maximum:
            raise argparse.ArgumentTypeError(f'must be between {minimum} and {maximum}')
        return number
    return parse


def arguments(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', required=True, type=bounded_integer(1, 2**31 - 1))
    parser.add_argument('--executable', required=True, type=Path,
                        help='expected executable of the selected app process')
    parser.add_argument('--output-dir', required=True, type=Path,
                        help='new directory for perf.data and perf.log; must not already exist')
    parser.add_argument('--seconds', type=bounded_integer(1, 60), default=10,
                        help='capture duration, 1–60 seconds (default: 10)')
    parser.add_argument('--countdown', type=bounded_integer(1, 10), default=5,
                        help='time to switch to the app, 1–10 seconds (default: 5)')
    return parser.parse_args(argv)


def announce(message):
    print(message, flush=True)


def capture(args):
    target = Target(args.pid, args.executable)
    commands = {}
    for name in ('sudo', 'timeout', 'perf', 'sleep'):
        path = shutil.which(name)
        if path is None:
            raise Refused(f'Required command is not installed: {name}')
        commands[name] = path

    output = args.output_dir.absolute()
    try:
        output.mkdir(mode=0o700)
    except FileExistsError as error:
        raise Refused(f'Output directory already exists; choose a fresh path: {output}') from error

    announce(f'Target: PID {args.pid}, {target.executable}')
    announce(f'Capture: {args.seconds} seconds; files will stay in {output}')
    announce('Interaction: repeatedly tilt/orbit with the right mouse button and zoom '
             'with the mouse wheel throughout the recording.')
    announce(f'Authenticate below. Then, when the {args.countdown}-second countdown starts, '
             'switch to the app and begin those camera movements.')
    announce('Leave this terminal open; return after the recording to read its result.')

    try:
        # Authenticate before the countdown so a password prompt cannot consume it.
        subprocess.run([commands['sudo'], '-v'], check=True)
        for remaining in range(args.countdown, 0, -1):
            announce(f'Switch to the app and move the camera: recording in {remaining}…')
            time.sleep(1)
        target.verify()

        data = output / 'perf.data'
        # The enclosing root-owned timeout bounds perf even if this terminal closes.
        command = [commands['sudo'], '-n', commands['timeout'], '--signal=INT',
                   '--kill-after=3', str(args.seconds + 5), commands['perf'], 'record',
                   '-F', '99', '-g', '-p', str(args.pid), '-o', str(data), '--',
                   commands['sleep'], str(args.seconds)]
        announce(f'RECORDING for {args.seconds} seconds: keep orbiting and zooming.')
        with (output / 'perf.log').open('x') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                                    timeout=args.seconds + 12, check=False)
        if result.returncode != 0:
            raise Refused(f'perf exited with status {result.returncode}; inspect perf.log')
        target.verify()
        size = data.stat().st_size
        if size == 0:
            raise Refused('perf produced an empty capture; inspect perf.log')
        announce(f'Capture complete: {data} ({size:,} bytes). You can stop moving the camera.')
        announce(f'Profiler output: {output / "perf.log"}')
    except BaseException:
        announce(f'Capture did not complete successfully. Any partial files remain in {output}.')
        raise


def main(argv=None):
    args = arguments(argv)
    try:
        capture(args)
    except KeyboardInterrupt:
        print('Profiling interrupted. No complete capture is claimed.', file=sys.stderr)
        return 130
    except (Refused, OSError, subprocess.SubprocessError) as error:
        print(f'Profiling failed: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
