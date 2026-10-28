"""Reject generated Python files in Git's index, including force-added files."""

import os
import subprocess
import sys


def main():
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--full-name", "-z", "--", ":/"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode:
        sys.stderr.write("Could not inspect tracked files.\n")
        sys.stderr.buffer.write(result.stderr)
        return 2
    forbidden = []
    for entry in result.stdout.split(b"\0"):
        path = os.fsdecode(entry)
        if path.endswith((".pyc", ".pyo", ".pyd")) or any(
            part in ("__pycache__", ".pytest_cache") for part in path.split("/")[:-1]
        ):
            forbidden.append(path)
    if forbidden:
        sys.stderr.write("Generated Python files are tracked; keep these outside Git:\n")
        for path in sorted(set(forbidden)):
            sys.stderr.write(f"  {path!r}\n")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
