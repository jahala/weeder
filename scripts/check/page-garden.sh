#!/usr/bin/env bash
# Evidence for page.tend2.html c2: the page carries the garden's own footer, the
# canonical shears in its header, and no number the calibration report does not
# state.
#
# Nothing here is trusted from a copy. The garden footer is compared against the
# reference implementation in the umbrella's `.brand/components.md`, fetched by
# `scripts/umbrella-brand.sh` from the repository petalsrc.example names; the
# mark is compared against `.brand/products/weed/assets/weed-mark.svg` shape by
# shape; and every number the page states about the calibration is read out of
# `docs/calibration-2026-09.md` first and looked for on the page second.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

page="$root/index.html"
[ -f "$page" ] || {
  echo "index.html is missing from the repository root: there is no page to judge" >&2
  exit 1
}
report="$root/docs/calibration-2026-09.md"
[ -f "$report" ] || {
  echo "$report is missing: the page's numbers have nothing to be true to" >&2
  exit 3
}

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the page cannot be parsed. The check refuses to pass unread." >&2
  exit 3
}

IFS=$'\t' read -r checkout sha version product < <(bash "$root/scripts/umbrella-brand.sh")
components="$checkout/.brand/components.md"
mark="$root/.brand/products/$product/assets/weed-mark.svg"
[ -f "$components" ] || {
  echo "$components is missing from the umbrella at $sha: there is no reference footer to copy" >&2
  exit 3
}
[ -f "$mark" ] || {
  echo "$mark is missing: the header has no canonical mark to carry" >&2
  exit 3
}

python3 - "$page" "$components" "$mark" "$report" "$version" "$sha" <<'PY'
import re
import sys

page_path, components_path, mark_path, report_path, version, sha = sys.argv[1:7]
page = open(page_path, encoding="utf-8").read()
components = open(components_path, encoding="utf-8").read()
mark = open(mark_path, encoding="utf-8").read()
report = open(report_path, encoding="utf-8").read()

complaints = []


def complain(message):
    complaints.append(message)


# ── html, read as a flat sequence of tags and text ──────────────────────────
TOKEN = re.compile(
    r"<!--.*?-->"
    r"|<(/?)([A-Za-z][A-Za-z0-9]*)((?:[^>\"']|\"[^\"]*\"|'[^']*')*)>"
    r"|([^<]+)",
    re.S,
)
ATTR = re.compile(r"([A-Za-z_:][-A-Za-z0-9_:.]*)(?:\s*=\s*(\"[^\"]*\"|'[^']*'|[^\s/>]+))?")
# A link is allowed to point somewhere else on every page; a colour, a class and
# a shape are not.
LINKS = ("href", "src")


def attributes(raw):
    raw = raw.rstrip().rstrip("/")
    found = {}
    for match in ATTR.finditer(raw):
        name = match.group(1).lower()
        value = match.group(2) or ""
        if value[:1] in ("\"", "'"):
            value = value[1:-1]
        found[name] = " ".join(value.split())
    return found


def classes(attrs):
    return attrs.get("class", "").split()


def tokens(html):
    out = []
    for match in TOKEN.finditer(html):
        if match.group(0).startswith("<!--"):
            continue
        if match.group(2) is not None:
            closing, name, raw = match.group(1), match.group(2).lower(), match.group(3)
            out.append(("close" if closing else "open", name, attributes(raw)))
            continue
        text = " ".join(match.group(4).split())
        if text:
            out.append(("text", text, {}))
    return out


def skeleton(token):
    kind, name, attrs = token
    if kind == "text":
        # words are held to their own rule below, which knows what a page may say
        return ("text",)
    if kind != "open":
        return (kind, name)
    shown = {}
    for key, value in attrs.items():
        if key in LINKS:
            shown[key] = "*"
        elif key == "class":
            # the current-product pill is the one class a page is meant to move
            kept = sorted(word for word in value.split() if word != "is-current")
            if kept:
                shown[key] = " ".join(kept)
        else:
            shown[key] = value
    return (kind, name, tuple(sorted(shown.items())))


def region(html, opening, closing):
    start = html.find(opening)
    if start < 0:
        return None
    end = html.find(closing, start)
    if end < 0:
        return None
    return html[start:end + len(closing)]


# The identity column and the product's own positioning line are what
# components.md lets a page change; every other word in the footer is the
# garden's and must be carried across untouched.
def exempt_text(token_list):
    exempt = set()
    stack = []
    columns = 0
    bottom_spans = 0
    for index, (kind, name, attrs) in enumerate(token_list):
        if kind == "open":
            names = classes(attrs)
            if "gf-col" in names:
                columns += 1
            if "gf-bottom" in [word for frame in stack for word in frame[1]] and name == "span":
                bottom_spans += 1
            stack.append((name, names, columns, bottom_spans))
        elif kind == "close":
            while stack and stack[-1][0] != name:
                stack.pop()
            if stack:
                stack.pop()
        else:
            in_identity = any("gf-col" in frame[1] and frame[2] == 1 for frame in stack)
            in_position = any(
                frame[0] == "span" and frame[3] == 2
                and any("gf-bottom" in other[1] for other in stack)
                for frame in stack
            )
            if in_identity or in_position:
                exempt.add(index)
    return exempt


