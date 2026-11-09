#!/usr/bin/env python3
"""Test-only snapshot interoperability; never imported by either library."""

import argparse
import contextlib
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
import subprocess
import tempfile


PAGE_SIZES = (512, 1024, 4096, 8192, 65536)
SNAPSHOT = Path("ltx/0/0000000000000001-0000000000000001.ltx")


def readonly(path):
    # mode=ro deliberately fails on a missing image instead of creating one.
    return sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)


def hash_value(digest, value):
    if value is None:
        tag, content = b"n", b""
    elif isinstance(value, int):
        tag, content = b"i", str(value).encode("ascii")
    elif isinstance(value, float):
        tag, content = b"f", struct.pack(">d", value)
    elif isinstance(value, str):
        tag, content = b"s", value.encode("utf-8")
    elif isinstance(value, bytes):
        tag, content = b"b", value
    else:
        raise TypeError(f"unsupported SQLite value: {type(value).__name__}")
    digest.update(tag)
    digest.update(struct.pack(">Q", len(content)))
    digest.update(content)


def fingerprint(path):
    """Hash every value and schema entry, without materializing the database."""
    digest = hashlib.sha256()
    with contextlib.closing(readonly(path)) as connection:
        for check in ("quick_check", "integrity_check"):
            if connection.execute(f"PRAGMA {check}").fetchall() != [("ok",)]:
                raise AssertionError(f"{check} failed: {path.name}")
        if connection.execute("PRAGMA foreign_key_check").fetchone() is not None:
            raise AssertionError(f"foreign_key_check failed: {path.name}")
        for pragma in ("page_size", "encoding", "application_id", "user_version"):
            hash_value(digest, pragma)
            hash_value(digest, connection.execute(f"PRAGMA {pragma}").fetchone()[0])
        for row in connection.execute(
            "SELECT type, name, tbl_name, sql FROM sqlite_schema "
            "WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name"
        ):
            for value in row:
                hash_value(digest, value)
        tables = connection.execute(
            "SELECT name FROM sqlite_schema WHERE type='table' "
            "AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        for (table,) in tables:
            hash_value(digest, table)
            quoted = '"' + table.replace('"', '""') + '"'
            # These fixtures all have an explicit integer primary key named id.
            for row in connection.execute(f"SELECT * FROM {quoted} ORDER BY id"):
                digest.update(b"r")
                for value in row:
                    hash_value(digest, value)
            digest.update(b"e")
    return digest.hexdigest()


def insert_rows(connection, count):
    connection.executemany(
        "INSERT INTO records VALUES (?, ?, ?, ?, ?)",
        (
            (
                identifier,
                f"row-{identifier:07d}-á\x00終",
                bytes((identifier % 256, (identifier // 256) % 256)),
                None if identifier % 3 == 0 else -identifier,
                identifier / 8.0,
            )
            for identifier in range(1, count + 1)
        ),
    )


def create_fixture(path, page_size, profile):
    with contextlib.closing(sqlite3.connect(path)) as connection:
        connection.execute(f"PRAGMA page_size={page_size}")
        if connection.execute("PRAGMA journal_mode=WAL").fetchone() != ("wal",):
            raise AssertionError("fixture did not enter WAL mode")
        connection.execute("PRAGMA application_id=1397838418")
        connection.execute("PRAGMA user_version=7")
        if profile != "empty":
            connection.execute(
                "CREATE TABLE records (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, "
                "content BLOB NOT NULL, optional_value INTEGER, ratio REAL NOT NULL)"
            )
            connection.execute("CREATE INDEX records_payload ON records(payload)")
            count = {"zero-rows": 0, "one-row": 1, "100k": 100_000, "1m": 1_000_000,
                     "blobs": 2, "rollback": 128, "vacuum-reuse": 4096}[profile]
            insert_rows(connection, count)
            connection.commit()
            if profile == "blobs":
                for identifier, size in ((1, 1024 * 1024), (2, 16 * 1024 * 1024)):
                    # Includes all byte values, not only compressible zeroblob().
                    content = bytes(range(256)) * (size // 256)
                    connection.execute(
                        "UPDATE records SET content=? WHERE id=?", (content, identifier)
                    )
                connection.commit()
            elif profile == "rollback":
                connection.execute("UPDATE records SET payload='must-not-survive'")
                connection.execute("DELETE FROM records WHERE id % 2 = 0")
                connection.execute("CREATE TABLE rolled_back (id INTEGER PRIMARY KEY)")
                connection.rollback()
                if connection.execute("SELECT count(*) FROM records").fetchone() != (128,):
                    raise AssertionError("rollback fixture lost rows")
            elif profile == "vacuum-reuse":
                before = connection.execute("PRAGMA page_count").fetchone()[0]
                connection.execute("DELETE FROM records WHERE id > 32")
                connection.commit()
                connection.execute("VACUUM")
                after = connection.execute("PRAGMA page_count").fetchone()[0]
                if after >= before:
                    raise AssertionError("VACUUM fixture did not shrink")
                connection.executemany(
                    "INSERT INTO records VALUES (?, 'reused', X'00FF', NULL, -0.5)",
                    ((identifier,) for identifier in range(33, 257)),
                )
                connection.commit()
        busy, log_frames, checkpointed = connection.execute(
            "PRAGMA wal_checkpoint(TRUNCATE)"
        ).fetchone()
        if busy != 0 or log_frames != checkpointed:
            raise AssertionError("fixture checkpoint incomplete")
    # Only the fixture generator uses Python's SQLite runtime. All snapshots and
    # library restores use the real Go/Rust adapters and their version gates.


def run(command, *arguments):
    subprocess.run([str(command), *map(str, arguments)], check=True, timeout=300)


def verify_restore(path, expected):
    if fingerprint(path) != expected:
        raise AssertionError(f"restored schema/content differs: {path.name}")


def exercise_invalid_metadata(directory, go, rust, repository):
    manifest_path = repository / "_meta/repository.json"
    original = manifest_path.read_bytes()
    manifest = json.loads(original)
    invalid_documents = []
    for field in ("repository_id", "database_id"):
        for identity in (
            "00000000-0000-0000-0000-000000000000",
            "550e8400-e29b-11d4-a716-446655440000",
            "550e8400-e29b-41d4-0716-446655440000",
            "550e8400e29b41d4a716446655440000",
            "urn:uuid:550e8400-e29b-41d4-a716-446655440000",
            "{550e8400-e29b-41d4-a716-446655440000}",
        ):
            invalid_documents.append(json.dumps({**manifest, field: identity}).encode())
    for change in ({"schema_version": 2}, {"format_version": 3}, {"unknown": True}):
        invalid_documents.append(json.dumps({**manifest, **change}).encode())
    invalid_documents.append(original + b"{}")
    try:
        for index, content in enumerate(invalid_documents):
            manifest_path.write_bytes(content)
            for name, consumer in (("go", go), ("rust", rust)):
                destination = directory / f"invalid-{index}-{name}.sqlite"
                result = subprocess.run(
                    [str(consumer), "restore", str(repository), str(destination)],
                    capture_output=True, check=False, timeout=30,
                )
                if result.returncode == 0 or destination.exists():
                    raise AssertionError(f"{name} accepted invalid metadata case {index}")
                if manifest_path.read_bytes() != content:
                    raise AssertionError(f"{name} modified invalid metadata case {index}")
    finally:
        manifest_path.write_bytes(original)
    # Uppercase is valid and must not be rejected as a side effect of validation.
    uppercase = {**manifest, "repository_id": manifest["repository_id"].upper(),
                 "database_id": manifest["database_id"].upper()}
    try:
        manifest_path.write_text(json.dumps(uppercase), encoding="utf-8")
        expected = fingerprint(directory / "source.sqlite")
        for name, consumer in (("go", go), ("rust", rust)):
            destination = directory / f"uppercase-{name}.sqlite"
            run(consumer, "restore", repository, destination)
            verify_restore(destination, expected)
    finally:
        manifest_path.write_bytes(original)
    print("metadata matrix: 32 invalid restores rejected, 2 uppercase restores passed", flush=True)


def exercise_case(directory, go, rust, reference, page_size, profile):
    directory.mkdir()
    source = directory / "source.sqlite"
    create_fixture(source, page_size, profile)
    expected = fingerprint(source)
    for producer_name, producer, consumer in (("go", go, rust), ("rust", rust, go)):
        repository = directory / f"{producer_name}-repository"
        run(producer, "snapshot", source, repository)
        restored = directory / f"{producer_name}-cross.sqlite"
        run(consumer, "restore", repository, restored)
        verify_restore(restored, expected)
        restored_reference = directory / f"{producer_name}-reference.sqlite"
        run(reference, "verify", repository / SNAPSHOT, restored_reference)
        verify_restore(restored_reference, expected)
    reference_ltx = directory / "reference.ltx"
    run(reference, "create", source, reference_ltx)
    for name, consumer in (("go", go), ("rust", rust)):
        restored = directory / f"reference-{name}.sqlite"
        run(consumer, "decode-ltx", reference_ltx, restored)
        verify_restore(restored, expected)
    # Snapshot/restore must not change logical state in the source either.
    verify_restore(source, expected)
    if profile == "one-row":
        exercise_invalid_metadata(directory, go, rust, directory / "go-repository")
    print(f"snapshot matrix: {profile}, page_size={page_size}: 6 restores passed", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--go", required=True, type=Path)
    parser.add_argument("--rust", required=True, type=Path)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--temporary-root", required=True, type=Path)
    arguments = parser.parse_args()
    cases = [(page_size, profile) for page_size in PAGE_SIZES
             for profile in ("empty", "zero-rows")]
    for profile in ("one-row", "100k", "1m", "blobs", "rollback", "vacuum-reuse"):
        cases.append((4096, profile))
    for page_size, profile in cases:
        with tempfile.TemporaryDirectory(prefix="snapshot-matrix-", dir=arguments.temporary_root) as root:
            exercise_case(Path(root) / f"{profile}-{page_size}", arguments.go.resolve(),
                          arguments.rust.resolve(), arguments.reference.resolve(), page_size, profile)
    print(f"snapshot matrix passed: {len(cases)} cases, {len(cases) * 6} restores")


if __name__ == "__main__":
    main()
