# Apple .icns made of PNG entries, without iconutil (runs on Windows).
# Spec: docs/specs/macos.md ("SCAS.app and the .dmg"). Tests: tools/test_icns.py.
#   pack({px: png_bytes}) -> icns bytes; needs every size in SIZES.
#   entries(icns_bytes) -> [(type, png_bytes)]; png_size(png_bytes) -> width.
import struct

# Type -> pixel size. "@2x" types reuse a size: ic11 = 16@2x, ic12 = 32@2x, ...
TYPES = {
    "icp4": 16, "icp5": 32, "ic11": 32, "icp6": 64, "ic12": 64, "ic07": 128,
    "ic08": 256, "ic13": 256, "ic09": 512, "ic14": 512, "ic10": 1024,
}
SIZES = sorted(set(TYPES.values()))
PNG_SIG = b"\x89PNG\r\n\x1a\n"


def png_size(data):
    if data[:8] != PNG_SIG or data[12:16] != b"IHDR":
        raise ValueError("not a PNG")
    w, h = struct.unpack(">II", data[16:24])
    if w != h:
        raise ValueError(f"PNG is {w}x{h}, not square")
    return w


def pack(pngs):
    body = b""
    for t, size in TYPES.items():
        if size not in pngs:
            raise ValueError(f"no {size} px PNG")
        data = pngs[size]
        if png_size(data) != size:
            raise ValueError(f"the {size} px PNG is {png_size(data)} px")
        body += t.encode() + struct.pack(">I", 8 + len(data)) + data
    return b"icns" + struct.pack(">I", 8 + len(body)) + body


def entries(data):
    if data[:4] != b"icns" or struct.unpack(">I", data[4:8])[0] != len(data):
        raise ValueError("not an icns file")
    out, p = [], 8
    while p < len(data):
        t, n = data[p:p + 4].decode(), struct.unpack(">I", data[p + 4:p + 8])[0]
        out.append((t, data[p + 8:p + n]))
        p += n
    return out
