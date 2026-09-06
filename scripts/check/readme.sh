#!/usr/bin/env bash
# Evidence for garden-fit: the licence and the front door.
#
# LICENSE is compared against the MIT text word for word, so a paraphrase or a
# quietly altered clause cannot pass as MIT. README.md is read the way a
# newcomer reads it, the first paragraph has to say what weeder is, and then the
# way a machine reads it: every path it cites exists, and every weeder command it
# cites is a command the binary answers, with flags the binary prints. A readme
# that names a file nobody wrote is the first lie a reader is told.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

status=0
for file in LICENSE README.md; do
  [ -f "$file" ] || { echo "$file is missing" >&2; status=1; }
done
[ "$status" -eq 0 ] || exit "$status"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the readme cannot be read. The check refuses to pass on an unread readme." >&2
  exit 3
}

cargo build --quiet --bin weeder
binary="target/debug/weeder"

python3 - "$binary" <<'PY' || status=1
import os
import re
import subprocess
import sys

BINARY = sys.argv[1]

MIT = """MIT License

Copyright (c) {holder}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""

complaints = []


def words(text):
    return re.sub(r"\s+", " ", text).strip()


# --- LICENSE -----------------------------------------------------------------

licence = open("LICENSE", encoding="utf-8").read()
holder = re.search(r"^Copyright \(c\) (.+)$", licence, re.M)
if holder is None:
    complaints.append("LICENSE carries no `Copyright (c) <year> <owner>` line")
else:
    named = holder.group(1).strip()
    if not re.match(r"^\d{4}(-\d{4})? +\S", named):
        complaints.append(f"LICENSE's copyright line is `{named}`, which names no year and owner")
    # Whitespace is where licence files differ innocently, the wrapping, so
    # the comparison is on words. Everything else has to match exactly.
    if words(licence) != words(MIT.format(holder=named)):
        complaints.append(
            "LICENSE is not the MIT text. Compare it with the canonical licence; a paraphrase is "
            "a different licence, whatever the heading says"
        )

# --- README ------------------------------------------------------------------

readme = open("README.md", encoding="utf-8").read()
lines = readme.splitlines()

if not lines or lines[0].strip() != "# weeder":
    complaints.append("README.md does not open with the title `# weeder`")
else:
    after = [line for line in lines[1:] if line.strip()]
    opening = after[0] if after else ""
    if opening.startswith(("#", "-", "*", "|", "```", ">", "[!")):
        complaints.append(
            f"README.md's first line after the title is `{opening[:60]}`, and a reader who wants "
            "to know what weeder is has to read past it"
        )
    else:
        paragraph = []
        for line in lines[1:]:
            if line.strip():
                paragraph.append(line.strip())
            elif paragraph:
                break
        paragraph = " ".join(paragraph)
        if "weeder" not in paragraph:
            complaints.append("README.md's opening paragraph never names weeder")
        if paragraph.count(".") < 1 or len(paragraph) < 80:
            complaints.append(f"README.md's opening paragraph is too thin to say what weeder is: `{paragraph}`")

# The claims the owner has not settled, each marked where it is made. A mark
# belongs to the block it sits in, a bullet with its continuation lines, or a
# paragraph, because that is the unit a reader takes a claim from.
def blocks(lines):
    out = []
    current = []
    for line in lines:
        opens = re.match(r"^\s*([-*+]|\d+\.)\s|^#", line)
        if not line.strip() or opens:
            if current:
                out.append(" ".join(current))
            current = []
        if line.strip():
            current.append(line.strip())
    if current:
        out.append(" ".join(current))
    return out


marked_blocks = [block for block in blocks(lines) if "[flagged]" in block.lower()]
for claim, pattern in (
    ("the manifest schema", r"\bschema\b"),
):
    if not any(re.search(pattern, block, re.I) for block in marked_blocks):
        complaints.append(f"README.md does not mark {claim} as [flagged]")

# --- every path the readme cites ---------------------------------------------

# A citation is a path when its first segment names something at the repository
# root. That keeps `origin/main` and `github/codeql-action` out, and keeps every
# path into this repository in.
top = set(os.listdir("."))
spans = re.findall(r"`([^`\n]+)`", readme)
cited = set()
for span in spans:
    candidate = span.strip().rstrip("/")
    if not candidate or " " in candidate or "://" in candidate:
        continue
    first = candidate.split("/")[0]
    if first in top and (("/" in candidate) or os.path.exists(candidate)):
        cited.add(candidate)

missing = sorted(path for path in cited if not os.path.exists(path))
for path in missing:
    complaints.append(f"README.md cites `{path}`, which does not exist")
if len(cited) < 5:
    complaints.append(
        f"README.md cites {len(cited)} paths into this repository, which is too few for the check "
        "to mean anything; point a reader at the files"
    )

# --- every weeder command the readme cites -------------------------------------

# The binary's own help is the only list of what weeder answers. A cited command
# is walked against it: the subcommands have to exist, the flags have to be
# printed at the node they are passed to, and where clap prints the values a
# flag or an argument accepts, the cited value has to be one of them.


def section(text, heading):
    out = []
    seen = False
    for line in text.splitlines():
        if line.strip() == heading:
            seen = True
            continue
        if seen:
            if not line.strip():
                break
            out.append(line.strip())
    return out


def choices(line):
    found = re.search(r"\[possible values: ([^\]]+)\]", line)
    return {value.strip() for value in found.group(1).split(",")} if found else None


class Node:
    def __init__(self, text):
        self.commands = {
            line.split()[0] for line in section(text, "Commands:") if line.split()
        } - {"help"}
        self.flags = {}
        for line in section(text, "Options:"):
            values = choices(line)
            takes_value = bool(re.search(r"^(-\S+,? )*--\S+ <", line))
            for word in line.split():
                if not word.startswith("-"):
                    break
                self.flags[word.rstrip(",")] = (takes_value, values)
        self.positionals = [choices(line) for line in section(text, "Arguments:")]


nodes = {}


def node_at(path):
    key = tuple(path)
    if key not in nodes:
        printed = subprocess.run(
            [BINARY] + list(path) + ["--help"], capture_output=True, text=True
        )
        nodes[key] = Node(printed.stdout) if printed.returncode == 0 else None
    return nodes[key]


commands = []
for block in re.findall(r"```[a-z]*\n(.*?)```", readme, re.S):
    commands += [line.strip() for line in block.splitlines()]
commands += [span.strip() for span in spans]

cited_commands = sorted({c for c in commands if re.match(r"^weeder( |$)", c)})
for command in cited_commands:
    # A shell line, with redirections and anything downstream of a pipe left out.
    tokens = re.split(r"[|>]", command)[0].split()[1:]
    walked = []
    node = node_at([])
    while tokens and node is not None and node.commands and not tokens[0].startswith("-"):
        step = tokens[0]
        if step not in node.commands:
            complaints.append(
                f"README.md cites `{command}`, and `weeder {' '.join(walked)}`".rstrip()
                + f" has no `{step}` command"
            )
            node = None
            break
        walked.append(step)
        tokens = tokens[1:]
        node = node_at(walked)
        if node is None:
            complaints.append(f"README.md cites `{command}`, which the binary does not answer")
    if node is None:
        continue

    where = ("weeder " + " ".join(walked)).strip()
    positional = 0
    while tokens:
        token = tokens.pop(0)
        if token == "--":
            break
        if token.startswith("-"):
            flag, _, inline = token.partition("=")
            if flag not in node.flags:
                complaints.append(f"README.md cites `{command}`, and `{where}` prints no {flag}")
                continue
            takes_value, allowed = node.flags[flag]
            value = inline if inline else (tokens.pop(0) if takes_value and tokens else None)
            if takes_value and value is None:
                complaints.append(f"README.md cites `{command}`, and {flag} takes a value")
            elif allowed is not None and value is not None and value not in allowed:
                complaints.append(
                    f"README.md cites `{command}`, and `{where}` takes {flag} "
                    f"{' or '.join(sorted(allowed))}, not {value}"
                )
            continue
        allowed = node.positionals[positional] if positional < len(node.positionals) else None
        if allowed is not None and token not in allowed:
            complaints.append(
                f"README.md cites `{command}`, and `{where}` takes "
                f"{' or '.join(sorted(allowed))}, not {token}"
            )
        positional += 1

# The garden footer, until weeder has a page of its own to carry it.
if "a plotplot garden tool" not in readme:
    complaints.append(
        "README.md carries no garden footer: every bed names the garden it belongs to, and "
        "weeder has no page yet for it to sit on"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"MIT, {len(cited)} paths cited and present, {len(cited_commands)} weeder commands answered")
PY

if [ "$status" -eq 0 ]; then
  echo "LICENSE is MIT; README.md says what weeder is, flags what the owner has not settled, and cites nothing that does not exist"
fi
exit "$status"
