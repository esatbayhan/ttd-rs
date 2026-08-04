#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TMP_DIR=$(mktemp -d)
SENTINEL_DIR="$TMP_DIR/sentinel"
SENTINEL_FILE="$SENTINEL_DIR/keep.txt"

cleanup() {
  if [[ -n "${TMP_DIR:-}" && -d "$TMP_DIR" ]]; then
    rm -rf -- "$TMP_DIR"
  fi
}
trap cleanup EXIT

mkdir -p "$SENTINEL_DIR"
printf 'keep me\n' > "$SENTINEL_FILE"

set +e
bash "$PROJECT_DIR/scripts/generate-demo-data.sh" "$SENTINEL_DIR" >/dev/null 2>&1
status=$?
set -e

failed=false
if [[ $status -ne 2 ]]; then
  printf 'expected argument invocation to exit 2, got %d\n' "$status" >&2
  failed=true
fi
if [[ ! -f "$SENTINEL_FILE" ]]; then
  printf 'generator removed the sentinel file\n' >&2
  failed=true
elif [[ $(<"$SENTINEL_FILE") != "keep me" ]]; then
  printf 'generator changed the sentinel file\n' >&2
  failed=true
fi

if [[ $failed == true ]]; then
  exit 1
fi
