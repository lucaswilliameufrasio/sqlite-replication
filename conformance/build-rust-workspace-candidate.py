#!/usr/bin/env python3
"""Create a local, non-release Rust source candidate from the current checkout.

This includes tracked and non-ignored untracked files, including local edits.
The output is not a license approval or permission to distribute.
"""

import argparse
import gzip
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile


ROOT = Path(__file__).resolve().parent.parent
SCOPES = ("rust", "conformance/fixtures", "docs/sbom/2026-11-05")
WARNING = (
    "LOCAL TEST CANDIDATE — NOT APPROVED FOR DISTRIBUTION\n"
    "Contains uncommitted work and preliminary notices; rights remain unconfirmed.\n"
    "Includes Rust workspace, test fixtures, and preliminary SBOM/notice evidence.\n"
    "Does not vendor third-party dependencies or contain an approved project license.\n"
).encode()


def add_file(archive, name, content, executable=False):
    info = tarfile.TarInfo(name)
    info.size = len(content)
    info.mode = 0o755 if executable else 0o644
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mtime = 0
    archive.addfile(info, io.BytesIO(content))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new .tar.gz path; never overwritten")
    args = parser.parse_args()
    result = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *SCOPES],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    )
    paths = sorted(set(result.stdout.decode("utf-8").split("\0")) - {""})
    files = []
    for name in paths:
        path = ROOT / name
        if path.is_symlink() or not path.is_file():
            raise SystemExit(f"refusing non-regular file: {name}")
        if any(part.startswith(".env") or part in {".git", "target"} for part in path.parts):
            raise SystemExit(f"refusing sensitive or build path: {name}")
        content = path.read_bytes()
        files.append((name, content, bool(path.stat().st_mode & 0o111)))

    manifest = "".join(f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data, _ in files).encode()
    with args.output.open("xb") as output:
        with gzip.GzipFile(filename="", fileobj=output, mode="wb", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                add_file(archive, "CANDIDATE-WARNING.txt", WARNING)
                add_file(archive, "SHA256SUMS", manifest)
                for name, content, executable in files:
                    add_file(archive, name, content, executable)
    print(f"candidate={args.output} files={len(files)}")


if __name__ == "__main__":
    main()