reference = None
after = components.split("### Reference implementation", 1)
if len(after) == 2:
    fenced = re.search(r"```html\n(.*?)\n```", after[1], re.S)
    if fenced:
        reference = fenced.group(1)
if reference is None:
    complain(
        "the umbrella's components.md at %s carries no html reference implementation "
        "under '### Reference implementation'" % sha
    )

footer = region(page, "<footer class=\"gf\">", "</footer>")
if footer is None:
    complain("index.html has no <footer class=\"gf\"> ... </footer>: the garden footer is not on the page")

if reference is not None and footer is not None:
    want, have = tokens(reference), tokens(footer)
    want_exempt, have_exempt = exempt_text(want), exempt_text(have)
    for index in range(max(len(want), len(have))):
        if index >= len(want):
            complain("the footer runs past the reference implementation at %r" % (have[index],))
            break
        if index >= len(have):
            complain("the footer stops short of the reference implementation at %r" % (want[index],))
            break
        if skeleton(want[index]) != skeleton(have[index]):
            complain(
                "the footer departs from the reference implementation: expected %r, found %r"
                % (skeleton(want[index]), skeleton(have[index]))
            )
            break
        if want[index][0] == "text" and index not in want_exempt:
            if index in have_exempt or want[index][1] != have[index][1]:
                complain(
                    "the footer rewrites the garden's own words: expected %r, found %r"
                    % (want[index][1], have[index][1])
                )
                break

    # the plotplot band sits above the columns
    def first(token_list, needle):
        for index, (kind, _, attrs) in enumerate(token_list):
            if kind == "open" and needle in classes(attrs):
                return index
        return None

    band, grid = first(have, "gf-plot"), first(have, "gf-grid")
    if band is None:
        complain("the footer carries no plotplot band: a product page names the umbrella")
    elif grid is None:
        complain("the footer carries no column grid")
    elif band > grid:
        complain("the plotplot band sits below the columns; it belongs above them")

    # the garden row: every bed the reference lists, each with its bloom dot,
    # and this product marked current
    def garden_row(token_list, where):
        beds, depth, current = [], None, []
        for kind, name, attrs in token_list:
            if kind == "open" and "gf-garden" in classes(attrs):
                depth = 0
                continue
            if depth is None:
                continue
            if kind == "open" and name == "a":
                beds.append({"style": attrs.get("style", ""), "dot": False, "name": ""})
                if "is-current" in classes(attrs):
                    current.append(len(beds) - 1)
            elif kind == "open" and name == "span" and beds and "gf-dot" in classes(attrs):
                beds[-1]["dot"] = True
            elif kind == "text" and beds:
                beds[-1]["name"] = (beds[-1]["name"] + " " + name).strip()
            elif kind == "close" and name == "nav":
                break
        if not beds:
            complain("%s has no garden row" % where)
        return beds, current

    wanted_beds, _ = garden_row(want, "the reference implementation")
    have_beds, current = garden_row(have, "the page's footer")
    if [bed["name"] for bed in wanted_beds] != [bed["name"] for bed in have_beds]:
        complain(
            "the garden row lists %r, and the reference lists %r"
            % ([bed["name"] for bed in have_beds], [bed["name"] for bed in wanted_beds])
        )
    else:
        for bed in have_beds:
            if not bed["dot"]:
                complain("the garden row's %s has no bloom dot" % bed["name"])
            if "--bloom:var(--pp-%s)" % bed["name"] not in bed["style"].replace(" ", ""):
                complain(
                    "the garden row's %s carries %r rather than its own bloom"
                    % (bed["name"], bed["style"])
                )
    if len(current) != 1:
        complain("the garden row marks %d pills current; exactly one page is current" % len(current))
    elif have_beds[current[0]]["name"] != "weed":
        complain(
            "the garden row marks %s current on weed's own page" % have_beds[current[0]]["name"]
        )


# ── the header carries the canonical shears ────────────────────────────────
GEOMETRY = ("d", "cx", "cy", "r", "rx", "ry", "points", "x", "y", "width", "height", "transform")


def shapes(html):
    drawn = []
    for kind, name, attrs in tokens(html):
        if kind != "open" or name not in ("path", "circle", "ellipse", "polygon", "rect", "line"):
            continue
        drawn.append((name, tuple(sorted(
            (key, value) for key, value in attrs.items() if key in GEOMETRY
        ))))
    return drawn


header = region(page, "<header", "</header>")
if header is None:
    complain("index.html has no <header> ... </header>")
