#!/bin/bash
# Smoke test of the shipped dmg on a Mac (the CI runner). Spec: docs/specs/macos.md
# ("SCAS.app and the .dmg"): start, outlive the launcher, reopen without a second
# server, quit with the dashboard's Exit.
#   tools/smoke_macos_app.sh SCAS-1.6.1.dmg
set -euo pipefail
dmg="$1"
url="http://127.0.0.1:4777"
logs="$HOME/Library/Logs/SCAS"

fail() { echo "FAIL: $*"; tail -n 40 "$logs"/*.log 2>/dev/null || true; exit 1; }
servers() { pgrep -f 'Contents/MacOS/scas$' | wc -l | tr -d ' '; }
last_word() { tail -n 1 "$logs/launcher.log" | awk '{print $3}'; }
answers() { curl -sf -o /dev/null --max-time 2 "$url/api/state"; }

mnt="$(mktemp -d)/mnt"
hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mnt"
[ -d "$mnt/SCAS.app" ] || fail "no SCAS.app in the dmg"
[ "$(readlink "$mnt/Applications")" = /Applications ] || fail "no Applications link"
apps="$(mktemp -d)"
ditto "$mnt/SCAS.app" "$apps/SCAS.app"
hdiutil detach "$mnt"
codesign --verify --deep --strict --verbose=2 "$apps/SCAS.app"

[ "$(servers)" = 0 ] || fail "a scas was already running"
open "$apps/SCAS.app"
up=""
for _ in $(seq 1 30); do answers && { up=1; break; }; sleep 1; done
[ -n "$up" ] || fail "no answer on $url after 30 s"
[ "$(last_word)" = start ] || fail "first launch: launcher did not start scas"
sleep 5
pgrep -f 'MacOS/scas-launcher' > /dev/null && fail "the launcher is still running"
answers || fail "scas died with the launcher"
[ "$(servers)" = 1 ] || fail "want 1 scas, have $(servers)"

open "$apps/SCAS.app"
sleep 5
[ "$(last_word)" = open ] || fail "second launch did not reopen the dashboard"
[ "$(servers)" = 1 ] || fail "second launch left $(servers) scas processes"

curl -sf -X POST -H 'X-CUD: 1' -o /dev/null "$url/api/exit" || fail "Exit refused"
for _ in $(seq 1 10); do [ "$(servers)" = 0 ] && break; sleep 1; done
[ "$(servers)" = 0 ] || fail "scas still running after Exit"
echo "ok: started, outlived the launcher, reopened with one server, quit"
cat "$logs/launcher.log"
