# Builds the Linux `scas` from any host (Windows included) with cargo-zigbuild.
# Spec: docs/specs/linux.md; why this route: docs/specs/linux-build-prior-art.md.
#
# In:  an optional target (default x86_64-unknown-linux-musl, a static binary;
#      x86_64-unknown-linux-gnu.2.17 gives a glibc >= 2.17 one).
# Out: target/linux/<target>/release/scas, checked to be an x86_64 ELF (static for
#      musl); prints its size and SHA-256. Exit code 1 when the build or check fails.
# Needs: rustup target add <target>; pip install ziglang cargo-zigbuild (or
#        cargo install --locked cargo-zigbuild). The pip `ziglang` zig is put on PATH.
#
#   python tools/build_linux.py [target]
import hashlib, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET_DIR = "target/linux"


def elf_info(data):
    # {"x86_64", "static"} for an ELF64 little-endian file, else None. Static = no PT_INTERP.
    if len(data) < 64 or data[:4] != b"\x7fELF" or data[4] != 2 or data[5] != 1:
        return None
    machine, = struct.unpack_from("<H", data, 18)
    phoff, = struct.unpack_from("<Q", data, 32)
    phentsize, phnum = struct.unpack_from("<HH", data, 54)
    interp = any(
        struct.unpack_from("<I", data, phoff + i * phentsize)[0] == 3
        for i in range(phnum) if phoff + (i + 1) * phentsize <= len(data)
    )
    return {"x86_64": machine == 62, "static": not interp}


def command(target):
    return ["cargo", "zigbuild", "--release", "--locked", "--target", target, "--target-dir", TARGET_DIR]


def output(target):
    # cargo-zigbuild drops the ".2.17" glibc suffix from the output folder.
    return Path(TARGET_DIR) / target.split(".")[0] / "release" / "scas"


def add_zig(env, zig_dir):
    parts = env.get("PATH", "").split(os.pathsep)
    if str(zig_dir) not in parts:
        env["PATH"] = os.pathsep.join([p for p in parts if p] + [str(zig_dir)])


def pip_zig_dir():
    try:
        import ziglang
        return Path(ziglang.__file__).parent
    except ImportError:
        return None


def main(argv):
    target = argv[1] if len(argv) > 1 else "x86_64-unknown-linux-musl"
    env = dict(os.environ)
    if shutil.which("zig") is None and (z := pip_zig_dir()):
        add_zig(env, z)
    if subprocess.run(command(target), cwd=ROOT, env=env).returncode != 0:
        return 1
    path = ROOT / output(target)
    data = path.read_bytes()
    info = elf_info(data)
    want_static = "musl" in target
    if not info or not info["x86_64"] or (want_static and not info["static"]):
        print(f"FAIL: {path} is not the expected Linux binary: {info}")
        return 1
    print(f"{path}\n  {len(data):,} bytes, {'static' if info['static'] else 'dynamic'}, "
          f"sha256 {hashlib.sha256(data).hexdigest()}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
