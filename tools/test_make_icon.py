# Tests for make_icon.py (docs/specs/exe-resources.md). Run:
#   python -P tools/test_make_icon.py
import sys, tempfile, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_icon  # noqa: E402
from PIL import Image  # noqa: E402

LOGO = Path(__file__).resolve().parent.parent / "assets" / "logo.jpg"
BLUE = (24, 77, 127)
GREEN, AMBER = (63, 179, 79), (242, 182, 50)


def near(c, want, tol=40):
    return c[3] > 200 and all(abs(a - b) < tol for a, b in zip(c[:3], want))


def rows_of(img, want):
    # Rows holding a pixel of colour `want`.
    px = img.load()
    return [y for y in range(img.height) if any(near(px[x, y], want) for x in range(img.width))]


def blue_box(img):
    # Bounding box of the logo-blue pixels (dark, blue above red).
    px = img.load()
    pts = [(x, y) for y in range(img.height) for x in range(img.width)
           if px[x, y][3] > 128 and px[x, y][2] < 200 and px[x, y][2] > px[x, y][0] + 30]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), min(ys), max(xs) + 1, max(ys) + 1


class MakeIcon(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.logo = Image.open(LOGO)
        cls.icons = make_icon.build(cls.logo)

    def test_every_size_is_a_square_with_alpha(self):
        self.assertEqual(sorted(self.icons), make_icon.SIZES)
        for s, img in self.icons.items():
            self.assertEqual((img.mode, img.size), ("RGBA", (s, s)))

    def test_corners_are_clear_and_the_middle_is_white_or_blue(self):
        for s, img in self.icons.items():
            self.assertEqual(img.getpixel((0, 0))[3], 0, s)
            self.assertEqual(img.getpixel((s - 1, s - 1))[3], 0, s)
            self.assertEqual(img.getpixel((s // 2, 1))[3], 255, f"{s}: the square reaches the edge")

    def test_the_first_letter_is_found(self):
        x0, y0, x1, y1 = make_icon.first_letter(self.logo)
        self.assertLess(x0, 30)
        self.assertTrue(30 < x1 - x0 < 70 and 30 < y1 - y0 < 70, (x0, y0, x1, y1))

    def test_small_sizes_show_the_letter_large_the_word(self):
        for s in make_icon.SIZES:
            x0, y0, x1, y1 = blue_box(self.icons[s])
            ratio = (x1 - x0) / (y1 - y0)
            if s < 256:
                self.assertTrue(0.6 < ratio < 1.5, f"{s}: one letter, {ratio:.2f}")
                self.assertGreater(y1 - y0, s * 0.3, f"{s}: the letter is large")
            else:
                self.assertGreater(ratio, 2.5, f"{s}: the whole word, {ratio:.2f}")

    def test_two_usage_bars_under_the_logo(self):
        for s, img in self.icons.items():
            g, a = rows_of(img, GREEN), rows_of(img, AMBER)
            self.assertTrue(g and a, f"{s}: both bars drawn")
            self.assertLess(max(g), min(a), f"{s}: green above amber")
            self.assertLessEqual(blue_box(img)[3], min(g), f"{s}: bars under the logo")
            self.assertGreaterEqual(len(g), 2 if s < 64 else s // 12, f"{s}: bar height")

    def test_each_bar_is_partly_filled(self):
        for s, img in self.icons.items():
            px = img.load()
            for want, part in ((GREEN, 0.60), (AMBER, 0.85)):
                y = rows_of(img, want)[0]
                xs = [x for x in range(s) if near(px[x, y], want)]
                grey = [x for x in range(max(xs) + 1, s) if px[x, y][3] > 200 and 180 < px[x, y][0] < 240
                        and abs(px[x, y][0] - px[x, y][2]) < 12]
                self.assertTrue(grey, f"{s}: grey rest after {want}")
                filled = (max(xs) + 1 - min(xs)) / (max(grey) + 1 - min(xs))
                self.assertAlmostEqual(filled, part, delta=0.15, msg=f"{s} {want}")

    def test_the_logo_blue_is_kept(self):
        img = self.icons[256]
        x0, y0, x1, y1 = blue_box(img)
        px = img.load()
        near = any(all(abs(a - b) < 24 for a, b in zip(px[x, y][:3], BLUE))
                   for y in range(y0, y1) for x in range(x0, x1))
        self.assertTrue(near)

    def test_writes_the_ico_and_the_page_icon(self):
        with tempfile.TemporaryDirectory() as d:
            ico, png = Path(d) / "a.ico", Path(d) / "a.png"
            make_icon.write(self.icons, ico, png)
            with Image.open(ico) as got:
                self.assertEqual(sorted(s[0] for s in got.info["sizes"]), make_icon.SIZES)
            with Image.open(png) as p:
                self.assertEqual(p.size, (64, 64))


if __name__ == "__main__":
    unittest.main()
