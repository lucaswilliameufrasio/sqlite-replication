"""Tests for the test oracle itself; the integration matrix uses real engines."""

import contextlib
from pathlib import Path
import sqlite3
import tempfile
import unittest

from snapshot_matrix import create_fixture, fingerprint, verify_restore


class SnapshotOracleTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "fixture.sqlite"

    def test_equal_counts_and_lengths_do_not_hide_changed_content(self):
        create_fixture(self.path, 4096, "one-row")
        expected = fingerprint(self.path)
        with contextlib.closing(sqlite3.connect(self.path)) as connection:
            connection.execute("UPDATE records SET content=X'0001'")
            connection.commit()
        with self.assertRaisesRegex(AssertionError, "schema/content differs"):
            verify_restore(self.path, expected)

    def test_missing_index_is_not_a_valid_restore(self):
        create_fixture(self.path, 4096, "one-row")
        expected = fingerprint(self.path)
        with contextlib.closing(sqlite3.connect(self.path)) as connection:
            connection.execute("DROP INDEX records_payload")
            connection.commit()
        self.assertNotEqual(fingerprint(self.path), expected)

    def test_null_and_empty_blob_and_text_are_distinct(self):
        create_fixture(self.path, 4096, "one-row")
        fingerprints = set()
        for value in (None, b"", "", 0, 0.5):
            with contextlib.closing(sqlite3.connect(self.path)) as connection:
                connection.execute("UPDATE records SET optional_value=?", (value,))
                connection.commit()
            fingerprints.add(fingerprint(self.path))
        self.assertEqual(len(fingerprints), 5)

    def test_empty_schema_and_zero_rows_are_different(self):
        create_fixture(self.path, 4096, "empty")
        zero_rows = Path(self.directory.name) / "zero-rows.sqlite"
        create_fixture(zero_rows, 4096, "zero-rows")
        self.assertNotEqual(fingerprint(self.path), fingerprint(zero_rows))

    def test_missing_database_is_not_created(self):
        with self.assertRaises(sqlite3.OperationalError):
            fingerprint(self.path)
        self.assertFalse(self.path.exists())

    def test_rollback_and_vacuum_fixtures(self):
        for profile, expected_count in (("rollback", 128), ("vacuum-reuse", 256)):
            path = Path(self.directory.name) / f"{profile}.sqlite"
            create_fixture(path, 4096, profile)
            with contextlib.closing(sqlite3.connect(path)) as connection:
                self.assertEqual(connection.execute("SELECT count(*) FROM records").fetchone(),
                                 (expected_count,))
                self.assertEqual(connection.execute(
                    "SELECT count(*) FROM sqlite_schema WHERE name='rolled_back'"
                ).fetchone(), (0,))
                self.assertEqual(connection.execute(
                    "SELECT count(*) FROM records WHERE payload='must-not-survive'"
                ).fetchone(), (0,))
            fingerprint(path)


if __name__ == "__main__":
    unittest.main()
