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
- Then the card shows: "Ready: open a new terminal and type `cgemini`", the model in use
  (a select: the Gemini models of the key, default as in the research), "Remove".
  `POST /api/gemini/model {model}`, `DELETE /api/gemini`.
- The key itself is never sent back to the page (only its last 4 characters).

### Copy to add (no paste, no "API key" words)

Most people do not know what an API key is. Google gives a new AI Studio user a key by
itself once they accept the terms, so the dialog only asks: sign in, then press the copy
button next to the key ("access code" in the text).
- Opening the dialog sends `POST /api/gemini/watch {on:true}`; closing it `{on:false}`.
  While on (10 minutes at most), the app looks at the Windows clipboard every 0.5 s
  (only when its sequence number changes, and once at the start).
- Only text that is exactly a Google key counts: `AIza` + 35 of `[0-9A-Za-z_-]`, or
  `AQ.` + 20-200 of `[0-9A-Za-z_.-]` (the format AI Studio gives since May 2026).
  Anything else is ignored: never stored, logged or sent anywhere.
- A key seen: checked with Google and saved exactly as `POST /api/gemini/key` does; the
  watch ends. A key Google refuses: the error shows and the watch goes on.
- /api/state: `geminiWatch: null | {state: "waiting"|"checking"|"error", error, until}`.
  The page asks for the state every second while the dialog is open.
- Pasting by hand stays, under "Or paste it here".

## The `cgemini` command

- `%USERPROFILE%\.claude-usage-dashboard\bin\cgemini.cmd`, rewritten at every start of the
  app (the port changes) and when the key is saved; deleted with the key.
- That folder is added once to the user's PATH (HKCU\Environment, then a
  WM_SETTINGCHANGE broadcast) when the key is first saved; new terminals find `cgemini`.
  Removed from PATH with the key.
- The script: `setlocal` (so nothing leaks into the terminal after it ends), checks the
  app answers (`curl -s http://127.0.0.1:<port>/v1/ping`), else prints "Claude Usage is
  not running: start it from the Start menu" and stops; sets `ANTHROPIC_BASE_URL`, the
  token and the model variables (see research), runs `claude %*`, `endlocal`.

## The proxy (`src/gemini/`)

- Paths under `/v1/` on the dashboard's own server, before the page's checks; every one
  needs the token (`Authorization: Bearer <token>` or `x-api-key`), so a web page in the
  browser cannot use the key. No key saved: 401 with an Anthropic-shaped error.
- `POST /v1/messages`: Anthropic request → Gemini `streamGenerateContent?alt=sse`
  (or `generateContent` when `stream` is false) → Anthropic SSE (or one message JSON).
  `POST /v1/messages/count_tokens`, `GET /v1/models`, `GET /v1/ping`: as the research says.
- Translation rules (request, tool schemas, thought signatures, stream events, stop
  reasons, errors, script variables, default models): `docs/SO_SANH_GEMINI_PROXY.md` →
  "Chọn". Models: `gemini-3.8-flash`, and `gemini-3.5-flash-lite` for Claude's "haiku".
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
