#!/usr/bin/env python3
"""Generate a pinned-oracle LTX v2 recovery chain without overwriting fixtures."""

import hashlib
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent
REFERENCE = ROOT / "reference"
FIXTURE_ROOT = ROOT / "fixtures" / "ltx-v2"
FIXTURE_DIRECTORY = FIXTURE_ROOT / "recovery-chain-4096"
TIMESTAMP_1 = 1_700_000_000_000
TIMESTAMP_2 = TIMESTAMP_1 + 100
TIMESTAMP_3 = TIMESTAMP_1 + 200


def create_base(path):
    with sqlite3.connect(path) as connection:
        connection.execute("PRAGMA page_size=4096")
        connection.execute("PRAGMA journal_mode=DELETE")
        connection.execute("PRAGMA application_id=1397838418")
        connection.execute("PRAGMA user_version=7")
        connection.execute(
            "CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL)"
        )
        connection.executemany(
            "INSERT INTO records VALUES (?, ?)",
            ((row_id, f"value-{row_id:03d}") for row_id in range(1, 33)),
        )
        connection.commit()
        connection.execute("VACUUM")


def run_reference(*arguments):
    subprocess.run(["go", "run", ".", *map(str, arguments)], cwd=REFERENCE, check=True)


def main():
    if FIXTURE_DIRECTORY.exists():
        raise FileExistsError(f"Refusing to overwrite existing fixtures: {FIXTURE_DIRECTORY}")

    FIXTURE_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ltx-chain-fixture-", dir=FIXTURE_ROOT) as temporary:
        work = Path(temporary)
        base = work / "state-1.sqlite"
        middle = work / "state-2.sqlite"
        final = work / "state-3.sqlite"
        create_base(base)
        shutil.copyfile(base, middle)
        with sqlite3.connect(middle) as connection:
            connection.execute("UPDATE records SET value='updated-middle' WHERE id=7")
            connection.execute("INSERT INTO records VALUES (33, 'inserted-middle')")
            connection.commit()
        shutil.copyfile(middle, final)
        with sqlite3.connect(final) as connection:
            connection.execute("UPDATE records SET value='updated-final' WHERE id=7")
            connection.execute("DELETE FROM records WHERE id=2")
            connection.commit()

        generated = work / "fixtures"
        generated.mkdir()
        base_ltx = generated / "snapshot-1.ltx"
        delta_2 = generated / "delta-2.ltx"
        delta_3 = generated / "delta-3.ltx"
        compacted_2 = generated / "compacted-2.ltx"
        run_reference("create-at", base, base_ltx, "0000000000000001", "0000000000000001", TIMESTAMP_1)
        run_reference("create-delta", base, middle, delta_2, "0000000000000002", TIMESTAMP_2)
        run_reference("create-delta", middle, final, delta_3, "0000000000000003", TIMESTAMP_3)
        run_reference("create-at", middle, compacted_2, "0000000000000001", "0000000000000002", TIMESTAMP_2)
        for fixture in (base_ltx, delta_2, delta_3, compacted_2):
            run_reference("verify-ltx", fixture)

        # Keep one atomic directory publish so a failed generator cannot leave
        # a partial fixture set that blocks a later deliberate generation.
        os.rename(generated, FIXTURE_DIRECTORY)

    for fixture in sorted(FIXTURE_DIRECTORY.iterdir()):
        digest = hashlib.sha256(fixture.read_bytes()).hexdigest()
        print(f"{fixture.name}  {digest}")


if __name__ == "__main__":
    main()
