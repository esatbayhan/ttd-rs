#!/usr/bin/env bash
set -euo pipefail
# ---------------------------------------------------------------------------
# capture-screenshots.sh – generate demo data and capture vhs screenshots
#
# Prerequisites: vhs (install via your package manager or
#   go install github.com/charmbracelet/vhs@latest)
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

if ! command -v vhs &>/dev/null; then
  echo "Error: 'vhs' is not installed."
  echo "  Arch: pacman -S vhs"
  echo "  Go:   go install github.com/charmbracelet/vhs@latest"
  exit 1
fi

echo "=== Generating demo dataset ==="
bash scripts/generate-demo-data.sh

echo "=== Building release binary ==="
cargo build --release

echo "=== Capturing screenshots ==="
mkdir -p docs/screenshots

for tape in scripts/tapes/*.tape; do
  name=$(basename "$tape" .tape)
  echo "  Running $name ..."
  vhs -q "$tape"
done

echo ""
echo "=== Extracting PNGs from GIFs ==="
for gif in docs/screenshots/*.gif; do
  base="${gif%.gif}"
  png="${base}.png"
  echo "  $gif -> $png"

  frame_count=$(magick identify "$gif" | wc -l)
  pick=$((frame_count * 85 / 100))
  if [ "$pick" -lt 1 ]; then pick=1; fi

  magick "${gif}[0-${pick}]" -coalesce -delete 0--2 "$png"
done

echo ""
echo "Done. Output in docs/screenshots/:"
ls -lh docs/screenshots/
