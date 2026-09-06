#!/usr/bin/env bash
# Evidence for garden-fit: the product layer F6 verifies exists, in the shape the
# umbrella's own product layers are written in, and every contrast number in it
# is the number the umbrella's method produces.
#
# The numbers are recomputed rather than read. A palette table is worth having
# only if a colour that fails its class cannot be written down as one that
# passes, so this script measures every hex in every measured column against the
# background that column names and refuses a row that disagrees with the maths.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

layer=".brand/products/weed"
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
for mark in weed-mark.svg weed-mark-night.svg; do
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

# The voice delta carries weed's terminology, in both directions: the word to
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

# The accent is decided (bramble, 2026-09-05) and is one value everywhere it is
# claimed: the identity table, the colour delta, and both marks' fills.
accent="$(grep -E '^\| Accent \|' "$layer/identity.md" | grep -oE '#[0-9A-Fa-f]{6}' | head -1 || true)"
if [ -z "$accent" ]; then
  echo "$layer/identity.md names no accent hex in its Accent row" >&2
  status=1
else
  if ! grep -qiF "$accent" "$layer/colors.md"; then
    echo "$layer/colors.md never names the accent $accent the identity claims" >&2
    status=1
  fi
  for mark in weed-mark.svg weed-mark-night.svg; do
    if ! grep -qiF "$accent" "$layer/assets/$mark"; then
      echo "$layer/assets/$mark does not use the accent $accent" >&2
      status=1
    fi
    for hex in $(grep -oE '#[0-9A-Fa-f]{6}' "$layer/assets/$mark" | tr 'a-f' 'A-F' | sort -u); do
      case "$hex" in
        "$(printf '%s' "$accent" | tr 'a-f' 'A-F')"|"#357E2C"|"#84C56A") ;;
        *) echo "$layer/assets/$mark uses $hex, which is neither the accent nor the plant's two greens" >&2; status=1 ;;
      esac
    done
  done
fi
if grep -qF "[flagged]" "$layer/identity.md" "$layer/colors.md"; then
  echo "$layer still marks something [flagged]; the name, the accent and the licence are settled" >&2
  status=1
fi

# Every measured number, measured again. A column whose heading carries a hex is
# a contrast column, and the cell below it is a ratio against that background.
measured="$(python3 - "$layer/colors.md" <<'PY'
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

HEX = re.compile(r"#[0-9A-Fa-f]{6}")

def cells(line):
    return [cell.strip() for cell in line.strip().strip("|").split("|")]

source = open(sys.argv[1], encoding="utf-8").read().splitlines()
checked = 0
wrong = []
header = None
for line in source:
    if not line.startswith("|"):
        header = None
        continue
    row = cells(line)
    if header is None:
        header = row
        continue
    if all(set(cell) <= {"-", ":"} for cell in row):
        continue
    if "Hex" not in header:
        continue
    subject = HEX.search(row[header.index("Hex")])
    if subject is None:
        continue
    for column, heading in enumerate(header):
        background = HEX.search(heading)
        if background is None or column >= len(row):
            continue
        try:
            recorded = float(row[column])
        except ValueError:
            wrong.append(f"{row[0]}: the {heading} cell is '{row[column]}', which is not a ratio")
            continue
        measured = ratio(subject.group(), background.group())
        checked += 1
        if abs(measured - recorded) > 0.05:
            wrong.append(
                f"{row[0]} {subject.group()} on {background.group()}: "
                f"recorded {recorded:.1f}, measured {measured:.2f}"
            )

for complaint in wrong:
    print(complaint, file=sys.stderr)
if wrong:
    raise SystemExit(1)
if checked == 0:
    print("no contrast ratio was measured: no table carries a Hex column and a background heading", file=sys.stderr)
    raise SystemExit(1)
print(checked)
PY
)" || status=1

# petalsrc.example points at the umbrella and names this product's layer.
if ! grep -qF "product: weed" "$layer/petalsrc.example"; then
  echo "$layer/petalsrc.example does not name weed as the product layer to fetch" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo ".brand/products/weed: the shape is whole, the accent $accent is one value everywhere, $measured contrast ratios measured and correct"
fi
exit "$status"
