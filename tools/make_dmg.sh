#!/bin/sh
# SCAS.app and a link to /Applications in one compressed, read-only disk image (UDZO),
# the way Alacritty's Makefile does it. Spec: docs/specs/macos.md ("SCAS.app and the .dmg").
#   tools/make_dmg.sh dist/SCAS.app SCAS-1.6.1.dmg      (macOS only: hdiutil, ditto)
set -eu
app="$1"
out="$2"
stage="$(mktemp -d)"
ditto "$app" "$stage/SCAS.app"  # keeps modes, links and the signature
ln -s /Applications "$stage/Applications"
# hdiutil on hosted runners now and then fails with "Resource busy": try again.
for i in 1 2 3 4 5; do
  if hdiutil create -volname SCAS -fs HFS+ -srcfolder "$stage" -ov -format UDZO "$out"; then
    rm -rf "$stage"
    exit 0
  fi
  sleep $((i * 5))
done
echo "hdiutil create failed 5 times" >&2
exit 1
