#!/bin/sh
# Render one generated board to PNG: ./render.sh BHome  ->  /tmp/BHome.png
cd "$(dirname "$0")"
size=$(node render.js "project/$1.dc.html" "/tmp/$1.html")
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --hide-scrollbars --window-size="$(echo "$size" | tr x ,)" --virtual-time-budget=4000 --screenshot="/tmp/$1.png" "file:///tmp/$1.html" 2>/dev/null
echo "/tmp/$1.png"
