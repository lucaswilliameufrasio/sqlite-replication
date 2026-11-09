import hashlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parent.parent
BUILDER = ROOT / "conformance" / "build-rust-workspace-candidate.py"


class RustWorkspaceCandidateTest(unittest.TestCase):
    def test_reproducible_manifest_warning_and_no_overwrite(self):
        with tempfile.TemporaryDirectory(prefix="candidate-test-") as directory:
            first = Path(directory) / "first.tar.gz"
            second = Path(directory) / "second.tar.gz"
            for output in (first, second):
                subprocess.run([sys.executable, str(BUILDER), str(output)], check=True, capture_output=True)
            first_bytes = first.read_bytes()
            self.assertEqual(hashlib.sha256(first_bytes).digest(), hashlib.sha256(second.read_bytes()).digest())
            with tarfile.open(first, "r:gz") as archive:
                members = archive.getmembers()
                names = [member.name for member in members]
                self.assertEqual(len(names), len(set(names)))
                for member in members:
                    self.assertTrue(member.isfile(), member.name)
                    path = PurePosixPath(member.name)
                    self.assertFalse(path.is_absolute(), member.name)
                    self.assertNotIn("..", path.parts)
                lines = archive.extractfile("SHA256SUMS").read().decode().splitlines()
                manifest = {}
                for line in lines:
                    digest, name = line.split("  ", 1)
                    self.assertNotIn(name, manifest)
                    manifest[name] = digest
                self.assertEqual(set(names), set(manifest) | {"SHA256SUMS", "CANDIDATE-WARNING.txt"})
                self.assertIn("rust/Cargo.lock", manifest)
                self.assertTrue(any(name.startswith("conformance/fixtures/") for name in manifest))
                self.assertTrue(any(name.startswith("docs/sbom/2026-11-05/") for name in manifest))
                for name, digest in manifest.items():
                    self.assertNotIn("target", PurePosixPath(name).parts)
                    self.assertEqual(hashlib.sha256(archive.extractfile(name).read()).hexdigest(), digest, name)
                warning = archive.extractfile("CANDIDATE-WARNING.txt").read().decode()
                self.assertIn("NOT APPROVED FOR DISTRIBUTION", warning)
                self.assertIn("new, separate Cargo target directory", warning)
            retry = subprocess.run([sys.executable, str(BUILDER), str(first)], capture_output=True)
            self.assertNotEqual(retry.returncode, 0)
            self.assertEqual(first.read_bytes(), first_bytes)


if __name__ == "__main__":
    unittest.main()
