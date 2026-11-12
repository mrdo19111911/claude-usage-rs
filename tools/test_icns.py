# Tests for icns.py (docs/specs/macos.md, "SCAS.app and the .dmg"). Run:
#   python -P tools/test_icns.py
import struct, sys, unittest, zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import icns  # noqa: E402


def png(size):
    # A minimal valid PNG of size x size (grey, one filter byte per row).
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    raw = b"".join(b"\0" + b"\x80" * size for _ in range(size))
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 0, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


PNGS = {s: png(s) for s in icns.SIZES}


class Pack(unittest.TestCase):
    def test_header_and_length(self):
        data = icns.pack(PNGS)
        self.assertEqual(data[:4], b"icns")
        self.assertEqual(struct.unpack(">I", data[4:8])[0], len(data))

    def test_every_entry_is_a_png_of_its_size(self):
        entries = icns.entries(icns.pack(PNGS))
        self.assertEqual({t for t, _ in entries}, set(icns.TYPES))
        for t, body in entries:
            self.assertEqual(icns.png_size(body), icns.TYPES[t], t)

    def test_wrong_size_png_is_refused(self):
        bad = dict(PNGS)
        bad[512] = png(256)
        with self.assertRaises(ValueError):
            icns.pack(bad)

    def test_missing_size_is_refused(self):
        bad = dict(PNGS)
        del bad[1024]
        with self.assertRaises(ValueError):
            icns.pack(bad)


class Committed(unittest.TestCase):
    def test_assets_icns_is_complete(self):
        data = (Path(__file__).resolve().parent.parent / "assets" / "scas.icns").read_bytes()
        entries = dict(icns.entries(data))
        self.assertEqual(set(entries), set(icns.TYPES))
        for t, body in entries.items():
            self.assertEqual(icns.png_size(body), icns.TYPES[t], t)


if __name__ == "__main__":
    unittest.main()
