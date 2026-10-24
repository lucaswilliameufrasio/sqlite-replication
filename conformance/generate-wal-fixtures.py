#!/usr/bin/env python3
"""Generate tiny deterministic WAL parser vectors from the published format."""

import hashlib
import pathlib
import struct


ROOT = pathlib.Path(__file__).resolve().parent
FIXTURE_DIRECTORY = ROOT / "fixtures" / "wal"
PAGE_SIZE = 512
SALT_1 = 0x11223344
SALT_2 = 0xAABBCCDD


def checksum(data, state, word_order):
    checksum_1, checksum_2 = state
    for offset in range(0, len(data), 8):
        first, second = struct.unpack(word_order + "II", data[offset : offset + 8])
        checksum_1 = (checksum_1 + first + checksum_2) & 0xFFFFFFFF
        checksum_2 = (checksum_2 + second + checksum_1) & 0xFFFFFFFF
    return checksum_1, checksum_2


def build_wal(magic, word_order, page_size=PAGE_SIZE):
    header_prefix = struct.pack(
        ">6I",
        magic,
        3007000,
        page_size,
        7,
        SALT_1,
        SALT_2,
    )
    state = checksum(header_prefix, (0, 0), word_order)
    wal_bytes = bytearray(header_prefix + struct.pack(">2I", *state))

    frames = (
        (1, 0, bytes(index & 0xFF for index in range(page_size))),
        (2, 2, bytes((index * 7 + 3) & 0xFF for index in range(page_size))),
        (1, 0, bytes((255 - index) & 0xFF for index in range(page_size))),
    )
    for page_number, database_size, page in frames:
        frame_prefix = struct.pack(">2I", page_number, database_size)
        state = checksum(frame_prefix + page, state, word_order)
        frame_header = frame_prefix + struct.pack(
            ">4I", SALT_1, SALT_2, state[0], state[1]
        )
        wal_bytes.extend(frame_header)
        wal_bytes.extend(page)

    return bytes(wal_bytes)


def main():
    FIXTURE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    fixtures = (
        ("sqlite-le.wal", 0x377F0682, "<"),
        ("sqlite-be.wal", 0x377F0683, ">"),
        ("sqlite-le-65536.wal", 0x377F0682, "<", 65_536),
    )
    for fixture in fixtures:
        name, magic, word_order = fixture[:3]
        page_size = fixture[3] if len(fixture) == 4 else PAGE_SIZE
        path = FIXTURE_DIRECTORY / name
        content = build_wal(magic, word_order, page_size)
        if path.exists():
            if path.read_bytes() != content:
                raise FileExistsError(f"refusing to replace different WAL fixture: {path}")
            print(f"{hashlib.sha256(content).hexdigest()}  {path.relative_to(ROOT)} (verified)")
            continue
        with path.open("xb") as fixture_file:
            fixture_file.write(content)
        print(f"{hashlib.sha256(content).hexdigest()}  {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
