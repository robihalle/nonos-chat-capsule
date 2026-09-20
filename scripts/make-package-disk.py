#!/usr/bin/env python3
# NONOS Operating System
# Copyright (C) 2026 NONOS Contributors
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Create a NEW VM transfer disk containing one downloaded .nonos package.

Uses the upstream NONOSTR1 store layout at LBA 256. This is a transport
container, not a trust grant. The guest installer must verify the package.
"""
import argparse
from pathlib import Path
import struct

BASE = 256 * 512
DISK_SIZE = 128 * 1024 * 1024
MAX_PACKAGE = 16 * 1024 * 1024
# Reserve all 64 native TOC slots so subsequent installs cannot overwrite the package.
TOC_SIZE = ((32 + 128 * 64 + 511) // 512) * 512


def stage(package, output):
    if package.suffix != ".nonos":
        raise ValueError("Expected a .nonos file.")
    if not package.is_file() or not 8 <= package.stat().st_size <= MAX_PACKAGE:
        raise ValueError("Package is missing, empty or exceeds 16 MiB.")
    if any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-" for c in package.name):
        raise ValueError("Use an ASCII package filename containing only letters, digits, dots, _ and -.")
    name = ("/pkgs/" + package.name).encode("ascii")
    if len(name) >= 96:
        raise ValueError("Package filename is too long.")
    body = package.read_bytes()
    if body[:4] != b"NOS1":
        raise ValueError("Not an upstream NOS1 package.")
    offset = BASE + TOC_SIZE
    header = b"NONOSTR1" + struct.pack("<II", 1, 1) + bytes(16)
    entry = name.ljust(96, b"\0") + struct.pack("<QQ", offset, len(body)) + bytes(16)
    # Exclusive create prevents an existing disk, file or symlink from being overwritten.
    with output.open("xb") as disk:
        disk.truncate(DISK_SIZE)
        disk.seek(BASE)
        disk.write(header + entry)
        disk.seek(offset)
        disk.write(body)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--package", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    args = ap.parse_args()
    try:
        stage(args.package, args.out)
    except (OSError, ValueError) as e:
        raise SystemExit(str(e))
    print("Created " + str(args.out) + ". Attach as the guest's first VirtIO block data disk.")
    print("Boot media must be separate (for example SATA/IDE). Verify and install inside NONOS.")


if __name__ == "__main__":
    main()
