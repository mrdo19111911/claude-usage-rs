# Wording review: what an office user sees

Read-only review of every string a user can see in SCAS: the widget (gadget), the tray menu, the dashboard, the help page, and the messages the server sends to the page. Readers are office staff, not developers. Rewrites stay in English and are about the same length as the current text. Widget text has to fit about 300 px.

Levels: **H** = likely misread or not understood at all; **M** = understood with effort, or the same thing has two names; **L** = polish.

Note found while reviewing: `src/tray/sections.rs:149-153` already has a test that expects `USED_UP == "USED UP · AVAILABLE AT"` and `available_at(i64::MAX, 0) == "not known"`. The code still draws `"FULL · FREE AGAIN"` and `"?"` (lines 129, 134), and neither `USED_UP` nor `available_at` exists yet, so the test module will not compile until that change goes in.

## 1. Gadget (widget window, tray tooltip, widget dialogs)

| Lvl | file:line | Current text | Why it confuses | Rewrite |
|---|---|---|---|---|
| H | src/tray/sections.rs:129 | `FULL · FREE AGAIN` | "Full" of what? "Free" can read as "costs nothing". The owner already asked for this change. | `USED UP · AVAILABLE AT` |
| H | src/tray/sections.rs:134 | `?` (reset time unknown) | A bare "?" looks like a bug. | `not known` |
| H | src/tray/sections.rs:113 | `26 · 74%` | Two numbers, only one with a %, and nothing says which is 5-hour and which is weekly. | `26% · 74%`, plus a small right-aligned `5-hour · weekly` in the SWITCH TO title row |
| H | src/tray/buttons.rs:314, src/tray/sections.rs:24,26 | chip/button `CLI` | "CLI" is developer jargon. The product is called Claude Code. | `Code` (next to `Desktop`) |
| M | src/tray/sections.rs:101 | `SWITCH TO` | Fine as a verb, but it doesn't say these accounts still have room left. | `AVAILABLE · SWITCH TO` |
| M | src/tray/sections.rs:74 | `2h05m · 16:30` | Two times with no verb. The user can't tell whether this is time used, time left, or a reset time. | `resets in 2h05m · 16:30` (if too wide: `in 2h05m · 16:30`) |
| M | src/tray/sections.rs:76 | `not started yet` | Sounds like the account or app hasn't started. It actually means this limit has no use yet. | `no use yet` |
| M | src/tray/sections.rs:65 | `No figures yet` | "Figures" is vague. | `Usage not checked yet` |
| M | src/tray/sections.rs:20-31 | (no title above the in-use cards) | Every other block has a title. The top cards don't say they are the accounts in use now. | Title `IN USE NOW` (same words as the dashboard) |
| M | src/tray/icon.rs:17 | `Claude CLI: bob\n5-hour 26% · weekly 74%` | "CLI" again, and nothing says whether 26% is used or left. | `Claude Code: bob\n5-hour 26% used · weekly 74% used` |
| M | src/tray/buttons.rs:338 | `Claude Desktop will close and open again on {email}. Running tasks stop. Go on?` | "Open again on" is unusual English. "Running tasks" is vague. | `Claude Desktop will restart signed in as {email}. Any reply in progress is cut off. Continue?` |
| M | src/tray/events.rs:92 | `Claude Desktop is switching accounts. The dashboard will close as soon as it is done.` | The user chose Exit on the app, but the message talks about "the dashboard". | `Claude Desktop is switching accounts. Smartlog Claude Auto Switch will close when it is done.` |
| L | src/tray/layout.rs:9-12 | `· 3 min ago` | No subject. | `· updated 3 min ago` (or keep it if too wide) |
| L | src/tray/layout.rs:60, src/tray/mod.rs:159, src/tray/icon.rs:19 | `Claude usage` | A third name for the app (also "Smartlog Claude Auto Switch" and "dashboard"). | Header can stay `Claude usage`; window and tooltip titles: `Smartlog Claude Auto Switch` |
| L | src/tray/buttons.rs:352, src/tray/events.rs:94 | message box title `Claude usage` | Same as above. | `Smartlog Claude Auto Switch` |
| L | src/tray/sections.rs:69 | `5-hour` / `Weekly` | OK, but alone "5-hour" could be read as a duration. | Keep. The help page explains it. |
| L | src/tray/sections.rs:80-83 | light tick on the bar (no text) | The pace tick is explained only in Help. | Keep. Maybe add a line to the first-run help. |

