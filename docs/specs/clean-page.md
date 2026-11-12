# A tidy page, glass look (spec)

Asked 2026-11-12: "Mấy cái này không để cho end user nhìn thấy, phải ấn nút mới xuất hiện.
Cần phần mềm nó gọn gàng. hiệu ứng glassy, morphism". The user named three things:
1. an account's raw error ("refresh failed (400): Refresh token not found or invalid — /login
   this account again");
2. the Settings card with its "Refresh now" and "Check for new login" buttons;
3. the "Usage log · 22,658 lines since 28 Oct" row.

## What the page shows by default

- **Account error: a short label in plain words, one sentence on a click.** Then asked
  (same day, of "Anthropic asked to slow down (HTTP 429); next check in 5 min"): "cần viết
  bằng tiếng người". The card never shows the raw text (no HTTP codes, no "refresh failed");
  that stays in Diagnostics. It shows one small button, and its click opens one plain
  sentence under it (`aria-expanded`); clicking again closes it:

  | the error holds | label | sentence |
  |---|---|---|
  | `429`, "slow down" | ⏳ Paused for a while | Anthropic asked us to check less often. The figures shown stay as they were; the next check is in N min (N from the text). |
  | `/login`, "refresh token", "no refresh token" | ⚠ Sign in again | This account's sign-in has ended. Sign in once more to keep seeing its usage and switching to it. + a **Sign in** button (`data-signin`, its email) |
  | "renews", "token expired" | ⏳ Waiting for Claude Code | The sign-in needs renewing; Claude Code does it the next time you use it on this account. |
  | `HTTP 5xx` | ⚠ Anthropic is busy | Anthropic's servers had a problem. It tries again by itself. |
  | "dns", "connect", "timed out", "network" | ⚠ No connection | Couldn't reach Anthropic. Check the internet connection; it tries again by itself. |
  | anything else | ⚠ Can't check now | Couldn't check this account just now. It tries again by itself; details under Settings → Advanced → Diagnostics. |

  The open state survives the page's redraws (every few seconds) for that account, and is
  forgotten on reload.
- **Settings: behind a button.** The Settings card is hidden (`hidden`) when the page opens.
  A "⚙ Settings" button in the top bar (`aria-controls`, `aria-expanded`) shows it and
  scrolls to it; pressing it again hides it. Not remembered: every visit starts tidy.
- **Inside Settings, at first:** Exit, the auto-switch settings and the checkboxes, Updates.
- **Advanced (closed `<details>`, "Advanced"):** "Refresh now", "Check for new login", the
  Usage log row, the long notes on adding accounts and the check pace, the event list and
  Diagnostics. Accounts are picked up by themselves within ~10 s and checked on their own, so
  these are for troubleshooting.

Nothing else changes: same ids, same routes, same behaviour of each control.

## Glass look

Recipe and the projects it comes from: [../SO_SANH_GLASS.md](../SO_SANH_GLASS.md).
- A separate stylesheet `glass.css`, linked after `style.css` on the page and Help: only
  overrides, so `style.css` keeps its layout.
- The page background: near black (asked the same day, "cho màu đen sâu hơn": `--bg` grey
  level ≤ 10, the solid card ≤ 22, the glass tint from it), with soft fixed blobs in the app's
  colours, fainter so the black reads, not animated.
- Top-level cards and the top bar: translucent, `backdrop-filter` blur, a 1 px light border, a
  soft inner highlight and outer shadow. Cards inside cards (logins, panels): lighter tint,
  no second blur (cost: ~10 cards with charts).
- `@supports not (backdrop-filter…)`: solid cards as before. `prefers-reduced-transparency:
  reduce`: solid cards too. Text keeps its contrast (colours unchanged on top of the glass).

## Parts

- `js/tidy.js` (loaded before `card.js`): `errorLabel(text)`, `errBadge(a)`, and the click
  handling for `[data-errmore]`, `#settingsBtn`.
- `card.js` draws `errBadge(a)` where it drew the raw error.
- `index.html`: `#settingsBtn` in the top bar; `#settingsCard` with `hidden`; `<details
  id="advanced">` holding the parts listed above.

## Tests (written first): `src/web_tidy_tests.rs`

- The top bar has `#settingsBtn` controlling `#settingsCard`, which is `hidden`.
- `#refresh`, `#import`, `#logRow`, `#events`, `#diag` sit inside `#advanced`, a closed
  `<details>` inside `#settingsCard`; Exit and Updates do not.
- `card.js` no longer prints `a.error` itself; it calls `errBadge(a)`. `tidy.js` has the
  table's labels and sentences, never draws the raw text, draws a `data-errmore` button with
  `aria-expanded` and a hidden detail, offers `data-signin` for a sign-in, and keeps the open
  set across redraws.
- `glass.css` is served, linked after `style.css` on both pages, and has `backdrop-filter`,
  an `@supports` fallback and a `prefers-reduced-transparency` fallback.

## Done when

Tests green, files ≤200 lines; in the browser: no raw error, no Settings card and no Usage
log row on first load; each opens with its button; glass measured (computed
`backdrop-filter` on a card is not `none`).
