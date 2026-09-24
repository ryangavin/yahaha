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

shot laptop-1440 1440 900 "demo=1"
shot laptop-1280 1280 800 "demo=1"
shot large-1920 1920 1080 "demo=1"
shot light-1440 1440 900 "demo=1&theme=light"
shot tooltip-pad 1440 900 "demo=1&tip=section.main_b"
shot tooltip-fader 1440 900 "demo=1&tip=mixer.panel.right2"
shot help-mode 1440 900 "demo=1&help=1&tip=transport.sync_start"
shot shift-layer 1440 900 "demo=1&shift=1"
shot stopped-armed 1440 900 "demo=0"
shot drawer-parts 1440 900 "demo=1&open=parts"
shot drawer-mixer-panel 1440 900 "demo=1&open=mixer"
shot drawer-multipad 1440 900 "demo=1&open=multipad"
shot narrow-1024 1024 900 "demo=1"
shot light-1024 1024 900 "demo=1&theme=light"
shot light-1920 1920 1080 "demo=1&theme=light"
shot min-900x600 900 600 "demo=1"
shot tooltip-keystrip 1440 900 "demo=1&tip=keystrip.keys"
shot tooltip-lead 1440 900 "demo=1&tip=lead.progress"
shot browser-1440 1440 900 "demo=0&open=browser"
shot browser-playing-1440 1440 900 "demo=1&open=browser"
shot browser-light-1440 1440 900 "demo=0&open=browser&theme=light"
shot browser-1024 1024 768 "demo=0&open=browser"
shot browser-1920 1920 1080 "demo=1&open=browser"
shot browser-60k-1440 1440 900 "demo=0&open=browser&styles=60000"
shot settings-chord 1440 900 "demo=1&open=settings&tab=chord"
shot settings-split 1440 900 "demo=1&open=settings&tab=split"
shot settings-transpose 1440 900 "demo=1&open=settings&tab=transpose"
shot settings-style 1440 900 "demo=1&open=settings&tab=style"
shot settings-audio 1440 900 "demo=1&open=settings&tab=audio"
shot settings-midi 1440 900 "demo=1&open=settings&tab=midi"
shot settings-library 1440 900 "demo=1&open=settings&tab=library"
shot settings-light-1440 1440 900 "demo=1&open=settings&tab=chord&theme=light"
shot settings-light-split 1440 900 "demo=1&open=settings&tab=split&theme=light"
shot settings-narrow-1024 1024 768 "demo=1&open=settings&tab=split"
shot settings-large-1920 1920 1080 "demo=1&open=settings&tab=audio"
shot tooltip-settings-fingering 1440 900 "demo=1&open=settings&tab=chord&tip=fingering.single_finger"
