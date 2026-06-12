#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INPUT="$ROOT_DIR/examples/demo/investment_projects/demo_terminal.txt"
OUTPUT="$ROOT_DIR/examples/demo/investment_projects/demo.gif"

ffmpeg -y \
  -f lavfi -i "color=c=0b1020:s=960x540:d=3:r=2" \
  -vf "drawtext=textfile=${INPUT}:fontcolor=f8fafc:fontsize=28:x=48:y=44:line_spacing=12,drawbox=x=24:y=24:w=912:h=492:color=38bdf8@0.55:t=3" \
  -loop 0 \
  "$OUTPUT" >/dev/null 2>&1

echo "wrote $OUTPUT"
