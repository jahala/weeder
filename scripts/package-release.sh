#!/usr/bin/env bash
# Package one platform's release artifact, in the shape the garden's lockfile
# pins and the stem unpacks.
#
# The contract, settled between the stem builder and the contracts loop on
# 2026-09-08: one `<name>-<triple>.tar.gz` per platform, carrying at its top
# level the executable `install.binary_name` names (defaulting to `faces.cli`),
# `garden.json`, and the skill at the path `faces.skill` names, with a
# `<asset>.sha256` published beside it. `plotplot init` reads the manifest out
# of the verified artifact rather than off a raw file at a tag, so a tarball
# that carries only a binary leaves the umbrella nothing to read.
#
# Every name here comes out of `garden.json`. The manifest is what the umbrella
# reads and what the lock pins, so it is also what the packaging obeys; a name
# spelled a second time in this file is a name that can disagree with itself.
#
# usage: bash scripts/package-release.sh <target-triple> <binary-dir> <out-dir>
#   <target-triple>  the Rust target the binary was built for, which names the asset
#   <binary-dir>     where the built executable is, `target/<triple>/release` in the workflow
#   <out-dir>        where the asset and its digest are written
set -euo pipefail

target="${1:?a target triple is required}"
binary_dir="${2:?a directory holding the built executable is required}"
out_dir="${3:?a directory to write the asset into is required}"

root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$root/garden.json"

command -v node >/dev/null 2>&1 || {
  echo "node is not on PATH, so garden.json cannot be read and the asset cannot be named" >&2
  exit 3
}
[ -f "$manifest" ] || { echo "$manifest is missing: nothing names what goes in the artifact" >&2; exit 3; }

field() {
  node -e '
    const manifest = require(process.argv[1]);
    const read = (path) => path.split(".").reduce((value, key) => (value ?? {})[key], manifest);
    const value = read(process.argv[2]) ?? (process.argv[3] ? read(process.argv[3]) : undefined);
    if (typeof value !== "string" || value === "") {
      console.error(`garden.json carries no ${process.argv[2]}`);
      process.exit(3);
    }
    process.stdout.write(value);
  ' "$manifest" "$1" "${2-}"
}

name="$(field name)"
version="$(field version)"
skill="$(field faces.skill)"
# The executable inside the artifact is `install.binary_name` where the manifest
# names one, and the cli's own name where it does not.
binary="$(field install.binary_name faces.cli)"
# Windows carries the suffix its loader insists on; the manifest names the
# executable, not the platform's spelling of it.
case "$target" in
  *windows*) binary="$binary.exe" ;;
esac

asset="$name-$target.tar.gz"

[ -f "$binary_dir/$binary" ] || {
  echo "$binary_dir/$binary is missing: build $target before packaging it" >&2
  exit 3
}
[ -f "$root/$skill" ] || { echo "$root/$skill is missing: garden.json names it as the skill" >&2; exit 3; }

mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"
stage="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$stage"' EXIT

install_into_stage() {
  local source="$1" destination="$2"
  mkdir -p "$stage/$(dirname "$destination")"
  cp "$source" "$stage/$destination"
}
install_into_stage "$binary_dir/$binary" "$binary"
install_into_stage "$manifest" "garden.json"
install_into_stage "$root/$skill" "$skill"
chmod 755 "$stage/$binary"

# The members are named rather than swept, so the archive holds what the
# contract lists and nothing a stray file in the staging directory added.
members=()
while IFS= read -r member; do
  members+=("$member")
done < <(printf '%s\n%s\n%s\n' "$binary" "garden.json" "$skill" | cut -d/ -f1 | sort -u)
# macOS tar writes a second `._<name>` entry beside every file that carries an
# extended attribute, and every file copied on this platform carries one. The
# artifact holds what the contract lists, so the attributes are left behind.
COPYFILE_DISABLE=1 tar czf "$out_dir/$asset" -C "$stage" "${members[@]}"

digest() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1"
  else
    echo "neither sha256sum nor shasum is on PATH, so the asset cannot be checksummed" >&2
    exit 3
  fi
}
# Written from inside the directory, so the digest file names the asset beside
# it and `sha256sum -c` reads it wherever the release is downloaded to.
(cd "$out_dir" && digest "$asset" > "$asset.sha256")

echo "$asset: $binary, garden.json and $skill, weeder $version for $target"
