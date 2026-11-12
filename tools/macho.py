# Read a Mach-O (fat or thin) and say what is wrong with it as a SCAS macOS binary.
# Spec: docs/specs/macos.md ("Build"). Tests: tools/test_macho.py.
#   slices(data) -> [{arch, filetype, signed, minos, dylibs}]
#   problems(slices) -> [str]; empty means: arm64 + x86_64 executables, arm64 signed,
#   minos 11.0, every dylib under /usr/lib/.
import struct

FAT, THIN64 = 0xCAFEBABE, 0xFEEDFACF
ARCHS = {0x01000007: "x86_64", 0x0100000C: "arm64"}
LC_LOAD_DYLIBS = {0xC, 0x80000018, 0x8000001F}  # load, weak, reexport
LC_CODE_SIGNATURE, LC_BUILD_VERSION = 0x1D, 0x32
EXECUTE = 2
WANT_ARCHS, WANT_MINOS = {"arm64", "x86_64"}, "11.0"


def _version(v):
    return f"{v >> 16}.{(v >> 8) & 0xFF}"


def _thin(s):
    if len(s) < 32 or struct.unpack("<I", s[:4])[0] != THIN64:
        raise ValueError("not a 64-bit little-endian Mach-O slice")
    _, cpu, _, filetype, ncmds, _, _, _ = struct.unpack("<IiiIIIII", s[:32])
    out = {"arch": ARCHS.get(cpu, hex(cpu)), "filetype": filetype, "signed": False,
           "minos": None, "dylibs": []}
    p = 32
    for _ in range(ncmds):
        cmd, size = struct.unpack("<II", s[p:p + 8])
        if cmd in LC_LOAD_DYLIBS:
            name_off = struct.unpack("<I", s[p + 8:p + 12])[0]
            out["dylibs"].append(s[p + name_off:p + size].split(b"\0")[0].decode())
        elif cmd == LC_CODE_SIGNATURE:
            out["signed"] = True
        elif cmd == LC_BUILD_VERSION:
            out["minos"] = _version(struct.unpack("<I", s[p + 12:p + 16])[0])
        p += size
    return out


def slices(data):
    if len(data) < 8:
        raise ValueError("too short for Mach-O")
    if struct.unpack(">I", data[:4])[0] != FAT:
        return [_thin(data)]
    n = struct.unpack(">I", data[4:8])[0]
    out = []
    for i in range(n):
        _, _, off, size, _ = struct.unpack(">iiIII", data[8 + 20 * i:28 + 20 * i])
        out.append(_thin(data[off:off + size]))
    return out


def problems(sl):
    got = sorted(x["arch"] for x in sl)
    out = [] if got == sorted(WANT_ARCHS) else [f"slices {got}, want arm64 and x86_64"]
    for x in sl:
        a = x["arch"]
        if x["filetype"] != EXECUTE:
            out.append(f"{a}: filetype {x['filetype']}, not an executable")
        if a == "arm64" and not x["signed"]:
            out.append("arm64: no code signature (Apple Silicon will not run it)")
        if x["minos"] != WANT_MINOS:
            out.append(f"{a}: minos {x['minos']}, want {WANT_MINOS}")
        out += [f"{a}: links {d}, not a system library" for d in x["dylibs"]
                if not d.startswith("/usr/lib/")]
    return out
