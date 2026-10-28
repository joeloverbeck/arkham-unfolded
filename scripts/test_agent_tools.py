"""Exercise the read and tracked-file commands through their actual CLI paths."""

import hashlib
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parent


def run_script(name, *args, cwd=None):
    return subprocess.run(
        [sys.executable, "-B", str(SCRIPTS / name), *map(str, args)],
        cwd=cwd,
        capture_output=True,
        check=False,
    )


class ContextPages(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "document.txt"

    def test_continuations_cover_selected_lines_once_with_bounded_unicode_output(self):
        lines = [f"  source {n}: café 漢字 — {'x' * (n % 37)}\n" for n in range(1, 81)]
        source = "".join(lines).encode("utf-8")
        self.path.write_bytes(source)
        args = ["--start", "3", "--end", "79", "--max-bytes", "512"]
        observed = []
        pages = 0
        while True:
            result = run_script("read_context.py", self.path, *args)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLessEqual(len(result.stdout), 512)
            text = result.stdout.decode("utf-8")
            self.assertIn(hashlib.sha256(source).hexdigest(), text)
            rows = re.findall(r"^ *(\d+) \| (.*)\n", text, re.MULTILINE)
            self.assertTrue(rows)
            observed.extend((int(number), content + "\n") for number, content in rows)
            pages += 1
            self.assertLessEqual(pages, len(lines))
            continuation = re.search(r"^NEXT (.+)$", text, re.MULTILINE)
            if not continuation:
                self.assertTrue(text.endswith("\nEND\n"))
                break
            args = shlex.split(continuation.group(1))
        self.assertGreater(pages, 1)
        self.assertEqual(observed, list(enumerate(lines, 1))[2:79])
        self.assertEqual(self.path.read_bytes(), source)

    def test_changed_source_refuses_continuation_without_printing_new_content(self):
        original = b"original\n" * 100
        self.path.write_bytes(original)
        first = run_script("read_context.py", self.path, "--max-bytes", "512")
        continuation = re.search(rb"^NEXT (.+)$", first.stdout, re.MULTILINE)
        self.assertIsNotNone(continuation)
        self.path.write_bytes(b"changed\n" * 100)
        result = run_script("read_context.py", self.path, *shlex.split(continuation[1].decode()))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"source changed", result.stderr)

    def test_oversized_line_and_invalid_ranges_refuse_without_partial_output(self):
        self.path.write_bytes(b"x" * 13000 + b"\n")
        result = run_script("read_context.py", self.path, "--max-bytes", "12000")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"line 1 cannot fit", result.stderr)
        self.path.write_bytes(b"one\ntwo\n")
        for args in [("--start", "0"), ("--end", "3"), ("--start", "2", "--end", "1"),
                     ("--max-bytes", "511"), ("--max-bytes", "12001")]:
            with self.subTest(args=args):
                result = run_script("read_context.py", self.path, *args)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, b"")

    def test_empty_file_crlf_and_missing_final_newline_are_explicit(self):
        self.path.write_bytes(b"")
        empty = run_script("read_context.py", self.path)
        self.assertEqual(empty.returncode, 0, empty.stderr)
        self.assertIn(b"EMPTY FILE\nEND\n", empty.stdout)
        self.path.write_bytes(b"first\r\nlast without newline")
        result = run_script("read_context.py", self.path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(b"     1 | first\r\n", result.stdout)
        self.assertIn(b"     2 | last without newline\nEND (source has no final newline)\n", result.stdout)

    def test_missing_and_non_utf8_files_emit_no_source(self):
        missing = run_script("read_context.py", self.path)
        self.assertEqual(missing.returncode, 2)
        self.assertEqual(missing.stdout, b"")
        self.path.write_bytes(b"before\n\xff\nafter\n")
        invalid = run_script("read_context.py", self.path)
        self.assertEqual(invalid.returncode, 2)
        self.assertEqual(invalid.stdout, b"")


class TrackedFiles(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "--quiet")

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.root, capture_output=True, check=True)

    def write(self, path, data=b"fixture\n"):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)

    def test_untracked_caches_and_tracked_python_sources_are_allowed(self):
        self.write("scripts/__pycache__/native_verify.cpython-311.pyc")
        self.write(".pytest_cache/v/cache/nodeids")
        self.write("scripts/native_verify.py")
        self.write("scripts/test_native_verify.py")
        self.git("add", "scripts/native_verify.py", "scripts/test_native_verify.py")
        result = run_script("check_tracked_files.py", cwd=self.root)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_force_added_generated_files_fail_even_when_missing_from_worktree(self):
        self.write(".gitignore", b"__pycache__/\n*.py[cod]\n.pytest_cache/\n")
        paths = ["scripts/__pycache__/native_verify.cpython-311.pyc", "cache.pyo", "cache.pyd",
                 ".pytest_cache/v/cache/nodeids", "scripts/__pycache__/odd\nname.txt"]
        for path in paths:
            self.write(path)
            self.git("add", "--force", "--", path)
            (self.root / path).unlink()
        (self.root / "child").mkdir()
        result = run_script("check_tracked_files.py", cwd=self.root / "child")
        self.assertEqual(result.returncode, 1)
        for path in paths:
            self.assertIn(repr(path).encode(), result.stderr)
        self.assertEqual(result.stdout, b"")

    def test_git_failure_is_not_reported_as_a_clean_tree(self):
        with tempfile.TemporaryDirectory() as outside:
            result = run_script("check_tracked_files.py", cwd=outside)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b"Could not inspect tracked files", result.stderr)

    def test_repository_gate_rejects_caches_before_running_tests(self):
        for name in ("check", "check_tracked_files.py"):
            self.write("scripts/" + name, (SCRIPTS / name).read_bytes())
        self.write("scripts/test_unexpected.py", b'raise AssertionError("UNEXPECTED_TEST_EXECUTION")\n')
        self.write("scripts/__pycache__/generated.pyc")
        self.git("add", "--force", "scripts/__pycache__/generated.pyc")
        result = subprocess.run(
            ["bash", str(self.root / "scripts/check")],
            cwd=self.root,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn(b"Generated Python files are tracked", result.stderr)
        self.assertNotIn(b"UNEXPECTED_TEST_EXECUTION", result.stderr)


if __name__ == "__main__":
    unittest.main()
