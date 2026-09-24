#!/bin/sh
# Screenshots of the app in mock mode, into docs/screenshots/. Needs the dev server
# (`npm run dev`) and Google Chrome. Usage: npm run screenshots [base-url]
set -eu
BASE="${1:-http://localhost:5173/}"
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
OUT="$(dirname "$0")/../docs/screenshots"
mkdir -p "$OUT"

shot() { # name width height query
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --window-size="$2,$3" --virtual-time-budget=2500 \
    --screenshot="$OUT/$1.png" "$BASE?$4" >/dev/null 2>&1
  echo "$OUT/$1.png"
}

shot main-1440-dark 1440 900 "demo=1"
shot main-1440-light 1440 900 "demo=1&theme=light"
shot main-1280 1280 800 "demo=1"
shot main-1024 1024 1100 "demo=1"
shot main-390 390 1500 "demo=1"
shot tooltip-main-b 1440 900 "demo=1&tip=section.main_b"
shot tooltip-tempo 1440 900 "demo=1&tip=tempo.tap"
shot help-mode 1440 900 "demo=1&help=1&tip=transport.sync_start"
shot stopped-armed 1440 900 "demo=0"
