# Tests for build_macos_app.py and macos_bundle.py (docs/specs/macos.md,
# "SCAS.app and the .dmg"). Run:
#   python -P tools/test_build_macos_app.py
import plistlib, shutil, subprocess, sys, tarfile, tempfile, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_macos_app  # noqa: E402
import macos_bundle  # noqa: E402

BIN, ICON = b"\xca\xfe\xba\xbe-fake-binary", b"icns-fake"
APP = "SCAS.app/Contents"


def entries():
    return {p: (data, mode) for p, data, mode in macos_bundle.entries(BIN, ICON, "1.6.1")}


class Plist(unittest.TestCase):
    def test_keys(self):
        p = plistlib.loads(entries()[f"{APP}/Info.plist"][0])
        self.assertEqual(p["CFBundleExecutable"], "scas-launcher")
        self.assertEqual(p["CFBundleName"], "SCAS")
        self.assertEqual(p["CFBundleDisplayName"], "Smartlog Claude Auto Switch")
        self.assertEqual(p["CFBundleIdentifier"], "io.github.mrdo19111911.scas")
        self.assertEqual(p["CFBundleShortVersionString"], "1.6.1")
        self.assertEqual(p["CFBundleVersion"], "1.6.1")
        self.assertEqual(p["CFBundlePackageType"], "APPL")
        self.assertEqual(p["CFBundleIconFile"], "scas")
        self.assertEqual(p["LSMinimumSystemVersion"], "11.0")
        self.assertIs(p["LSUIElement"], True)


class Layout(unittest.TestCase):
    def test_files_and_modes(self):
        e = entries()
        self.assertEqual(e[f"{APP}/MacOS/scas"], (BIN, 0o755))
        self.assertEqual(e[f"{APP}/Resources/scas.icns"], (ICON, 0o644))
        self.assertEqual(e[f"{APP}/MacOS/scas-launcher"][1], 0o755)
        self.assertEqual(e[f"{APP}/Info.plist"][1], 0o644)
        for d in ["SCAS.app", APP, f"{APP}/MacOS", f"{APP}/Resources"]:
            self.assertEqual(e[d], (None, 0o755), d)

    def test_no_two_paths_differ_only_in_case(self):
        # macOS volumes are case-insensitive by default: SCAS and scas would be one file.
        paths = [p.lower() for p, _, _ in macos_bundle.entries(BIN, ICON, "1.6.1")]
        self.assertEqual(len(paths), len(set(paths)))

    def test_parents_come_first(self):
        paths = [p for p, _, _ in macos_bundle.entries(BIN, ICON, "1.6.1")]
        for i, p in enumerate(paths):
            parent = p.rsplit("/", 1)[0]
            if "/" in p:
                self.assertIn(parent, paths[:i], p)


class Launcher(unittest.TestCase):
    def setUp(self):
        self.text = entries()[f"{APP}/MacOS/scas-launcher"][0]

    def test_is_sh_with_lf_only(self):
        self.assertTrue(self.text.startswith(b"#!/bin/sh\n"))
        self.assertNotIn(b"\r", self.text)

    def test_asks_the_port_then_opens_or_starts(self):
        t = self.text.decode()
        self.assertIn("${PORT:-4777}", t)
        self.assertIn("/api/state", t)
        self.assertIn("/usr/bin/open", t)
        self.assertIn("nohup", t)
        self.assertIn("set -m", t)
        self.assertIn('"$here/scas"', t)
        self.assertIn("Library/Logs/SCAS", t)

    @unittest.skipUnless(shutil.which("sh"), "no sh")
    def test_sh_parses_it(self):
        with tempfile.TemporaryDirectory() as d:
            f = Path(d) / "SCAS"
            f.write_bytes(self.text)
            r = subprocess.run(["sh", "-n", str(f)], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)


class Write(unittest.TestCase):
    def test_tar_carries_modes(self):
        with tempfile.TemporaryDirectory() as d:
            out = macos_bundle.write_tar(macos_bundle.entries(BIN, ICON, "1.6.1"), Path(d) / "a.tar.gz")
            with tarfile.open(out) as t:
                m = {x.name: x for x in t.getmembers()}
            self.assertTrue(m["SCAS.app"].isdir())
            self.assertEqual(m["SCAS.app"].mode, 0o755)
            self.assertEqual(m[f"{APP}/MacOS/scas-launcher"].mode, 0o755)
            self.assertEqual(m[f"{APP}/MacOS/scas"].mode, 0o755)
            self.assertEqual(m[f"{APP}/Info.plist"].mode, 0o644)

    def test_dir_has_the_files(self):
        with tempfile.TemporaryDirectory() as d:
            macos_bundle.write_dir(macos_bundle.entries(BIN, ICON, "1.6.1"), Path(d))
            self.assertEqual((Path(d) / APP / "MacOS" / "scas").read_bytes(), BIN)
            self.assertTrue((Path(d) / APP / "Info.plist").is_file())


class Main(unittest.TestCase):
    def test_refuses_a_bad_binary(self):
        with tempfile.TemporaryDirectory() as d:
            b = Path(d) / "scas"
            b.write_bytes(b"MZ not a mac binary at all")
            with self.assertRaises(SystemExit):
                build_macos_app.main(["--binary", str(b), "--out", d])
            self.assertFalse((Path(d) / "SCAS.app").exists())

    def test_names(self):
        self.assertEqual(build_macos_app.tar_name("1.6.1"), "SCAS-1.6.1-macos-app.tar.gz")


if __name__ == "__main__":
    unittest.main()
