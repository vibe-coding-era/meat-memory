#!/usr/bin/env python3
import re
import sys
from pathlib import Path

THRESHOLDS = {
    "main.rs": {"region": 95.0, "function": 95.0, "line": 95.0},
    "lifecycle_impl.rs": {"region": 95.0, "function": 95.0, "line": 95.0},
}


def parse_percent(value: str) -> float:
    return float(value.rstrip("%"))


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: v2_7-coverage-threshold.py <llvm-cov-report.txt>", file=sys.stderr)
        return 2

    report_path = Path(sys.argv[1])
    report = report_path.read_text()
    rows = {}
    pattern = re.compile(
        r"^(?P<file>\S+)\s+"
        r"\d+\s+\d+\s+(?P<region>\d+\.\d+%)\s+"
        r"\d+\s+\d+\s+(?P<function>\d+\.\d+%)\s+"
        r"\d+\s+\d+\s+(?P<line>\d+\.\d+%)",
        re.MULTILINE,
    )
    for match in pattern.finditer(report):
        rows[Path(match.group("file")).name] = {
            "region": parse_percent(match.group("region")),
            "function": parse_percent(match.group("function")),
            "line": parse_percent(match.group("line")),
        }

    failed = False
    for filename, thresholds in THRESHOLDS.items():
        if filename not in rows:
            print(f"[coverage] missing required file row: {filename}", file=sys.stderr)
            failed = True
            continue
        for metric, minimum in thresholds.items():
            actual = rows[filename][metric]
            if actual < minimum:
                print(
                    f"[coverage] {filename} {metric} {actual:.2f}% < {minimum:.2f}%",
                    file=sys.stderr,
                )
                failed = True

    if failed:
        return 1

    print("[coverage] thresholds passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
