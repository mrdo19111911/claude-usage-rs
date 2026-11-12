# Glassmorphism for the dashboard: comparison and recipe

Researched 2026-11-12. All values in the table were read from each project's own CSS or source
(fetched raw from GitHub or the site's JS bundle), not taken from blog summaries.

## Comparison

| Project + URL | Method (values as written in their source) | Fit for our dark dashboard | Cost / perf |
|---|---|---|---|
| css.glass generator (Hype4) https://css.glass | One rule: `background: rgba(255,255,255,0.2)`, `border-radius: 16px`, `box-shadow: 0 4px 30px rgba(0,0,0,0.1)`, `backdrop-filter: blur(5px)` + `-webkit-` copy, `border: 1px solid rgba(255,255,255,0.3)`. Slider limits: blur max 20, alpha 0..1 | Good base shape, but the white 20% fill and 30% border are made for photo backgrounds; on #1a1a1b they look milky. No inset highlight, no fallback | 1 blur layer per element, nothing else. Cheapest |
| glasscn-ui (shadcn glass variant) https://github.com/itsjavi/glasscn-ui | Tailwind recipe `src/recipes/glass-cva.ts`: glass = `bg-white/70 dark:bg-gray-950/50` + `border border-border`; default `backdrop-blur-md` (Tailwind 3 = 12px); options sm..3xl. Repo uses gray = slate, so dark glass = `rgb(2 6 23 / .5)`, dark border = slate-700 `#334155`. Inset glow was tried and commented out ("this overrides the outset shadow"). Demo bg = photo, `background-attachment: fixed`, animated 40s | Best fit: a dark tinted glass at 50% with a solid-ish border, made for text-heavy components (cards, buttons, inputs) | Blur on every component by default (cards AND buttons inside them), so nested blur. We should not copy that part |
| liquid-glass-effect-macos (826 stars) https://github.com/lucasromerodb/liquid-glass-effect-macos | 4 stacked layers: wrapper `box-shadow: 0 6px 6px rgba(0,0,0,.2), 0 0 20px rgba(0,0,0,.1)`; effect layer `backdrop-filter: blur(3px)` + `filter: url(#glass-distortion)` (SVG feTurbulence + displacement); tint `rgba(255,255,255,.25)`; shine `inset 2px 2px 1px 0 rgba(255,255,255,.5), inset -1px -1px 1px 1px rgba(255,255,255,.5)` | The shine (two inset shadows) is the useful idea. SVG distortion is eye candy, hurts chart readability | Expensive: SVG filter per element, 3 extra elements, Chromium-only refraction. Not for 10 cards |
| "CSS-only glass, liquid glass lite" (vilvarion pen, gist tuliopc23) https://gist.github.com/tuliopc23/6cabf60c3b306a7fd585c6cc737e2392 | Single element: `background-color: oklch(1 0 0 / 10%)`, `backdrop-filter: blur(0.25em) brightness(1.2) saturate(1.2)`, shadows: `0 .1em .25em 0 oklch(0 0 0/15%)`, `0 0 1em 0 oklch(0 0 0/12.5%)`, `inset .075em .075em .05em 0 oklch(1 0 0/40%)`, `inset -.075em -.075em .05em 0 oklch(1 0 0/40%)`, `inset 0 0 3em .5em oklch(0 0 0/20%)`. Author note: "Clean CSS and performant" | Good: one element, highlight from inset shadows only. Values need scaling down for dark UI | 1 blur per element; 5 shadows are cheap (painted once, not per frame) |

Supporting references (technique, not a project):
- Josh Comeau, "Next-level frosted glass" https://www.joshwcomeau.com/css/backdrop-filter/ :
  `backdrop-filter: blur(16px)`; example filters `blur(8px) brightness(90%) saturate(140%)`;
  base style "Very opaque, for unsupported browsers", glass only inside `@supports`.
- CSS Filter Effects 2, Backdrop Root https://drafts.csswg.org/filter-effects-2/ : an element with
  `backdrop-filter`, `filter`, `opacity < 1`, `mask`/`clip-path` or `mix-blend-mode` is a Backdrop Root, so a
  blurred child of a blurred card only sees the card's own content (spec cites "exponential" cost as the reason).
- MDN browser-compat-data: `backdrop-filter` Chrome 76+, Edge 79+, Firefox 103+, Safari 18 (9 with `-webkit-`).
  `prefers-reduced-transparency` Chrome/Edge 118+, Firefox behind a flag, Safari no. On Windows it follows
  Settings > Personalization > Colors > Transparency effects.
- Firefox bugs 1718471 / 1742043 report jank with backdrop-filter on many elements; Chromium handles it better.

## Recommended recipe

Decision: one blur layer per top-level card (about 10 layers, fine in Edge/Chrome), never on anything inside a
card. Shape from css.glass, dark tint from glasscn-ui, highlight from the liquid-glass shine, scaled for dark.

```css
:root {
  --page-bg: #141517;                       /* a bit darker than #1a1a1b so blobs read */
  --glass-bg: rgba(35, 42, 49, 0.55);       /* our card #232a31 at 55% (glasscn dark: 50%) */
  --glass-bg-solid: #232a31;                /* fallback, = today's card */
  --glass-nested: rgba(255, 255, 255, 0.04);/* inner card: lighter tint, NO blur */
  --glass-border: rgba(255, 255, 255, 0.10);/* css.glass uses .3 on photos; .10 on dark */
  --glass-blur: blur(12px) saturate(140%);  /* glasscn md = 12px; saturate from Comeau */
  --glass-highlight: inset 0 1px 0 rgba(255, 255, 255, 0.08),
                     inset 1px 0 0 rgba(255, 255, 255, 0.04);
  --glass-shadow: 0 4px 30px rgba(0, 0, 0, 0.35);  /* css.glass geometry, darker for dark bg */
  --radius: 16px;
  --text: #ececec; --text-muted: #a0a8b0;
}
body { background: var(--page-bg); color: var(--text); }
body::before {                               /* fixed, not animated, painted once */
  content: ""; position: fixed; inset: 0; z-index: -1; pointer-events: none;
  background:
    radial-gradient(40vw 40vw at 10% 0%,   rgba(43, 181, 168, 0.20), transparent 70%),  /* teal */
    radial-gradient(35vw 35vw at 95% 20%,  rgba(163, 124, 240, 0.18), transparent 70%), /* purple */
    radial-gradient(45vw 45vw at 50% 110%, rgba(63, 179, 79, 0.14), transparent 70%);   /* green */
}
.card {
  background: var(--glass-bg-solid);         /* default = fallback */
  border: 1px solid var(--glass-border); border-radius: var(--radius);
  box-shadow: var(--glass-shadow), var(--glass-highlight);
}
@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .card { background: var(--glass-bg);
          -webkit-backdrop-filter: var(--glass-blur); backdrop-filter: var(--glass-blur); }
}
.card .card, .card .sub {                    /* nested: lighter, no second blur */
  background: var(--glass-nested); border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 12px; box-shadow: var(--glass-highlight);
  backdrop-filter: none; -webkit-backdrop-filter: none;
}
.btn {
  background: rgba(255, 255, 255, 0.06); border: 1px solid var(--glass-border);
  border-radius: 10px; color: var(--text); box-shadow: var(--glass-highlight);
  transition: background-color .15s ease-out;              /* no blur on buttons */
}
.btn:hover { background: rgba(255, 255, 255, 0.10); }
.btn:focus-visible { outline: 2px solid #2bb5a8; outline-offset: 2px; }
.pill {                                       /* accent at low alpha, text in accent */
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  color: var(--accent); border-radius: 999px; padding: 2px 10px;
}
@media (prefers-reduced-transparency: reduce) {
  body::before { display: none; }
  .card { background: var(--glass-bg-solid); backdrop-filter: none; -webkit-backdrop-filter: none; }
  .card .card, .card .sub { background: #2a323a; }
}
```

Performance rules:
- No `backdrop-filter` inside a card (buttons, pills, nested cards, tooltips over charts). It is a Backdrop
  Root anyway, so the inner blur would only blur the card's own content.
- Do not put `opacity < 1`, `filter`, `mask` or `will-change: opacity` on a card's ancestor (e.g. a fade-in
  wrapper): it becomes a Backdrop Root and the card blur stops seeing the page background.
- Do not add `will-change` globally; do not animate blobs or card size (re-blur each frame). Use the fixed
  `::before` layer, not `background-attachment: fixed` (glasscn demo uses it; it repaints on scroll).
- Over smooth blobs a blur is nearly invisible; if a slow PC janks, setting `--glass-blur: none` keeps the look.

Text contrast (WCAG ratios computed for worst case: blob peak at 25% under 55% glass):
- `#ececec` on glass: 10.5 to 11.9 (plain #232a31: 12.3). Muted `#a0a8b0`: 5.1 to 5.8. Both pass AA.
- Accents as text on the purple-tinted worst spot: amber 7.3, teal 5.2, green 4.9, purple 4.2, red 3.4.
  Keep red #e5484d and purple #a37cf0 for chart strokes/fills (3:1 is enough for graphics); for small
  red or purple text use `#ff7a7e` (5.3) and `#b896f5` (5.5).
- Never put text directly on a blob without a card behind it.
