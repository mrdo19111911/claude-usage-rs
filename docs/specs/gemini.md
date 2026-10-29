# "Claude shell, Gemini inside": the `cgemini` profile (spec)

Claude Code (the CLI) runs as usual, but its model is Gemini, reached with the user's own
Google AI Studio API key. Claude Code's documented `ANTHROPIC_BASE_URL` points it at this
app, which translates Anthropic Messages requests to the Gemini API and the answers back.
The user's normal `claude` and its login are never touched. Why these choices:
`docs/SO_SANH_GEMINI_PROXY.md`.

## Onboarding (dashboard)

- A "Gemini" card under "Claude accounts": button **+ Add Gemini account (free)**.
- Click: opens `https://aistudio.google.com/app/apikey` in the browser and a dialog with
  three steps (sign in to Google; click *Create API key*; copy it and paste it here), a
  key field, **Done** / Cancel. No email or password is asked.
- Done: `POST /api/gemini/key {key}`. The app checks the key with Google (list models,
  10 s); a bad key is refused with Google's message. A good one is saved in state.json
  (`gemini.key`, same file and rights as the Claude logins), with a fresh random proxy
  token, and the `cgemini` command is written (below).
- Then the card is short, for someone who has never used a terminal (seen 2026-10-29:
  "Ready · key …", "Model", "open a new terminal", then three steps, were all too much):
  - "Gemini is ready (free)." and one button, **Use this**: it opens a new window with
    Claude Code running on Gemini (`POST /api/gemini/open`), nothing to type.
  - One line: "Free has a daily limit and can be slower than Claude."
  - Folded under "Advanced" (a `<details>`): the model select (the key's Gemini models
    that write text, no image, speech, embedding or robotics ones; default below),
    "Small side jobs use: …", and "Or type `cgemini` in a new terminal" with Copy (a
    terminal opened before the key was saved still has the old PATH).
  - Free only for Flash models: of the models the research lists (`docs/SO_SANH_GEMINI_PROXY.md`
    §5, 2026-10-29), every Flash and Flash Lite one has a free tier, `gemini-4-argon` and
    `gemini-3.1-pro-preview` have none (Pro: 0 requests). A model without "flash" in its
    name is not known to be free: in the select it reads "… (may cost money)", and when the
    model or the side-jobs model is one, the card says "Gemini is ready." (no "free") and the
    line is "This model may not be free. Pick a Flash one under Advanced to stay free."
  - "Remove".
  `POST /api/gemini/model {model}`, `DELETE /api/gemini`.
- Just after the key is saved, a green line above: "All set! Press Use this to start."
- **Use this** (`POST /api/gemini/open`): `cmd.exe /k "<cgemini.cmd>"` in a new console
  window, started in the user's home folder (Claude Code asks there whether to trust the
  folder), out of this app's job when allowed, as the update setup is. No key: refused
  ("Add a Gemini account first"). The demo opens nothing.
- The key itself is never sent back to the page (only its last 4 characters).

### Copy to add (no paste, no "API key" words)

Most people do not know what an API key is. The dialog calls it an "access code" and
asks: sign in, make a code if there is none, then press the copy button next to it.
(Seen on 2026-10-29: a Google account new to AI Studio has no key and no project. Create
API key then asks for a project: "No Cloud Projects Available" → Create project → Create
project again (name kept) → Create key. Google does not make a key by itself.)
- Opening the dialog sends `POST /api/gemini/watch {on:true}`; closing it `{on:false}`.
  While on (10 minutes at most), the app looks at the Windows clipboard every 0.5 s
  (only when its sequence number changes, and once at the start).
- 10 minutes at most, counted by the page from when the dialog opened: it sends `on:true`
  again only when the app has lost the watch (`geminiWatch` null, e.g. the app restarted)
  and never to make it longer. At 10 minutes the page sends `{on:false}`, stops asking for
  the state, opens "Or paste it here" and says "Stopped looking after 10 minutes: paste it
  below." (The app's own 10-minute end stays, for a page that is gone.)
- Leaving the page (tab closed, reloaded, gone to another page: `pagehide`) with the dialog
  open sends `{on:false}` with `keepalive`, so it still goes out as the page unloads. A
  page brought back from the browser's back/forward cache closes the dialog.
- Each opening of the dialog is a new session: a pasted key's answer that comes back after
  the dialog was closed (or closed and opened again) changes nothing in it: no error, no
  "Checking…" ended, no closing. A key it saved still shows with the page's next state.