## 2. Tray menu

| Lvl | file:line | Current text | Why it confuses | Rewrite |
|---|---|---|---|---|
| M | src/tray/menu.rs:416 | `Pin widget on the desktop` | Help says "Pin widget". "On the desktop" can be confused with Claude Desktop. | `Keep widget on screen` (and use the same words in help.html:60) |
| L | src/tray/menu.rs:418 | `Exit` | The user doesn't learn that auto-switch stops. | `Exit (stops auto-switch)` |
| L | src/tray/menu.rs:370 | `Could not check for updates.` | No next step. | `Could not check for updates. Check the internet and try again.` |
| L | src/tray/menu.rs:455,460 | raw update errors shown as-is (see section 5, update/install.rs) | Technical text in a dialog. | Fix at the source (section 5) |

## 3. Dashboard (index.html and js/*.js)

| Lvl | file:line | Current text | Why it confuses | Rewrite |
|---|---|---|---|---|
| H | src/web/index.html:36 | `+ Add account · Claude CLI` | "CLI" is jargon. The middle dot reads as two separate things. | `+ Add account for Claude Code` |
| H | src/web/index.html:37 | `+ Add account · Claude Desktop` | Same middle dot. | `+ Add account for Claude Desktop` |
| H | src/web/index.html:43 | long note: `...Their <b>auto-switch</b> switches: ... Charts: each window from its start to its reset; the dotted diagonal is an even pace.` | One paragraph explains five things. "Their auto-switch switches", "each window" and "even pace" are internal terms. | Split it in three. 1) `Claude Code (terminal and VS Code) switches at once. Claude Desktop restarts on the new account; any reply in progress is cut off.` 2) `"Allow auto-switch" lets the app move to this account when the one in use is nearly used up.` 3) `Charts show use from the start of each limit to its reset; the dotted line is steady use.` |
| H | src/web/index.html:102-104 | `Claude CLI at [95] % 5-hour` · `[99] % weekly` · `Claude Desktop at 98% of either` | Reads like broken phrases. "At" doesn't say "switch when used reaches". | `Switch Claude Code when 5-hour use reaches [95]% or weekly use reaches [99]%` · `Claude Desktop switches at 98% of either (it restarts)` |
| H | src/web/index.html:107 | `Use the account whose weekly limit resets soonest first (switches before a limit when another account's quota would expire unused)` | "Quota would expire unused" and "switches before a limit" are hard to follow. | `Use up first the account whose weekly limit resets soonest, so no weekly allowance goes to waste (may switch before a limit)` |
| H | src/web/index.html:131 | `Checks: every account at that pace; from 70% used twice as often, from 85% four times, from 95% eight times (at most every 15 s).` | "At that pace" points to nothing. The rule disagrees with help.html:55 (every 10 min, every minute from 90%). | Pick one rule and state it once: `Usage is checked every 10 min, more often near a limit (every minute from 90%). A used-up account is checked again after it resets.` |
| H | src/web/js/rows.js:101 | `3 ready now · weekly headroom 230% (≈ 2.3 accounts)` | "Headroom" is jargon. 230% looks impossible. | `3 accounts ready · weekly use left: about 2.3 accounts' worth` |
| H | src/web/js/sides.js:122 | ` · 5h 26% · week 74%` (switch dropdown) | "5h" is an abbreviation that appears nowhere else. | ` · 5-hour 26% · weekly 74%` |
| H | src/web/js/card.js:216 | toggle label `auto-switch` on each sign-in | Reads like the global Auto-switch setting. It actually means "auto-switch may move to this account". | `allow auto-switch` |
| H | src/web/js/base.js:41 | status `Active` | The same card says `✓ In use`; the gadget and panels say "In use". | `In use` |
| H | src/web/js/main.js:83 | `Dashboard stopped` / `Start scas.exe again to bring it back.` | Office staff won't find "scas.exe". | `App stopped` / `Open Smartlog Claude Auto Switch from the Start menu to start it again.` |
| M | src/web/index.html:22 | `Two separate logins: switching one never changes the other.` | "Login" vs "sign-in". The user can't tell which two. | `Claude Code and Claude Desktop sign in separately: switching one never changes the other.` |
| M | src/web/index.html:110 | `Show every account's Code sessions in Claude Desktop (when switching Desktop)` | "Code sessions" is an internal term. | `Keep the same Claude Code chat list in Claude Desktop on every account` |
| M | src/web/index.html:113 | `Renew saved Claude Desktop sign-ins at 4:00 (switches Desktop for a few minutes, then back)` | Doesn't say why. "4:00" could be a.m. or p.m. | `Keep saved Claude Desktop sign-ins from expiring (at 4:00 a.m. it switches Desktop briefly, then back)` |
| M | src/web/index.html:125 | `Check for new login` | Unclear what it looks for. | `Look for a newly signed-in account` |
| M | src/web/index.html:124 | `Refresh now` | Each card's button says `Check now`, a different verb for the same action. | `Check all now` |
| M | src/web/js/base.js:37 | `Waiting` | Waiting for what? | `Checking usage…` |
| M | src/web/js/base.js:36, card.js:298, chart.js:96 | `No figures yet` / `Waiting for figures` | Three wordings for one state. "Figures" is vague. | `Usage not checked yet` everywhere |
| M | src/web/js/base.js:45 | `Off` | Looks like the account is off. It actually means auto-switch skips it. | `Skipped by auto-switch` (short: `Auto-switch off`) |
| M | src/web/js/base.js:58, card.js:272 | `· not started yet` / `not started` | Same as the gadget; two variants. | `· no use yet` |
| M | src/web/js/sides.js:169 | ` · off` (dropdown) | Off what? | ` · auto-switch off` |
| M | src/web/js/sides.js:170 | `Auto-switch at 95% 5-hour · 99% weekly` | Word order is hard to read. | `Auto-switch at 95% (5-hour) or 99% (weekly)` |
| M | src/web/js/sides.js:181,201 | `Its own sign-in` | "Its" has no clear reference. | `Signs in separately` |
| M | src/web/js/sides.js:192-193 | `Auto-switch at 98% · switching restarts it` | "It" has no clear reference. | `Auto-switch at 98% · switching restarts the app` |
| M | src/web/js/card.js:235,250 | `Use here` | "Here" could mean this page. | `Switch to this` |
| M | src/web/js/card.js:246 | `Use here (first time)` / `after its first sign-in here` | The first time is really a sign-in. | `Sign in here` / `available after first sign-in` |
| M | src/web/js/card.js:234 | `no login yet` | "Login" vs "sign-in". | `not signed in yet` |
| M | src/web/js/card.js:244 | `Not on this computer` | Not clear what is missing. | `Claude Desktop not installed` |
| M | src/web/js/card.js:266 | `No Claude CLI login yet: sign in to see its figures` | "CLI", "login", "figures" in one line. | `Not signed in to Claude Code yet: sign in to see its usage` |
| M | src/web/js/main.js:126 | `Claude Desktop will close and open again on {x}. Running tasks stop. Go on?` | Same as the gadget dialog. | `Claude Desktop will restart signed in as {x}. Any reply in progress is cut off. Continue?` |
| M | src/web/js/main.js:134 | `Remove its Claude CLI login and its saved Claude Desktop sign-in from the dashboard?` | The user may fear the Claude account itself gets deleted. | `Remove this account from the app? (Your Claude account is not deleted.)` |
| M | src/web/js/main.js:81 | `Stop the dashboard? Checks and auto-switch stop until you start it again.` | It closes the whole app, not just a page. | `Close Smartlog Claude Auto Switch? Usage checks and auto-switch stop until you open it again.` |
| M | src/web/js/chart.js:55-56 | `At the last 20 min's pace` / `Expected limit at 16:30, in 1h20m` / `Expected ~80% at reset (18:00)` | Clipped, telegraphic phrases. | `If you keep this pace (last 20 min):` / `used up at 16:30 (in 1h20m)` / `about 80% used when it resets (18:00)` |
| M | src/web/js/chart.js:103 | `even pace` | Jargon. | `steady use` |
| M | src/web/js/chart.js:127 | `Recording since 14:05` | Sounds like audio or video recording. | `Tracked since 14:05` |
| M | src/web/index.html:60, gemini.js:67 | `what you send through cgemini` / `cgemini stops working` | "cgemini" is a command name the user never typed. | `what you send to Gemini` / `Claude Code on Gemini stops working` |
| M | src/web/js/gemini.js:30 | `Small side jobs use: {model}` | Unclear. | `Quick tasks use: {model}` |
| L | src/web/index.html:133 | `Diagnostics (no tokens shown)` | "Tokens" is jargon (and also means usage tokens). | `Technical details for support (no passwords shown)` |
| L | src/web/js/card.js:228 | `Sign-in until Nov 30 · 18 days` | Could read as a subscription end date. | `Saved sign-in lasts until Nov 30 (18 days)` |
| L | src/web/js/card.js:262 | ✕ tooltip `Remove` | Too short. | `Remove from this app` |
| L | src/web/js/sides.js:160 | `Both on this account` | Both what? | `Both apps use this account` |
| L | src/web/js/usage_log.js:262 | `1,234 lines since 28 Oct` | "Lines" is a file term. | `1,234 records since 28 Oct` |
| L | src/web/js/update.js:240 | `The dashboard closes and opens again in a moment.` | "Dashboard" used for the app. | `The app restarts in a moment.` |
| L | src/web/js/settings.js:168-171 | Diagnostics text (`Last poll`, `[ACTIVE]`, `Anthropic said:`) | Technical, but kept under Advanced for support. | Keep. Maybe `Last check` instead of `Last poll`. |
| L | src/web/js/rows.js:107, main.js:129 | `click Continue with Google there` | Assumes a Google sign-in. | `sign in there with {email}` |

