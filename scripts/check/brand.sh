#!/usr/bin/env bash
# Evidence for garden-fit: the product layer F6 verifies exists, in the shape the
# umbrella's own product layers are written in, and every contrast number the
# umbrella records for weeder's colours is the number the umbrella's method
# produces.
#
# The numbers are recomputed rather than read. A palette table is worth having
# only if a colour that fails its class cannot be written down as one that
# passes. The layer keeps no table of its own, its colour delta says the
# measured values live in the umbrella's `colors.md`, so this script fetches the
# umbrella at the version petalsrc.example names, measures every row there that
# names weeder's colours, and refuses a row that disagrees with the maths.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

layer=".brand/products/weeder"
status=0

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the contrast ratios cannot be measured. The check refuses to pass on unmeasured numbers." >&2
  exit 3
}

# The shape: the same files the umbrella's product layers carry, and a mark for
# each of the two surfaces.
for file in identity.md colors.md voice.md petalsrc.example; do
  if [ ! -f "$layer/$file" ]; then
    echo "$layer/$file is missing: the product layer is not in the shape the umbrella writes one in" >&2
    status=1
  fi
done
for mark in weeder-mark.svg weeder-mark-night.svg; do
  if [ ! -f "$layer/assets/$mark" ]; then
    echo "$layer/assets/$mark is missing: a mark is drawn for paper and for soil-night" >&2
    status=1
  fi
done
[ "$status" -eq 0 ] || exit "$status"

# identity.md carries the field table the umbrella's product layers open with.
for field in Product Tagline Accent; do
  if ! grep -qE "^\| $field \|" "$layer/identity.md"; then
    echo "$layer/identity.md has no $field row: the identity table is what a reader takes the product from" >&2
    status=1
  fi
done
for heading in "## Positioning" "## Mark" "## Logo usage"; do
  if ! grep -qF "$heading" "$layer/identity.md"; then
    echo "$layer/identity.md has no '$heading' section" >&2
    status=1
  fi
done

# The voice delta carries weeder's terminology, in both directions: the word to
# use and the word never to use.
if ! grep -qF "## Terminology" "$layer/voice.md"; then
  echo "$layer/voice.md has no terminology table: the voice delta is the terminology" >&2
  status=1
fi
for term in finding block warn note rule judge allowance guard honest; do
  if ! grep -qE "^\| .*\b${term}\b.*\|.*\|" "$layer/voice.md"; then
    echo "$layer/voice.md never says which word '$term' is" >&2
    status=1
  fi
done
for banned in violation error suppression lint; do
  if ! grep -qE "^\|[^|]*\|[^|]*\b${banned}\b[^|]*\|" "$layer/voice.md"; then
    echo "$layer/voice.md does not rule out '$banned' in its Not column" >&2
    status=1
  fi
done

# The accent is decided, the shed's bloom the umbrella's Product Accents table
# gives weeder, and is one value everywhere it is claimed: the identity table,
# the colour delta, and both marks' tiles. The marks are drawn in two values
# only, the tile and the ink identity.md names, on paper and on soil-night.
ink="$(grep -E '^- Two values only' "$layer/identity.md" | grep -oE 'ink `#[0-9A-Fa-f]{6}' | grep -oE '#[0-9A-Fa-f]{6}' | head -1 || true)"
if [ -z "$ink" ]; then
  echo "$layer/identity.md names no ink in its 'Two values only' line: the mark has no second value to be held to" >&2
  status=1
fi
accent="$(grep -E '^\| Accent \|' "$layer/identity.md" | grep -oE '#[0-9A-Fa-f]{6}' | head -1 || true)"
if [ -z "$accent" ]; then
  echo "$layer/identity.md names no accent hex in its Accent row" >&2
  status=1
else
  if ! grep -qiF "$accent" "$layer/colors.md"; then
    echo "$layer/colors.md never names the accent $accent the identity claims" >&2
    status=1
  fi
  for mark in weeder-mark.svg weeder-mark-night.svg; do
    for value in "$accent" "$ink"; do
      if [ -n "$value" ] && ! grep -qiF "$value" "$layer/assets/$mark"; then
        echo "$layer/assets/$mark does not use $value" >&2
        status=1
      fi
    done
    for hex in $(grep -oE '#[0-9A-Fa-f]{6}' "$layer/assets/$mark" | tr 'a-f' 'A-F' | sort -u); do
      case "$hex" in
        "$(printf '%s' "$accent" | tr 'a-f' 'A-F')"|"$(printf '%s' "$ink" | tr 'a-f' 'A-F')") ;;
        *) echo "$layer/assets/$mark uses $hex, which is neither the accent tile nor the ink" >&2; status=1 ;;
      esac
    done
  done
fi
if grep -qF "[flagged]" "$layer/identity.md" "$layer/colors.md"; then
  echo "$layer still marks something [flagged]; the name, the accent and the licence are settled" >&2
  status=1
fi

# Every number the umbrella records for weeder's colours, measured again. A row
# of its contrast tables reads "<colour> #hex on <background> #hex | ratio".
IFS=$'\t' read -r checkout _ version _ < <(bash "$root/scripts/umbrella-brand.sh")
measured="$(python3 - "$checkout/.brand/colors.md" "$version" <<'PY'
import re, sys

def _lin(channel):
    channel /= 255.0
    return channel / 12.92 if channel <= 0.03928 else ((channel + 0.055) / 1.055) ** 2.4

def luminance(colour):
    colour = colour.lstrip("#")
    red, green, blue = (int(colour[at:at + 2], 16) for at in (0, 2, 4))
    return 0.2126 * _lin(red) + 0.7152 * _lin(green) + 0.0722 * _lin(blue)

def ratio(one, other):
    a, b = luminance(one), luminance(other)
    return (max(a, b) + 0.05) / (min(a, b) + 0.05)

ROW = re.compile(r"^\|\s*([^|]*?#[0-9A-Fa-f]{6}[^|]*?\bon\b[^|]*?#[0-9A-Fa-f]{6}[^|]*?)\s*\|\s*([^|]*?)\s*\|")
HEX = re.compile(r"#[0-9A-Fa-f]{6}")

path, version = sys.argv[1:3]
checked = 0
wrong = []
for line in open(path, encoding="utf-8"):
    row = ROW.match(line)
    if row is None or "weeder" not in row.group(1).lower():
        continue
    pair, cell = row.group(1), row.group(2)
    subject, background = HEX.findall(pair)[:2]
    try:
        recorded = float(cell)
    except ValueError:
        wrong.append(f"{pair}: the ratio cell is '{cell}', which is not a ratio")
        continue
    measured = ratio(subject, background)
    checked += 1
    if abs(measured - recorded) > 0.05:
        wrong.append(f"{pair}: recorded {recorded:.1f}, measured {measured:.2f}")

for complaint in wrong:
    print(f"the umbrella's colors.md at {version}: {complaint}", file=sys.stderr)
if wrong:
    raise SystemExit(1)
if checked == 0:
    print(f"the umbrella's colors.md at {version} records no contrast ratio for weeder's colours", file=sys.stderr)
    raise SystemExit(1)
print(checked)
PY
)" || status=1

# petalsrc.example points at the umbrella and names this product's layer.
if ! grep -qF "product: weeder" "$layer/petalsrc.example"; then
  echo "$layer/petalsrc.example does not name weeder as the product layer to fetch" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo ".brand/products/weeder: the shape is whole, the accent $accent is one value everywhere, the marks are drawn in it and the ink $ink alone, and the $measured contrast ratios the umbrella records for weeder are measured and correct"
fi
exit "$status"
