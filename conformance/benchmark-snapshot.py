#!/usr/bin/env python3
"""Compare isolated one-shot snapshot/restore commands, not continuous replication."""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import sqlite3
import statistics
import subprocess
import tempfile
import time


def digest_database(path):
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        if connection.execute("PRAGMA integrity_check").fetchall() != [("ok",)]:
            raise RuntimeError("SQLite integrity check failed")
        digest = hashlib.sha256()
        count = 0
        for row_id, payload in connection.execute("SELECT id, payload FROM data ORDER BY id"):
            digest.update(row_id.to_bytes(8, "big"))
            digest.update(len(payload).to_bytes(8, "big"))
            digest.update(payload)
            count += 1
        return count, digest.hexdigest()
    finally:
        connection.close()


def run_timed(command):
    started = time.perf_counter_ns()
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
    elapsed = time.perf_counter_ns() - started
    if result.returncode:
        raise RuntimeError(f"{command[0]} exited {result.returncode}: {result.stderr.decode(errors='replace')[-2000:]}")
    return elapsed


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--go", type=pathlib.Path, required=True)
    parser.add_argument("--rust", type=pathlib.Path, required=True)
    parser.add_argument("--litestream", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--rows", type=int, default=10000)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    args = parser.parse_args()
    if not (1 <= args.rows <= 100000 and 1 <= args.samples <= 30 and 0 <= args.warmups <= 5):
        parser.error("rows 1..100000, samples 1..30, warmups 0..5 required")
    binaries = {name: str(path.resolve(strict=True)) for name, path in (("go", args.go), ("rust", args.rust), ("litestream", args.litestream))}
    results = []
    with tempfile.TemporaryDirectory(prefix="sqlite-snapshot-bench-") as work:
        root = pathlib.Path(work)
        source = root / "fixture.sqlite"
        connection = sqlite3.connect(source)
        if connection.execute("PRAGMA journal_mode=WAL").fetchone()[0] != "wal":
            raise RuntimeError("could not enable WAL mode")
        connection.execute("CREATE TABLE data (id INTEGER PRIMARY KEY, payload BLOB NOT NULL)")
        connection.executemany(
            "INSERT INTO data VALUES (?, ?)",
            ((i, hashlib.shake_256(f"sqlite-bench-v1:{i}".encode()).digest(256)) for i in range(args.rows)),
        )
        connection.commit()
        connection.close()
        expected = digest_database(source)
        size = source.stat().st_size
        for iteration in range(args.warmups + args.samples):
            # Rotate execution order to reduce systematic order bias.
            names = ["go", "rust", "litestream"]
            names = names[iteration % 3:] + names[:iteration % 3]
            for name in names:
                trial = root / f"{iteration}-{name}"
                trial.mkdir()
                database = trial / "source.sqlite"
                shutil.copyfile(source, database)
                repository = trial / "repository"
                restored = trial / "restored.sqlite"
                executable = binaries[name]
                if name == "litestream":
                    url = repository.as_uri()
                    snapshot = [executable, "replicate", "-once", "-force-snapshot", str(database), url]
                    restore = [executable, "restore", "-o", str(restored), url]
                else:
                    snapshot = [executable, "snapshot", str(database), str(repository)]
                    restore = [executable, "restore", str(repository), str(restored)]
                snapshot_ns = run_timed(snapshot)
                restore_ns = run_timed(restore)
                if digest_database(restored) != expected:
                    raise RuntimeError(f"{name} iteration {iteration}: restored content mismatch")
                results.append({"engine": name, "iteration": iteration, "warmup": iteration < args.warmups,
                                "snapshot_ns": snapshot_ns, "restore_ns": restore_ns,
                                "repository_bytes": sum(p.stat().st_size for p in repository.rglob("*") if p.is_file())})
        report = {"schema": 1, "rows": args.rows, "fixture_bytes": size, "content_sha256": expected[1],
                  "python_sqlite": sqlite3.sqlite_version, "platform": platform.platform(),
                  "machine": platform.machine(), "cpu_count_visible": os.cpu_count(),
                  "samples": args.samples, "warmups": args.warmups, "results": results}
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(report, output, indent=2)
            output.write("\n")
        for name in binaries:
            measured = [r for r in results if r["engine"] == name and not r["warmup"]]
            print(f"{name}: snapshot median {statistics.median(r['snapshot_ns'] for r in measured)/1e6:.3f} ms; "
                  f"restore median {statistics.median(r['restore_ns'] for r in measured)/1e6:.3f} ms; "
                  f"verified {len(measured)} samples")


if __name__ == "__main__":
    main()