## 4. Help page (help.html)

| Lvl | file:line | Current text | Why it confuses | Rewrite |
|---|---|---|---|---|
| H | src/web/help.html:54,56 | Q `What does 429 mean?` / `HTTP 429 means Anthropic asked us to slow down.` | The page no longer shows "429". It shows `⏳ Paused for a while`. | Q `What does "Paused for a while" mean?` A `Anthropic asked the app to check less often. It waits 5 minutes, then longer each time (up to 1 hour). The figures shown stay as they were.` |
| H | src/web/help.html:40 vs :55 | `1% of a week is about five times as much use as 1% of five hours` vs `1% of a week is about 6.7% of five hours` | The two statements contradict each other. | Keep one, or drop the ratio: `The weekly limit is much larger, so 1% of it is more use than 1% of the 5-hour limit.` |
| H | src/web/help.html:55 vs index.html:131 | two different check-frequency rules | The user can't tell which one is true. | One rule, the same text in both places (see dashboard row) |
| H | src/web/help.html:34 | `Claude CLI is Claude Code in a terminal and in VS Code` | This is where the two names start. Users know the product as "Claude Code". | `Claude Code is the Claude app in a terminal and in VS Code; they share one sign-in.` Then use "Claude Code" everywhere. |
| H | src/web/help.html:40 | one paragraph about auto-switch: thresholds, Desktop rules, the per-account switch, 10-minute gap | Too much in one block, with internal phrases ("Accounts switched off are skipped"). | A short list: `Claude Code: switches at 95% (5-hour) or 99% (weekly).` / `Claude Desktop: switches at 98%, only to accounts with "allow auto-switch" on, at most every 10 min; it restarts.` / `Accounts with "allow auto-switch" off are skipped.` |
| H | src/web/help.html:41 | `may switch before a limit when another account's quota would expire unused ... keep the old rule (most room left)` | "Quota would expire unused" and "the old rule" mean nothing to a new user. | `Weekly use you don't spend before the reset is lost, so the app uses first the account that resets soonest. Turn it off to always use the account with the most left.` |
| M | src/web/help.html:25 | `how much of the window's time has passed ... faster than the window allows` | "Window" is a technical term (a time period). | `how much of the 5-hour (or weekly) period has passed` |
| M | src/web/help.html:46 | `The first message ... rebuilds its context, which uses some quota.` | "Rebuilds its context" and "quota". | `The first message after a switch uses a little more of the limit than usual.` |
| M | src/web/help.html:61 | `switch that side to it` | "Side" is an internal term. | `switch Claude Code (or Claude Desktop) to that account` |
| M | src/web/help.html:71 | Gemini setup in one very long paragraph | Hard to follow. The dialog already shows the steps. | Short list of 4 steps + `Then press Use this.` |
| M | src/web/help.html:80 | `Claude Code reports a rate limit ... Only Flash models are free` | "Rate limit" and "Flash" are jargon. | `Claude Code says it hit a limit: wait and try again. Only models with "Flash" in the name are free.` |
| L | src/web/help.html:60 | `Pin widget` | The menu says "Pin widget on the desktop". | Match the menu (`Keep widget on screen`) |
| L | src/web/help.html:65 | `checks GitHub releases` | Not needed. | `checks for a new version` |
| L | src/web/help.html:91 | `Sign-in tokens` | Jargon. | `Your sign-ins` |
| L | src/web/help.html:24 | `how much is % left` | Odd wording. | `how much is left` |

