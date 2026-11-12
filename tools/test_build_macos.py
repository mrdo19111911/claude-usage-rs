# Tests for build_macos.py (docs/specs/macos.md, "Build"). Run:
#   python -P tools/test_build_macos.py
import hashlib, sys, tarfile, tempfile, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_macos  # noqa: E402


class Version(unittest.TestCase):
    def test_reads_package_version(self):
        toml = '[package]\nname = "scas"\nversion = "1.6.1"\n\n[dependencies]\nx = { version = "2" }\n'
        self.assertEqual(build_macos.version(toml), "1.6.1")


class Command(unittest.TestCase):
    def test_zigbuild_universal(self):
        c = build_macos.command()
        self.assertEqual(c[:2], ["cargo", "zigbuild"])
        for a in ["--release", "--locked", "--target", "universal2-apple-darwin"]:
            self.assertIn(a, c)

    def test_env_sets_deployment_target_and_zig_on_path(self):
        env = build_macos.build_env({"PATH": "C:\\x"}, Path("C:/zig"), Path("C:/t"))
        self.assertEqual(env["MACOSX_DEPLOYMENT_TARGET"], "11.0")
        self.assertTrue(env["PATH"].startswith(str(Path("C:/zig"))))
        self.assertTrue(env["PATH"].endswith("C:\\x"))
        self.assertEqual(env["CARGO_TARGET_DIR"], str(Path("C:/t")))


class Pack(unittest.TestCase):
    def test_tar_holds_scas_executable(self):
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "bin"
            src.write_bytes(b"\xca\xfe\xba\xbe" + b"x" * 100)
            out = build_macos.pack(src, Path(d) / "dist", "1.2.3")
            self.assertEqual(out.name, "scas-1.2.3-macos-universal.tar.gz")
            with tarfile.open(out) as t:
                (m,) = t.getmembers()
                self.assertEqual(m.name, "scas")
                self.assertEqual(m.mode, 0o755)
                self.assertEqual(t.extractfile(m).read(), src.read_bytes())

    def test_sha256(self):
        with tempfile.TemporaryDirectory() as d:
            f = Path(d) / "a"
            f.write_bytes(b"abc")
            self.assertEqual(build_macos.sha256(f), hashlib.sha256(b"abc").hexdigest())


if __name__ == "__main__":
    unittest.main()