- Only text that is exactly a Google key counts: `AIza` + 35 of `[0-9A-Za-z_-]`, or
  `AQ.` + 20-200 of `[0-9A-Za-z_.-]` (the format AI Studio gives since May 2026).
  Anything else is ignored: never stored, logged or sent anywhere.
- A key seen: checked with Google and saved exactly as `POST /api/gemini/key` does; the
  watch ends. A key Google refuses: the error shows and the watch goes on.
- /api/state: `geminiWatch: null | {state: "waiting"|"checking"|"error", error, until}`.
  The page asks for the state every second while the dialog is open.
- Pasting by hand stays, under "Or paste it here".
- Privacy, said plainly in the dialog and in help (AI Studio's own terms, seen 2026-10-29):
  with a free key Google may read what is sent (prompts, answers, so the user's code) and
  use it to improve its models; not for code that must stay private. The note sits as a
  warning above the steps, not in the small print.

### Step pictures

Words alone do not show where to click. The dialog has four steps, with real screenshots
of Google's pages under them, the place to click ringed in red:
1. Sign in: `/img/gem-1-signin.png`.
2. No code yet (first time): `/img/gem-2-create.png` (Create API key),
   `/img/gem-3-project.png` (Create project, in the project list),
   `/img/gem-4-newproject.png` (Create project, name kept), `/img/gem-5-createkey.png`
   (Create key). Someone who already sees a code skips this step.
3. `/img/gem-6-copy.png`: the Copy button at the end of the code's row.
4. That's it: the window picks it up.
The pictures carry no email, name, project ID or key: those parts are cut off or blurred
before the file is saved. They are PNG files built into the program (served with
`image/png`), each under 150 KB, with an `alt` text saying what they show; on a narrow
screen they shrink to fit. The help page shows the same pictures.

## The `cgemini` command

- `%USERPROFILE%\.claude-usage-dashboard\bin\cgemini.cmd`, rewritten at every start of the
  app (in case the port changed: the app keeps it when it can, modules.md) and when the
  key is saved; deleted with the key.
- That folder is added once to the user's PATH (HKCU\Environment, then a
  WM_SETTINGCHANGE broadcast) when the key is first saved; new terminals find `cgemini`.
  Removed from PATH with the key.
- The script: `setlocal` (so nothing leaks into the terminal after it ends), checks the
  app answers (`curl.exe -sf --noproxy "*" http://127.0.0.1:<port>/v1/ping`), else prints
  "Claude Usage is not running: start it from the Start menu" and stops; sets
  `ANTHROPIC_BASE_URL`, the token and the model variables (see research), runs
  `claude %*`, `endlocal`.
- The ping never goes through a proxy: it runs before `NO_PROXY` is set, and Windows'
  curl uses `HTTP_PROXY`/`http_proxy`/`ALL_PROXY` for 127.0.0.1 too (measured 2026-10-30,
  curl 8.13.0: "Uses proxy env variable http_proxy", then a time-out on a dead proxy).
- Another provider chosen in the terminal or the user's settings would win over the link:
  the script clears Claude Code's provider switches (`CLAUDE_CODE_USE_BEDROCK`,
  `_VERTEX`, `_FOUNDRY`, `_MANTLE`, `_ANTHROPIC_AWS`, the list on
  code.claude.com/docs/en/env-vars, 2026-10-30) with `set NAME=` (unset), and the
  profile's `env` sets each to `""`, which those docs ("Precedence") say Claude Code
  treats as unset for provider selection; `"0"` is not documented as off for these.
- The token goes in `ANTHROPIC_CUSTOM_HEADERS` as `x-cud-token` as well as in
  `ANTHROPIC_AUTH_TOKEN`: seen 2026-10-29, Claude Code signed in to claude.ai sends its own
  login in `Authorization` and ignores `ANTHROPIC_AUTH_TOKEN` (then retries a 401 eight
  times). The proxy never logs or keeps that login.
- `claude --strict-mcp-config %*`: no MCP servers. Seen 2026-10-29: 74 of 102 tools were
  MCP ones (half of a 214 KB request), and a free key has 250K tokens a minute.
- Finding `claude`: on PATH first; else `%USERPROFILE%\.local\bin\claude.exe`, where Claude
  Code's own installer puts it (seen 2026-10-29: installed there, but that folder was not on
  PATH). Neither: prints "Claude Code is not installed: see https://claude.com/claude-code"
  and stops.