## 5. Messages (events list in Settings, alerts, and error dialogs from the server)

| Lvl | file:line | Current text | Why it confuses | Rewrite |
|---|---|---|---|---|
| H | src/switch.rs:127 (+ server.rs:155, rank.rs:112) | `Switched a@x → b@x (manual)` / `(auto: 5-hour 95%)` | "auto:" and "manual" are internal tags. The arrow is terse. It doesn't say which app switched. | `Claude Code: switched from a@x to b@x (by you)` / `(5-hour limit at 95%)` |
| H | src/rank.rs:129 | `auto: weekly resets in 30h, sooner` | Can't be read on its own. | `b@x's weekly limit resets sooner (in 30 h), so it is used first` |
| M | src/switch.rs:141, src/desktop_auto.rs:55 | `a@x hit 5-hour 95% but no other account has room` | "Hit 5-hour 95%" and "room". | `a@x is at 95% of its 5-hour limit, and no other account has any left` |
| M | src/desktop_auto.rs:62 | `Claude Desktop: auto-switch (5-hour 98%)` | Clipped. | `Claude Desktop: switching automatically (5-hour limit at 98%)` |
| M | src/switch.rs:66 | `Claude Desktop: Code sessions shared (3 added, 1 updated, 0 removed)` | "Code sessions shared" is an internal term. | `Claude Desktop: Claude Code chat list copied (3 new, 1 updated)` |
| M | src/switch.rs:67, desktop/mod.rs:71-102, desktop/sessions_io.rs | `Code sessions not shared: C:\...\x.json: Access is denied` / `config.json: ...` / `backup: ...` | Raw file paths and OS errors. | `Claude Desktop: the chat list could not be copied (a file was locked). It tries again next switch.` Keep the raw text for Diagnostics. |
| M | src/switch.rs:50 | `This dashboard was started from inside Claude, so closing Claude would stop it half-way.` | "Started from inside Claude" and "half-way" are unclear. | `This app was opened by Claude, so restarting Claude Desktop would close it too. Open it from the Start menu instead.` |
| M | src/switch.rs:117 | `Claude Code's current login is not identified yet; try again in a few seconds` | "Not identified". | `Still reading which account Claude Code uses; try again in a few seconds` |
| M | src/anthropic.rs:136 | `could not check this login with Anthropic (...); not switched` | Wraps raw text such as `HTTP 401`. | `Could not reach Anthropic to check this account, so it was not switched. Try again shortly.` |
| M | src/anthropic.rs:138 | `this saved login belongs to another account; not switched` | Unclear what the user should do. | `This account's saved sign-in now belongs to a different account. Sign in to it again.` |
| M | src/anthropic.rs:156, poll.rs:132 | `Auto-switch failed: ... Anthropic asked to slow down (HTTP 429)` (events list) | The raw 429 is still visible in Settings → Advanced. | `Auto-switch waited: Anthropic asked the app to check less often` |
| M | src/signin.rs:43 | `Anthropic refused the sign-in (400): {up to 200 chars of raw body}` | Raw server body on the result page. | `Anthropic did not accept the sign-in. Close this tab and click + Add account again.` (raw text to Diagnostics) |
| M | src/state_load.rs:39 | `Saved accounts could not be read (...); the file is kept as {path}. Restore it after fixing it, with SCAS closed.` | Office staff cannot fix a JSON file. | `Your saved accounts could not be loaded. A copy was kept; please contact support (file: {path}).` |
| M | src/update/install.rs:128 | `The download does not match GitHub's SHA-256 digest; it was deleted.` | "SHA-256 digest" is jargon. | `The download was damaged and has been deleted. Please try again.` |
| M | src/update/install.rs:125 | `The download has {got} bytes, GitHub says {n}; it was deleted.` | Byte counts. | `The download was incomplete and has been deleted. Please try again.` |
| M | src/update/install.rs:108 | `The download of X was sent somewhere other than GitHub.` | Alarming and unclear. | `The update came from an unexpected place and was not installed.` |
| M | src/gemini/setup.rs:57, gemini/path_win.rs:31-68 | `The cgemini folder could not be put on PATH: could not change PATH (error 5)` | "PATH" and "error 5" are developer terms. | `Could not finish setting up Claude Code on Gemini (Windows blocked it). Restart the app and try again.` |
| M | src/gemini/stream_message.rs:24, gemini/upstream.rs:66 | `Gemini stopped with {reason}` / `Google refused the key (400): {msg}` | Raw codes. | `Gemini stopped answering ({reason})` / `Google did not accept this access code` |
| L | src/server.rs:126-165, switch.rs:90,97, desktop_auto.rs:76 | `not found` / `enabled required` / `name required` (alert) | Developer text, though rarely seen. | `This account is no longer in the list. Reload the page.` |
| L | src/signin.rs:87 | `Added a@x (signed in in the browser)` | "In in". | `Added a@x` |
| L | src/sync.rs:70 | `Added a@x (new Claude Code login)` | Fine. "Login" vs "sign-in". | `Added a@x (new Claude Code sign-in)` |
| L | src/signin.rs:100 | page title `Account added` even when the message is `Signed in again to a@x` | Title and message disagree. | Title `Signed in` when the account was already there |
| L | src/switch.rs:73 | `Claude Desktop: signed out here; sign in to the account to add` | Fine. "Here" is vague. | `Claude Desktop is signed out: sign in there with the account to add` |
| L | src/desktop/win.rs:159,181 | `Claude Desktop did not close` / `could not start Claude Desktop; start it from the Start menu` | Fine. Lowercase start. | Capitalise; keep |
| L | src/update/github.rs:42,99 | `The latest release's tag (...) is not a version.` / `GitHub's answer could not be read` | Technical. | `Could not read the update information. Try again later.` |
| L | src/gemini/api.rs:59, gemini/open.rs:37 | `on: true or false` / `Only on Windows` | Developer text / too short. | `Something went wrong; reload the page.` / `Claude Code on Gemini works on Windows only.` |