else:
    marks = re.findall(r"<svg\b.*?</svg>", header, re.S)
    canonical = shapes(mark)
    wanted_box = ""
    for kind, name, attrs in tokens(mark):
        if kind == "open" and name == "svg":
            wanted_box = attrs.get("viewbox", "")
            break
    carried = None
    for candidate in marks:
        if shapes(candidate) == canonical:
            carried = candidate
            break
    if carried is None:
        complain(
            "the header draws no svg whose shapes are %s: the mark beside the wordmark is not the "
            "canonical one" % mark_path
        )
    else:
        box = ""
        for kind, name, attrs in tokens(carried):
            if kind == "open" and name == "svg":
                box = attrs.get("viewbox", "")
                break
        if box != wanted_box:
            complain("the header's mark is drawn in viewBox %r and the asset in %r" % (box, wanted_box))
        wordmark = None
        tail = header[header.index(carried) + len(carried):]
        for index, (kind, name, attrs) in enumerate(tokens(tail)):
            if kind == "open" and "wordmark" in classes(attrs):
                following = tokens(tail)[index + 1]
                wordmark = following[1] if following[0] == "text" else ""
                break
        if wordmark is None:
            complain("no wordmark follows the mark in the header")
        elif wordmark != "weed":
            complain("the wordmark beside the mark reads %r rather than 'weed'" % wordmark)


# ── every number the page states about the calibration, read from the report ──
def one(pattern, what, text=report):
    found = re.search(pattern, text, re.M)
    if not found:
        complain("%s: the report does not state it in the shape this check reads" % what)
        return None
    return found.groups()

pooled = one(
    r"\|\s*\*\*pooled\*\*\s*\|\s*\*\*([\d,]+)\*\*\s*\|\s*\*\*([\d,]+)\*\*\s*\|\s*\*\*([\d,]+)\*\*"
    r"\s*\|\s*\*\*([\d,]+)\*\*\s*\|\s*\*\*([\d,]+)\*\*\s*\|\s*\*\*([\d,]+)\*\*\s*\|\s*\*\*([\d.]+)%\*\*",
    "the pooled row of the totals table",
)
repos = one(r"in (\d+) repositories", "the number of repositories judged")
floor = one(r"catches at least (\d+) percent of the anti-patterns", "the recall floor")
planted = one(r"([\d,]+) cases were planted", "the number of cases planted")
caught = one(r"([\d,]+) of them were caught", "the number of cases caught")
blind_blocks = one(
    r"blocked commits in \S+ at ([\d.]+) percent of (\d+) cases",
    "the blind re-grade's agreement on blocked commits",
)
blind_recall = one(
    r"recall cases in \S+ at ([\d.]+) percent of (\d+) cases",
    "the blind re-grade's agreement on recall cases",
)
old = one(
    r"the same (\d+) blocked commits counted (\d+) false positives, ([\d.]+) percent",
    "the count under the classes in use before the ruling",
)
dated = one(r"^# calibration: [^\n]*?(\d{4}-\d{2})\s*$", "the month the report is dated")

required = {}
if pooled:
    commits, blocked, _, _, _, false_positives, share = pooled
    required["commits judged"] = commits
    required["blocked commits"] = blocked
    required["block-level false positives"] = false_positives
    required["the false-positive share"] = share
if repos:
    required["the repositories judged"] = repos[0]
if floor:
    required["the recall floor"] = floor[0]
if planted:
    required["the cases planted"] = planted[0]
if caught:
    required["the cases caught"] = caught[0]
if old:
    required["the blocked commits under the old classes"] = old[0]
    required["the false positives under the old classes"] = old[1]
    required["the false-positive share under the old classes"] = old[2]
if dated:
    required["the month the calibration is dated"] = dated[0]

derived = set()
for label, agreement in (("blocked commits", blind_blocks), ("recall cases", blind_recall)):
    if not agreement:
        continue
    percent, cases = float(agreement[0]), int(agreement[1])
    agreed = round(percent * cases / 100)
    required["the blind re-grade on %s" % label] = "%d of %d" % (agreed, cases)
    derived.update({str(agreed), str(cases)})

numbers = None
for block in re.findall(r"<section\b.*?</section>", page, re.S):
    if re.match(r"<section\b[^>]*\bid=\"numbers\"", block):
        numbers = block
        break
if numbers is None:
    complain("index.html has no <section id=\"numbers\">: the calibration has nowhere to be stated")
else:
    plain = re.sub(r"<[^>]*>", " ", numbers)
    plain = " ".join(plain.split())
    for what, value in sorted(required.items()):
        if value not in plain:
            complain("the page does not state %s, which the report puts at %s" % (what, value))

    # And nothing the report cannot account for: every number in the section is
    # either one the report writes or one drawn from the ones it writes.
    NUMBER = re.compile(r"\d+(?:\.\d+)?")
    known = set(NUMBER.findall(report)) | derived
    for token in NUMBER.findall(plain):
        if token not in known:
            complain(
                "the page states %s in its numbers, and the report never writes that number" % token
            )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

print(
    "index.html: the garden footer matches the umbrella's reference implementation at %s, "
    "the garden row lists all eight beds with weed current, the header carries the canonical "
    "shears, and every calibration number on it is the report's own" % version
)
PY
