# assets/scas.icns for SCAS.app, drawn like the exe icon (make_icon.py): the first letter
# "s" up to 64 px, the whole logo from 128 px, the two usage bars under it.
# Spec: docs/specs/macos.md ("SCAS.app and the .dmg"). Checked by tools/test_icns.py.
#   python -P tools/make_icns.py
import io, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from PIL import Image  # noqa: E402

import icns  # noqa: E402
import make_icon  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent


def pngs(logo):
    rgb = logo.convert("RGB")
    letter = rgb.crop(make_icon.first_letter(rgb))
    word = rgb.crop(make_icon.ink_box(rgb))
    out = {}
    for s in icns.SIZES:
        img = make_icon.tile(word, s, 0.06) if s >= 128 else make_icon.tile(letter, s, 0.08)
        buf = io.BytesIO()
        img.save(buf, format="PNG", optimize=True)
        out[s] = buf.getvalue()
    return out


if __name__ == "__main__":
    data = icns.pack(pngs(Image.open(ROOT / "assets" / "logo.jpg")))
    (ROOT / "assets" / "scas.icns").write_bytes(data)
    print(f"wrote assets/scas.icns ({len(data)} bytes)")