Implementation note: `tidy.js:17` turns an error into "Sign in again" when the raw text contains `/login` or `refresh token` (it comes from anthropic.rs:42 `— /login this account again`). If that Rust text is reworded, update the match in `errorLabel` at the same time.

## Inconsistencies across screens

| Same thing | Names in use now (where) | Use this one |
|---|---|---|
| Claude Code (terminal + VS Code) | "Claude CLI" (dashboard, help, icon tooltip), "CLI" (widget buttons), "Claude Code" (diagnostics, Gemini card, events, errors), "terminal and VS Code" | **Claude Code** (widget button: **Code**) |
| What the user signs in with | "login" (index.html:22,125, card.js:234, events), "sign-in" (Desktop, help), "account" | **account** for the Claude account; **sign-in** for the saved credentials; drop "login" |
| Account in use | "Active" (status), "✓ In use" (card), "In use now" (panel), green dot | **In use** |
| Usage data | "figures", "No figures yet", "Waiting", "Waiting for figures", "usage" | **usage** / **Usage not checked yet** |
| Checking usage | "Refresh now", "Check now", "Check for new login", "Last poll" | **Check now** (one account), **Check all now** (all) |
| The program | "Smartlog Claude Auto Switch", "SCAS", "scas.exe", "Claude usage", "the dashboard", "the app" | **Smartlog Claude Auto Switch** (short: **the app**); "dashboard" only for the web page |
| Small floating window | "widget" (menu, help); the owner calls it "gadget"; "Pin widget" vs "Pin widget on the desktop" | **widget**, menu item **Keep widget on screen** |
| A limit at 100% | "FULL", "limit reached", "used up", "at a 100% limit", "full" (code) | **used up** |
| When it can be used again | "FREE AGAIN", "resets", "renews", "reset" | **resets** (and **available at** for used-up accounts) |
| Time period of a limit | "window", "limit", "quota", "headroom", "room" | **5-hour limit** / **weekly limit**; "left" for what remains |
| Per-account permission | "auto-switch" toggle, "Off", " · off", "switched off", "Desktop auto-switch is on" | **allow auto-switch** (toggle), **skipped by auto-switch** (status) |
| Limit not yet in use | "not started yet", "not started", "Not started yet" (help) | **no use yet** |
| Limit names | "5h", "5-hour", "five hours"; "week", "weekly", "Weekly" | **5-hour**, **weekly** |
| Desktop switch | "close and open again", "closes and opens again on", "restarts", "Running tasks stop", "a reply under way is cut off" | **restarts** ... **any reply in progress is cut off** |
| Gemini key | "access code", "API key", "key", "Gemini account" | **access code** (once: "Google calls it an API key") |
| Numbers that disagree | Check frequency (index.html:131 vs help.html:55); "5x" vs "6.7%" (help.html:40 vs :55) | One rule, the same words in both places |

## Names to use everywhere

- **Claude Code** for the terminal / VS Code app (widget button: **Code**). Never "CLI".
- **Claude Desktop** for the desktop app (widget button: **Desktop**).
- **account** for a Claude account; **sign-in** for what is saved; no "login".
- **In use** for the account an app uses now.
- **5-hour limit**, **weekly limit**; never "5h", "window", "quota", "headroom".
- **used up** for a limit at 100%; **available at HH:MM** for when it is usable again; **resets** for the reset.
- **no use yet** for a limit with no reset time.
- **Auto-switch** for the global setting; **allow auto-switch** for the per-account toggle; **skipped by auto-switch** for its status.
- **Check now** (one account) / **Check all now** (all).
- **Smartlog Claude Auto Switch** / **the app** for the program; **dashboard** only for the web page; **widget** for the small window.
- **restarts** for what Claude Desktop does on a switch.
- **access code** for the Gemini key.