### Its own Claude Code profile

Like any other provider that speaks Anthropic's API (Z.AI: `ANTHROPIC_BASE_URL` and a
key), only the link and the key change; Gemini's link is this app, the translator (seen
2026-10-29: Google answers 404 to `/v1/messages`, Z.AI 401). So that Claude Code opens
straight to the prompt and never sees the Claude login, `cgemini` runs it on a profile
of its own: `set "CLAUDE_CONFIG_DIR=%~dp0..\gemini-profile"`, that is
`%USERPROFILE%\.claude-usage-dashboard\gemini-profile`. Written with the script (each
start, key saved, model changed, Use this):
- `settings.json`: a copy of the user's `~/.claude/settings.json` (hooks, status line,
  their other settings), without `enabledPlugins` and `extraKnownMarketplaces` (the
  plugins are not in this profile: Claude Code would fetch them again) nor
  `apiKeyHelper`; its `env` holds the same link, token and model variables as the script,
  and the provider switches set to `""` (above), over the user's own `env`.
  Rewritten each time: edits there are lost.
- `CLAUDE.md`: a copy of the user's `~/.claude/CLAUDE.md`, if there is one.
- `.claude.json`: `hasCompletedOnboarding: true` (and `theme: "dark"`) set once, when the
  mark is missing (Claude Code drops `theme` from it later, seen 2026-10-29); what
  Claude Code itself writes there (its history, trusted folders) is kept.
- No `.credentials.json`: the claude.ai login is not in this profile, so Claude Code
  sends the token. Claude Code still asks once per folder whether to trust it (a safety
  question, never answered for the user).
- Removing the key leaves the profile (its history); the token in it no longer works.

## The proxy (`src/gemini/`)

- Paths under `/v1/` on the dashboard's own server, before the page's checks; every one
  needs the token (`x-cud-token`, `Authorization: Bearer <token>` or `x-api-key`), so a web page in the
  browser cannot use the key. No key saved: 401 with an Anthropic-shaped error.
- `POST /v1/messages`: Anthropic request → Gemini `streamGenerateContent?alt=sse`
  (or `generateContent` when `stream` is false) → Anthropic SSE (or one message JSON).
  `POST /v1/messages/count_tokens`, `GET /v1/models`, `GET /v1/ping`: as the research says.
- Translation rules (request, tool schemas, thought signatures, stream events, stop
  reasons, errors, script variables, default models): `docs/SO_SANH_GEMINI_PROXY.md` →
  "Chọn". Models: `gemini-3.5-flash-lite`, and `gemini-3.1-flash-lite` for Claude's
  "haiku". Why Lite (AI Studio's Rate Limit page, free tier, 2026-10-29): Flash models give
  20 requests a day and 5 a minute, Flash Lite 500 a day and 15 a minute, each model its
  own count; Gemma's 16K tokens a minute is less than one Claude Code request; Pro models
  0. A key without the default: the newest stable Flash Lite it has, else Flash.
- Google busy or the model's count used up (503, 429): the same request once more with
  the other model of the two (main ↔ fast), when it differs. Its own error otherwise.
- `GET /v1/ping` needs no token (the script's check; it answers `{}` only).
- The key goes to Google in the `x-goog-api-key` header, never in a URL.
- A Gemini call has its own ureq agent: no overall timeout (a reply can take minutes),
  30 s to connect, 120 s of silence at most.
- The answer streams to Claude Code as it arrives (chunked), not after the whole reply.

## Shape

- State: `gemini: Option<gemini::Config>` (key, token, model, fastModel, models, savedAt).
- /api/state: `gemini: null` or `{keyEnd, model, fastModel, models, command}`.
- Code: `src/gemini/` (mod, request, schema, sigs, stream, errors, proxy, api, script);
  server.rs sends `/v1/*` to `gemini::serve` before reading the body, and asks
  `gemini::api` first for `/api/gemini*`.

## Tests first

Pure parts: request translation (system, roles, text, image, tool_use/tool_result with
names from earlier ids, schema keys stripped, generation config), stream translation
(text only; text then a tool call; finish reasons; usage), thought signature round trip,
the token check, the script text, the key never in /api/state. Web: the card, dialog and
endpoints. One real run with a key: `cgemini -p "say hi"` answers, and a tool call
(reading a file) works.
