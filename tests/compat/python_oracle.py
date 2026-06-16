#!/usr/bin/env python3
"""Compatibility oracle for the legacy Python implementation.

This script is test-only. It keeps the Rust rewrite honest while the Python
implementation remains in the repository, but it is not part of the Rust
runtime.
"""

import argparse
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

import tabview.tabview as tabview  # noqa: E402


def read_lines(path):
    with open(ROOT / path, "rb") as handle:
        return handle.readlines()


def process_data(args):
    rows = tabview.process_data(
        read_lines(args.path),
        enc=args.encoding,
        delim=args.delimiter,
        quoting=args.quoting,
        quote_char=args.quote_char,
    )
    print(json.dumps({"rows": rows}, ensure_ascii=False, sort_keys=True))


def detect_encoding(args):
    encoding = tabview.detect_encoding(read_lines(args.path))
    print(json.dumps({"encoding": encoding}, sort_keys=True))


def header_classification(args):
    rows = tabview.process_data(read_lines(args.path), enc=args.encoding)
    header = [str(cell) for cell in rows[0]]
    is_header = len(rows) > 1 and not any(is_num(cell) for cell in header)
    print(
        json.dumps(
            {"header": header, "is_header": is_header},
            ensure_ascii=False,
            sort_keys=True,
        )
    )


def is_num(cell):
    try:
        float(cell)
        return True
    except ValueError:
        return False


def main():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(required=True)

    process_parser = subparsers.add_parser("process-data")
    process_parser.add_argument("path")
    process_parser.add_argument("--encoding")
    process_parser.add_argument("--delimiter")
    process_parser.add_argument("--quoting")
    process_parser.add_argument("--quote-char", default='"')
    process_parser.set_defaults(func=process_data)

    encoding_parser = subparsers.add_parser("detect-encoding")
    encoding_parser.add_argument("path")
    encoding_parser.set_defaults(func=detect_encoding)

    header_parser = subparsers.add_parser("header-classification")
    header_parser.add_argument("path")
    header_parser.add_argument("--encoding")
    header_parser.set_defaults(func=header_classification)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
