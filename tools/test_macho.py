# Tests for macho.py (docs/specs/macos.md, "Build"). Run:
#   python -P tools/test_macho.py
import struct, sys, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import macho  # noqa: E402

ARM, X86 = 0x0100000C, 0x01000007


def dylib_cmd(path):
    name = path.encode() + b"\0"
    name += b"\0" * (-(24 + len(name)) % 8)
    return struct.pack("<IIIIII", 0xC, 24 + len(name), 24, 0, 0, 0) + name


def build_cmd(minos):
    major, minor = minos
    return struct.pack("<IIIIII", 0x32, 24, 1, (major << 16) | (minor << 8), 27 << 16, 0)


def thin(cpu, signed=True, minos=(11, 0), dylibs=("/usr/lib/libSystem.B.dylib",), filetype=2):
    cmds = [build_cmd(minos)] + [dylib_cmd(d) for d in dylibs]
    if signed:
        cmds.append(struct.pack("<IIII", 0x1D, 16, 0, 0))
    body = b"".join(cmds)
    head = struct.pack("<IiiIIIII", 0xFEEDFACF, cpu, 0, filetype, len(cmds), len(body), 0, 0)
    return head + body


def fat(*slices):
    out = struct.pack(">II", 0xCAFEBABE, len(slices))
    off = 4096
    heads, blobs = b"", b""
    for cpu, data in slices:
        heads += struct.pack(">iiIII", cpu, 0, off + len(blobs), len(data), 12)
        blobs += data
    return out + heads + b"\0" * (off - len(out) - len(heads)) + blobs


GOOD = fat((X86, thin(X86, signed=False)), (ARM, thin(ARM)))


class Slices(unittest.TestCase):
    def test_reads_each_slice(self):
        s = {x["arch"]: x for x in macho.slices(GOOD)}
        self.assertEqual(set(s), {"arm64", "x86_64"})
        self.assertTrue(s["arm64"]["signed"])
        self.assertFalse(s["x86_64"]["signed"])
        self.assertEqual(s["arm64"]["minos"], "11.0")
        self.assertEqual(s["arm64"]["filetype"], 2)
        self.assertEqual(s["arm64"]["dylibs"], ["/usr/lib/libSystem.B.dylib"])

    def test_thin_file_is_one_slice(self):
        self.assertEqual([x["arch"] for x in macho.slices(thin(ARM))], ["arm64"])

    def test_not_macho_raises(self):
        with self.assertRaises(ValueError):
            macho.slices(b"MZ\x90\x00" + b"\0" * 60)


class Problems(unittest.TestCase):
    def test_good_universal_has_none(self):
        self.assertEqual(macho.problems(macho.slices(GOOD)), [])

    def test_missing_arch(self):
        p = macho.problems(macho.slices(fat((ARM, thin(ARM)))))
        self.assertTrue(any("x86_64" in x for x in p), p)

    def test_unsigned_arm64(self):
        p = macho.problems(macho.slices(fat((X86, thin(X86)), (ARM, thin(ARM, signed=False)))))
        self.assertTrue(any("arm64" in x and "sign" in x for x in p), p)

    def test_wrong_minos(self):
        p = macho.problems(macho.slices(fat((X86, thin(X86)), (ARM, thin(ARM, minos=(14, 0))))))
        self.assertTrue(any("14.0" in x for x in p), p)

    def test_foreign_dylib(self):
        bad = thin(ARM, dylibs=("/usr/lib/libSystem.B.dylib", "/opt/homebrew/lib/libz.dylib"))
        p = macho.problems(macho.slices(fat((X86, thin(X86)), (ARM, bad))))
        self.assertTrue(any("/opt/homebrew" in x for x in p), p)

    def test_not_executable(self):
        p = macho.problems(macho.slices(fat((X86, thin(X86)), (ARM, thin(ARM, filetype=6)))))
        self.assertTrue(any("executable" in x for x in p), p)


if __name__ == "__main__":
    unittest.main()
