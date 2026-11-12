# Build the universal macOS scas from Windows (or Linux) with cargo-zigbuild, check it,
# pack it. Spec: docs/specs/macos.md ("Build"); why: docs/specs/macos-build-prior-art.md.
# Tests: tools/test_build_macos.py. Run from the repo root:
#   python tools/build_macos.py
# Needs: python -m pip install --user ziglang ; cargo install cargo-zigbuild
import hashlib, io, os, re, subprocess, sys, tarfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import macho  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
TARGET = "universal2-apple-darwin"


def version(cargo_toml):
    m = re.search(r'^\[package\][^\[]*?^version\s*=\s*"([^"]+)"', cargo_toml, re.M | re.S)
    if not m:
        raise ValueError("no [package] version in Cargo.toml")
    return m.group(1)


def command():
    return ["cargo", "zigbuild", "--release", "--locked", "--target", TARGET]


def build_env(base, zig_dir, target_dir):
    env = dict(base)
    env["PATH"] = str(zig_dir) + os.pathsep + base.get("PATH", "")
    env["MACOSX_DEPLOYMENT_TARGET"] = macho.WANT_MINOS
    env["CARGO_TARGET_DIR"] = str(target_dir)
    return env


def zig_dir():
    # The ziglang PyPI package holds zig(.exe) next to its __init__.py.
    try:
        import ziglang
    except ImportError:
        sys.exit("zig not found: python -m pip install --user ziglang")
    return Path(ziglang.__file__).parent


def pack(binary, out_dir, ver):
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / f"scas-{ver}-macos-universal.tar.gz"
    data = binary.read_bytes()
    info = tarfile.TarInfo("scas")
    info.size, info.mode, info.mtime = len(data), 0o755, int(binary.stat().st_mtime)
    with tarfile.open(out, "w:gz") as t:
        t.addfile(info, io.BytesIO(data))
    return out


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ver = version((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    target_dir = ROOT / "target" / "mac-zig"
    env = build_env(os.environ, zig_dir(), target_dir)
    print("$", " ".join(command()), flush=True)
    if subprocess.run(command(), cwd=ROOT, env=env).returncode != 0:
        sys.exit("build failed")
    binary = target_dir / TARGET / "release" / "scas"
    sl = macho.slices(binary.read_bytes())
    for x in sl:
        print(f"  {x['arch']}: signed={x['signed']} minos={x['minos']} dylibs={x['dylibs']}")
    bad = macho.problems(sl)
    if bad:
        sys.exit("binary check failed:\n  " + "\n  ".join(bad))
    out = pack(binary, ROOT / "dist", ver)
    print(f"{out}\n  {out.stat().st_size} bytes, sha256 {sha256(out)}")


if __name__ == "__main__":
    main()
