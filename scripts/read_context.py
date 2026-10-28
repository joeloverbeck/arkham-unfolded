"""Read one bounded page of UTF-8 text without losing source lines."""

import argparse
import hashlib
from pathlib import Path
import sys


def page(data, start, end, max_bytes, expected_hash):
    if not 512 <= max_bytes <= 12000:
        raise ValueError("--max-bytes must be between 512 and 12000")
    digest = hashlib.sha256(data).hexdigest()
    if expected_hash is not None and digest != expected_hash:
        raise ValueError("source changed; restart the read from the intended revision")
    # Count LF-delimited lines like rg -n, preserving CRLF and the last newline.
    parts = data.decode("utf-8").split("\n")
    lines = [part + "\n" for part in parts[:-1]]
    if parts[-1]:
        lines.append(parts[-1])
    header = f"SHA256 {digest}\n"
    if not lines and start == 1 and end is None:
        return header + "EMPTY FILE\nEND\n"
    end = len(lines) if end is None else end
    if not 1 <= start <= end <= len(lines):
        raise ValueError(f"select an inclusive range within 1..{len(lines)}")
    header += f"SELECTED {start}..{end} of {len(lines)} lines\n"
    # Reserve enough space for the longest possible continuation footer.
    footer_size = len(
        f"\nNEXT --start {end + 1} --end {end} --max-bytes {max_bytes} --sha256 {digest}\n".encode("utf-8")
    )
    remaining = max_bytes - len(header.encode("utf-8")) - footer_size
    rows = []
    next_line = start
    for number in range(start, end + 1):
        row = f"{number:6} | {lines[number - 1]}"
        size = len(row.encode("utf-8"))
        if size > remaining:
            break
        rows.append(row)
        remaining -= size
        next_line = number + 1
    if not rows:
        raise ValueError(
            f"line {start} cannot fit this page; raise --max-bytes (up to 12000) "
            "or inspect that line separately; no source text was emitted"
        )
    if next_line <= end:
        footer = f"NEXT --start {next_line} --end {end} --max-bytes {max_bytes} --sha256 {digest}\n"
    else:
        footer = "END\n" if lines[end - 1].endswith("\n") else "END (source has no final newline)\n"
    return header + "".join(rows) + "\n" + footer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path, help="UTF-8 file to read; never modified")
    parser.add_argument("--start", type=int, default=1, help="first source line (1-based)")
    parser.add_argument("--end", type=int, help="last source line, inclusive (default: EOF)")
    parser.add_argument("--max-bytes", type=int, default=6000, help="entire page limit, 512..12000")
    parser.add_argument("--sha256", help="expected source hash from the preceding page")
    args = parser.parse_args()
    try:
        output = page(args.path.read_bytes(), args.start, args.end, args.max_bytes, args.sha256)
    except (OSError, UnicodeError, ValueError) as error:
        parser.exit(2, f"{parser.prog}: {error}\n")
    sys.stdout.buffer.write(output.encode("utf-8"))


if __name__ == "__main__":
    main()
