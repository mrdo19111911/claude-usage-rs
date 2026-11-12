# The app icon from the software's logo (docs/specs/exe-resources.md).
#
# In:  assets/logo.jpg, the "smartlog" wordmark (blue on white) with its tagline.
# Out: assets/scas.ico (16, 24, 32, 48, 64, 256 px) for the exe and Setup, and
#      src/web/img/app-icon.png (64 px) for the dashboard's tab. Each image is a white
#      rounded square, clear outside: the first letter "s" up to 64 px (the word can't be
#      read that small), the whole logo at 256 px; under it two usage bars, green over amber,
#      as in the tray icon.
#
#   python -P tools/make_icon.py
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SIZES = [16, 24, 32, 48, 64, 256]
PAGE_SIZE = 64
DARK = 200  # a logo pixel: grey level under this
# The two usage bars: the tray's 5-hour green and amber, each partly filled, grey after.
BARS = [((63, 179, 79, 255), 0.60), ((242, 182, 50, 255), 0.85)]
TRACK = (215, 215, 220, 255)


def _runs(values):
    # Consecutive runs of sorted ints, as (first, last).
    out = []
    for v in values:
        if out and v == out[-1][1] + 1:
            out[-1][1] = v
        else:
            out.append([v, v])
    return [tuple(r) for r in out]


def first_letter(logo):
    # Box of the wordmark's first letter: the rows where the word's lower-case letters
    # stand (the run of busy rows holding the busiest one), then the first run of
    # columns with ink in those rows.
    g = logo.convert("L")
    px, (w, h) = g.load(), g.size
    count = [sum(px[x, y] < DARK for x in range(w)) for y in range(h)]
    top = max(range(h), key=count.__getitem__)
    busy = [y for y in range(h) if count[y] >= count[top] // 4]
    y0, y1 = next(r for r in _runs(busy) if r[0] <= top <= r[1])
    cols = [x for x in range(w) if any(px[x, y] < DARK for y in range(y0, y1 + 1))]
    x0, x1 = _runs(cols)[0]
    return x0, y0, x1 + 1, y1 + 1


def ink_box(logo):
    # Box of everything that is not background.
    g = logo.convert("L").point(lambda v: 255 if v < DARK else 0)
    return g.getbbox()


def bar_layout(size):
    # (left, right, [(top, bottom) of the green bar, of the amber bar]) in whole pixels.
    m, bh, gap = max(2, round(size * 0.14)), max(2, round(size * 0.1)), max(1, round(size * 0.05))
    amber = size - m - bh
    green = amber - gap - bh
    return m, size - m, [(green, green + bh), (amber, amber + bh)]


def draw_bars(img):
    # The bars at the final size, on whole pixels, so they stay crisp at 16 px.
    x0, x1, rows = bar_layout(img.width)
    d = ImageDraw.Draw(img)
    for (color, part), (y0, y1) in zip(BARS, rows):
        d.rectangle((x0, y0, x1 - 1, y1 - 1), fill=TRACK)
        d.rectangle((x0, y0, x0 + max(1, round((x1 - x0) * part)) - 1, y1 - 1), fill=color)
    return img


def tile(art, size, pad):
    # `art` (logo pixels on white) centred above the bars on a white rounded square of
    # `size`, clear outside, then the bars.
    big = size * 4  # drawn large, then reduced: smooth corners and letter edges
    sq = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, big - 1, big - 1), radius=big * 0.2, fill=255)
    sq.paste((255, 255, 255, 255), (0, 0), mask)
    _, _, rows = bar_layout(size)
    top, bottom = big * pad, (rows[0][0] - max(1, round(size * 0.05))) * 4
    k = min(big * (1 - 2 * pad) / art.width, (bottom - top) / art.height)
    a = art.convert("RGBA").resize((max(1, round(art.width * k)), max(1, round(art.height * k))), Image.LANCZOS)
    sq.paste(a, ((big - a.width) // 2, round(top + (bottom - top - a.height) / 2)))
    sq.putalpha(Image.composite(sq.getchannel("A"), mask, mask))
    out = sq.resize((size, size), Image.LANCZOS)
    # The reduction's ringing leaves a faint film outside the corners: clear it.
    out.putalpha(out.getchannel("A").point(lambda v: 0 if v < 8 else v))
    return draw_bars(out)


def build(logo):
    rgb = logo.convert("RGB")
    letter = rgb.crop(first_letter(rgb))
    word = rgb.crop(ink_box(rgb))
    return {s: tile(word, s, 0.06) if s == 256 else tile(letter, s, 0.08) for s in SIZES}


def write(icons, ico_path, png_path):
    big = icons[256]
    big.save(ico_path, format="ICO", sizes=[(s, s) for s in SIZES],
             append_images=[icons[s] for s in SIZES if s != 256])
    icons[PAGE_SIZE].save(png_path, format="PNG")


if __name__ == "__main__":
    icons = build(Image.open(ROOT / "assets" / "logo.jpg"))
    write(icons, ROOT / "assets" / "scas.ico", ROOT / "src" / "web" / "img" / "app-icon.png")
    print("wrote assets/scas.ico and src/web/img/app-icon.png")
