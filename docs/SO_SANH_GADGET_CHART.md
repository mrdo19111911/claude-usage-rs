# Comparison: drawing the small usage chart in the gadget (GDI / GDI+)

Goal: click a gadget row -> it expands to ~100 px and draws a 5-hour line and a weekly
line (0-100 %), a dashed threshold, a vertical reset mark, labels 0/50/100 and a few time
ticks. Dark background (#1b1f24), 100-200 % DPI, Rust + windows-sys 0.61, no new crates.
Data: `Vec<(time_ms, five_pct, week_pct)>`, irregular spacing, hundreds of points.

## Projects read (sources fetched 2026-11-12)

### 1. System Informer - `phlib/graph.c` (C, raw GDI + manual rasterizer)
https://github.com/winsiderss/systeminformer/blob/master/phlib/graph.c
- Does not use Polyline. It writes pixels straight into a 32bpp DIB section
  (`PhDrawGraphDirect`, l.2054: "Draws a graph directly to memory").
- Classic mode `PhpDrawGraphClassic` (l.208): one loop per pixel column, right to left,
  fixed `Step = 2` px per sample with a midpoint column in between (comment l.380-420);
  each column fills a vertical span low..high, so no gaps and no anti-aliasing.
- Smooth mode `PhpDrawGraphSmooth` (l.1686): its own anti-aliasing. 4 subsamples per
  column (`PH_GRAPH_SMOOTH_SUBSAMPLES 4`, l.933, used l.1822/1961) for the fill, and a
  distance-to-segment stroke coverage (`PhpGraphSmoothStrokeCoverage`, l.1494), blended
  in linear light (sRGB<->linear tables, l.1018-1048).
- Buffering: `PhpCreateBufferedContext` (l.2429) = `CreateCompatibleDC(NULL)` + DIB
  section; WM_PAINT does `BitBlt` of only `rcPaint` (l.3012). Fade-out uses `GdiAlphaBlend`.
- Downsampling: none; the data is a ring buffer with one sample per tick, sized to width.
- DPI: fonts cached per DPI (`CachedFontDpi`, l.2506); the graph itself is in device px.

### 2. Rainmeter - `Library/MeterLine.cpp` (C++, was GDI+, now Direct2D wrapper "Gfx")
https://github.com/rainmeter/rainmeter/blob/master/Library/MeterLine.cpp
- One history value per pixel: buffer size = meter width (l.36 `maxSize = ... m_W`).
- Builds one path per line and strokes it with anti-aliasing (`DrawGeometry`, l.295;
  `SetStrokeWidth`, l.308; line join miter/bevel/round, l.140-156).
- Keeps a `0.55f` "GDI+ compatibility" offset (l.299): half-pixel shift so a 1 px
  anti-aliased stroke lands on pixel centres instead of smearing across two rows.
- Grid lines: at most one per 4 px, y placed at `bottom - Y - 0.5f` (l.272-289).
- Lesson: AA vector stroke + half-pixel offset is what makes thin lines crisp.

### 3. TrafficMonitor - `TaskBarDlg.cpp` / `DrawCommon.cpp` (C++ MFC, plain GDI)
https://github.com/zhongyang219/TrafficMonitor
- Taskbar usage graph `TryDrawGraph` (TaskBarDlg.cpp l.1413-1437): one value per pixel
  column, right to left, one vertical `MoveTo/LineTo` per column (`DrawLine`,
  DrawCommon.cpp l.214-222). Area chart, so no AA needed.
- Dashed frame via `CreatePen(PS_DOT ...)` (DrawCommon.cpp l.157).
- Bitmaps scaled with `SetStretchBltMode(HALFTONE)` + `SetBrushOrg(0,0)` before
  `StretchBlt` (DrawCommon.cpp l.73-78) - the standard GDI downscale-with-averaging recipe.
- DPI: `DPI(px)` helpers scale every size from the window's monitor DPI (l.560-600).

### 4. lastwarhq-scanner - `src/ui/paint.rs` (Rust, GDI + GDI+ flat API, no crates)
https://github.com/lastwarhq-dev/lastwarhq-scanner/blob/main/src/ui/paint.rs
- Header (l.1-2): "shapes through GDI+ (smooth edges), text through GDI (ClearType)".
- Hand-declared `gdiplus.dll` imports (l.40-77): `GdiplusStartup`, `GdipCreateFromHDC`,
  `GdipSetSmoothingMode`, `GdipCreatePen1`, `GdipDrawLines`...
- `graphics()` helper (l.423-436): create Graphics on the existing HDC, set
  `SmoothingModeAntiAlias` + `PixelOffsetModeHalf`, run closure, delete. Lines drawn with
  `GdipDrawLines` on f32 points (l.553, 610); pen width `scale.max(1.0)` (l.484).
- DPI: every coordinate is `v * scale` in f32 (l.383), so AA handles fractional px.
- This is exactly our situation (Rust tray-style window, GDI text, `s` scale factor).

## Comparison

| Method | Used by | AA quality | Fit for our chart | Cost / complexity |
|---|---|---|---|---|
| A. GDI `Polyline`, pen width `round(s)` | (common, TrafficMonitor-style) | none: jaggy diagonals at 100 %, OK-ish at 200 % | works, looks cheap at 100 % | lowest: ~40 lines, already-enabled features |
| B. GDI Polyline into 2x memory DC, `StretchBlt` HALFTONE down | TrafficMonitor uses HALFTONE for bitmaps | fair (2x2 box filter, slightly soft) | OK; text must be drawn after, at 1x | low-mid: extra DC/bitmap, 4x pixels, ~70 lines |
| C. GDI+ flat API, AntiAlias + PixelOffsetHalf, `GdipDrawLines` | Rainmeter (older), lastwarhq (Rust) | good, sub-pixel exact at any DPI | best: f32 coords, dashed pen built in | mid: `GdiplusStartup` once + ~15 imports; feature `Win32_Graphics_GdiPlus` exists in windows-sys 0.61.2 (checked in local registry), gdiplus.dll ships with Windows |
| D. Own rasterizer into DIB bits (coverage + linear blend) | System Informer | excellent | overkill for one small chart | high: hundreds of lines + tests |
| E. Direct2D | Rainmeter (current) | excellent | too heavy (COM, render target, device loss) | high |

## Recommendation: C (GDI+ flat API through windows-sys), copy lastwarhq's pattern

- Add feature `"Win32_Graphics_GdiPlus"` to windows-sys (no new crate). Call
  `GdiplusStartup` once at app start, `GdiplusShutdown` on exit.
- Keep text in GDI (`DrawTextW`, ClearType) - GDI+ text is worse; this split is what
  lastwarhq does.
- Draw into the memory DC the gadget already uses (`src/tray/widget.rs` l.181-188:
  `CreateCompatibleDC` + `BitBlt`), so no flicker and no new buffering code.
- Fallback if GDI+ is ever unwanted: B (2x supersample + HALFTONE), never plain A.

### Downsampling (none of the projects need it: they store 1 sample/px; we do)
- Map time -> x in f32 over [now - window, now]. If points per pixel column > 1, keep for
  each column the first, min, max, last point (M4 aggregation), in time order; this keeps
  spikes and caps the polyline at ~4 x width points (~1200 at 300 px). Gaps longer than
  e.g. 3x the normal interval break the line (start a new `GdipDrawLines` call).

### Drawing steps (per expanded row, plot rect `r` in device px)
1. Fill row background (GDI `FillRect`).
2. Grid + axes with GDI, integer-aligned 1 px pens: horizontal lines at 0/50/100 %
   (dim, e.g. #2d333b), time ticks every N hours. Crisp because pixel-aligned.
3. Labels "0", "50", "100" and tick times with `DrawTextW` (font height `-round(9*s)`).
4. `GdipCreateFromHDC(mem_dc)`, `SmoothingModeAntiAlias`, `PixelOffsetModeHalf`.
5. Threshold: pen width `max(1, s)`, `GdipSetPenDashStyle(DashStyleDash)`, `GdipDrawLine`.
6. Reset mark: vertical dashed or solid line at x(reset_ms), accent colour, alpha ~70 %.
7. Weekly line first (thinner/dimmer, width `1.0*s`), then 5-hour line on top
   (width `1.5*s`, round join): `GdipDrawLines(g, pen, pts, n)` per contiguous run.
8. Optional: last point dot `GdipFillEllipse` radius `2*s`.
9. Delete pens and Graphics, then the existing `BitBlt` to the window.

### Modules + tests (SDD/TDD, each file < 200 lines)
- `chart_data.rs` (pure): time->x, pct->y, M4 downsampling, gap splitting -> `Vec<Vec<(f32,f32)>>`.
  Test with numbers: 1000 points into 300 columns gives <= 1200 points, keeps min/max.
- `gdip.rs`: startup/shutdown + RAII wrappers for Graphics/Pen (Drop deletes).
- `chart_paint.rs`: steps 1-9 using the two above. Test: render to a 32bpp DIB, check
  pixels at known (x,y) are the line colour, and that 100 % and 200 % scale both draw.
