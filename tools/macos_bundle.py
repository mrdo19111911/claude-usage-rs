# SCAS.app's contents: Info.plist, the launcher script, the binary, the icon; written as
# a folder or as a tar whose entries carry the modes (Windows has no chmod).
# Spec: docs/specs/macos.md ("SCAS.app and the .dmg"). Tests: tools/test_build_macos_app.py.
#   entries(binary, icns, version) -> [(path, bytes or None for a folder, mode)], parents first
#   write_dir(entries, root); write_tar(entries, out) -> out
import io, os, plistlib, tarfile, time

APP = "SCAS.app"
BUNDLE_ID = "io.github.mrdo19111911.scas"
MIN_MACOS = "11.0"
# Not "SCAS": macOS volumes are case-insensitive, so it would be the binary `scas` itself.
LAUNCHER_NAME = "scas-launcher"

# The bundle's executable. SCAS has no Cocoa loop, so the launcher never stays: it opens
# the running dashboard, or starts scas detached, and exits (LSUIElement: no Dock icon).
LAUNCHER = """#!/bin/sh
# SCAS.app launcher (docs/specs/macos.md, "SCAS.app and the .dmg").
port="${PORT:-4777}"
url="http://127.0.0.1:$port"
logs="$HOME/Library/Logs/SCAS"
mkdir -p "$logs"
note() { echo "$(date '+%Y-%m-%d %H:%M:%S') $1 $port" >> "$logs/launcher.log"; }
if /usr/bin/curl -sf -o /dev/null --max-time 2 "$url/api/state"; then
  note open
  /usr/bin/open "$url"
  exit 0
fi
here="$(cd "$(dirname "$0")" && pwd)"
note start
set -m
/usr/bin/nohup "$here/scas" < /dev/null >> "$logs/scas.log" 2>&1 &
exit 0
"""


def info_plist(version):
    return plistlib.dumps({
        "CFBundleName": "SCAS",
        "CFBundleDisplayName": "Smartlog Claude Auto Switch",
        "CFBundleExecutable": LAUNCHER_NAME,
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "CFBundlePackageType": "APPL",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleIconFile": "scas",
        "LSMinimumSystemVersion": MIN_MACOS,
        "LSUIElement": True,
        "NSHighResolutionCapable": True,
    })


def entries(binary, icns, version):
    c = f"{APP}/Contents"
    return [
        (APP, None, 0o755),
        (c, None, 0o755),
        (f"{c}/Info.plist", info_plist(version), 0o644),
        (f"{c}/MacOS", None, 0o755),
        (f"{c}/MacOS/{LAUNCHER_NAME}", LAUNCHER.encode(), 0o755),
        (f"{c}/MacOS/scas", binary, 0o755),
        (f"{c}/Resources", None, 0o755),
        (f"{c}/Resources/scas.icns", icns, 0o644),
    ]


def write_dir(items, root):
    for path, data, mode in items:
        p = root / path
        if data is None:
            p.mkdir(parents=True, exist_ok=True)
        else:
            p.write_bytes(data)
        os.chmod(p, mode)  # real on macOS; only the read-only bit on Windows


def write_tar(items, out):
    out.parent.mkdir(parents=True, exist_ok=True)
    now = int(time.time())
    with tarfile.open(out, "w:gz") as t:
        for path, data, mode in items:
            info = tarfile.TarInfo(path)
            info.mode, info.mtime = mode, now
            if data is None:
                info.type = tarfile.DIRTYPE
                t.addfile(info)
            else:
                info.size = len(data)
                t.addfile(info, io.BytesIO(data))
    return out
