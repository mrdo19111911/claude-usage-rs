# Tests for build_linux.py (docs/specs/linux.md). Run:
#   python -P tools/test_build_linux.py
import struct, sys, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_linux  # noqa: E402


def elf(machine=62, interp=False, cls=2):
    # A minimal ELF64 little-endian header with one or two program headers.
    phs = [(1, 0)] + ([(3, 0)] if interp else [])  # PT_LOAD, PT_INTERP
    ident = b"\x7fELF" + bytes([cls, 1, 1]) + bytes(9)
    head = ident + struct.pack("<HHIQQQIHHHHHH", 2, machine, 1, 0, 64, 0, 0, 64, 56, len(phs), 64, 0, 0)
    body = b"".join(struct.pack("<IIQQQQQQ", t, 0, 0, 0, 0, 0, 0, 0) for t, _ in phs)
    return head + body


class ElfInfo(unittest.TestCase):
    def test_static_x86_64(self):
        self.assertEqual(build_linux.elf_info(elf()), {"x86_64": True, "static": True})

    def test_dynamic_has_interp(self):
        self.assertEqual(build_linux.elf_info(elf(interp=True)), {"x86_64": True, "static": False})

    def test_other_arch(self):
        self.assertFalse(build_linux.elf_info(elf(machine=183))["x86_64"])

    def test_not_elf(self):
        self.assertIsNone(build_linux.elf_info(b"MZ\x90\x00" + bytes(100)))
        self.assertIsNone(build_linux.elf_info(elf(cls=1)))


class Command(unittest.TestCase):
    def test_release_musl_own_target_dir(self):
        cmd = build_linux.command("x86_64-unknown-linux-musl")
        self.assertEqual(cmd[:2], ["cargo", "zigbuild"])
        self.assertIn("--release", cmd)
        self.assertIn("--locked", cmd)
        self.assertEqual(cmd[cmd.index("--target") + 1], "x86_64-unknown-linux-musl")
        self.assertEqual(cmd[cmd.index("--target-dir") + 1], "target/linux")

    def test_output_path(self):
        p = build_linux.output("x86_64-unknown-linux-gnu.2.17")
        self.assertEqual(p.as_posix(), "target/linux/x86_64-unknown-linux-gnu/release/scas")


class ZigOnPath(unittest.TestCase):
    def test_adds_pip_zig_dir_once(self):
        env = {"PATH": "a"}
        build_linux.add_zig(env, Path("z"))
        build_linux.add_zig(env, Path("z"))
        self.assertEqual(env["PATH"].split(build_linux.os.pathsep), ["a", "z"])


if __name__ == "__main__":
    unittest.main()
