#!/usr/bin/env python3
"""Generate a multi-delta LTX chain using the pinned independent oracle."""

import hashlib
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent
REFERENCE = ROOT / "reference"
FIXTURE_ROOT = ROOT / "fixtures" / "ltx-v2"
FIXTURE_DIRECTORY = FIXTURE_ROOT / "long-chain-4096"
DELTA_COUNT = 32
BASE_TIMESTAMP_MILLIS = 1_700_100_000_000


def create_base(path):
    with sqlite3.connect(path) as connection:
        connection.execute("PRAGMA page_size=4096")
        connection.execute("PRAGMA journal_mode=DELETE")
        connection.execute("CREATE TABLE chain_values (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        connection.execute("INSERT INTO chain_values VALUES (1, 'step-001')")
        connection.commit()
        connection.execute("VACUUM")


def main():
    if FIXTURE_DIRECTORY.exists():
        raise FileExistsError(f"Refusing to overwrite existing fixtures: {FIXTURE_DIRECTORY}")
    FIXTURE_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ltx-long-chain-", dir=FIXTURE_ROOT) as temporary:
        work = Path(temporary)
        generated = work / "fixtures"
        generated.mkdir()
        oracle = work / "reference"
        subprocess.run(["go", "build", "-o", str(oracle), "."], cwd=REFERENCE, check=True)

        previous = work / "state-1.sqlite"
        create_base(previous)
        snapshot = generated / "snapshot-1.ltx"
        subprocess.run(
            [str(oracle), "create-at", str(previous), str(snapshot), "0000000000000001", "0000000000000001", str(BASE_TIMESTAMP_MILLIS)],
            check=True,
        )
        fixtures = [snapshot]

        for txid in range(2, DELTA_COUNT + 2):
            current = work / f"state-{txid}.sqlite"
            shutil.copyfile(previous, current)
            with sqlite3.connect(current) as connection:
                connection.execute(
                    "UPDATE chain_values SET value=? WHERE id=1",
                    (f"step-{txid:03d}",),
                )
                connection.commit()
            delta = generated / f"delta-{txid:03d}.ltx"
            subprocess.run(
                [
                    str(oracle),
                    "create-delta",
                    str(previous),
                    str(current),
                    str(delta),
                    f"{txid:016x}",
                    str(BASE_TIMESTAMP_MILLIS + txid),
                ],
                check=True,
            )
            fixtures.append(delta)
            previous = current

        for fixture in fixtures:
            subprocess.run([str(oracle), "verify-ltx", str(fixture)], check=True)

        checksum_lines = []
        for fixture in fixtures:
            digest = hashlib.sha256(fixture.read_bytes()).hexdigest()
            checksum_lines.append(f"{digest}  {fixture.name}")
        (generated / "SHA256SUMS").write_text("\n".join(checksum_lines) + "\n", encoding="ascii")
        generated.rename(FIXTURE_DIRECTORY)

    print(f"{FIXTURE_DIRECTORY}: {len(fixtures)} independently verified LTX files")


if __name__ == "__main__":
    main()
