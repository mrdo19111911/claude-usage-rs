# Assemble SCAS.app from the universal binary; write it as a folder and as a tar.gz.
# Spec: docs/specs/macos.md ("SCAS.app and the .dmg"). Tests: tools/test_build_macos_app.py.
# Runs on Windows (after tools/build_macos.py) and on the macOS runner (after lipo):
#   python tools/build_macos_app.py [--binary PATH] [--out DIR]
import argparse, shutil, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_macos  # noqa: E402
import macho  # noqa: E402
import macos_bundle  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = ROOT / "target" / "mac-zig" / build_macos.TARGET / "release" / "scas"


def tar_name(version):
    return f"SCAS-{version}-macos-app.tar.gz"


def main(argv=None):
    ap = argparse.ArgumentParser(description="Build SCAS.app")
    ap.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    ap.add_argument("--out", type=Path, default=ROOT / "dist")
    a = ap.parse_args(argv)
    binary = a.binary.read_bytes()
    try:
        bad = macho.problems(macho.slices(binary))
    except ValueError as e:
        bad = [str(e)]
    if bad:
        sys.exit(f"{a.binary}: not a usable macOS binary:\n  " + "\n  ".join(bad))
    version = build_macos.version((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    icon = (ROOT / "assets" / "scas.icns").read_bytes()
    items = macos_bundle.entries(binary, icon, version)
    app = a.out / macos_bundle.APP
    if app.exists():
        shutil.rmtree(app)
    macos_bundle.write_dir(items, a.out)
    tar = macos_bundle.write_tar(items, a.out / tar_name(version))
    print(f"{app}\n{tar}\n  {tar.stat().st_size} bytes, sha256 {build_macos.sha256(tar)}")


if __name__ == "__main__":
    main()
