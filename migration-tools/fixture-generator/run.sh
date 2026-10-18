#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Regenerates fixtures/legacy-formats/generated from the unmodified baseline classes.
# Usage (repo root, JDK 17+, Gradle, Python 3 on PATH):  sh migration-tools/fixture-generator/run.sh [outdir]
# outdir must resolve below <repo>/fixtures/legacy-formats/; an existing non-empty directory is only
# replaced if it carries this generator's manifest.json.
set -eu
cd "$(dirname "$0")/../.."
ROOT="$(pwd -P)"
OUT="${1:-fixtures/legacy-formats/generated}"
case "$OUT" in /*) ABS="$OUT" ;; *) ABS="$ROOT/$OUT" ;; esac
# No component below the repository root may be a symlink (checked before anything is deleted).
assert_plain() {
  _cur="$ROOT"; _rest="${1#"$ROOT"/}"
  while [ -n "$_rest" ]; do
    _part="${_rest%%/*}"; case "$_rest" in */*) _rest="${_rest#*/}" ;; *) _rest="" ;; esac
    _cur="$_cur/$_part"
    if [ -L "$_cur" ]; then echo "refusing path '$1': '$_cur' is a symlink" >&2; exit 1; fi
    if [ -e "$_cur" ] && [ ! -d "$_cur" ]; then echo "refusing path '$1': '$_cur' is not a directory" >&2; exit 1; fi
  done
}
# No mountpoint may be, contain or lie beneath the paths we create or delete below. Uses the kernel's mount table
# (/proc/self/mountinfo) instead of device ids, so bind mounts of the same filesystem are detected as well.
# Fails closed when the table is unavailable. Octal escapes in mountinfo are decoded; the paths are passed via ENVIRON, not `awk -v`, so literal backslashes in names stay intact.
assert_no_mount() {
  [ -r /proc/self/mountinfo ] || { echo "refusing '$1': cannot verify mountpoints (/proc/self/mountinfo unreadable)" >&2; exit 1; }
  _hit="$(MIG_ROOT="$ROOT/" MIG_P="$1" awk '
    function dec(s,   o, i, c) { o = ""; for (i = 1; i <= length(s); i++) { c = substr(s, i, 1)
      if (c == "\\" && substr(s, i + 1, 3) ~ /^[0-7][0-7][0-7]$/) { o = o sprintf("%c", substr(s, i + 1, 1) * 64 + substr(s, i + 2, 1) * 8 + substr(s, i + 3, 1)); i += 3 } else o = o c }
      return o }
    BEGIN { root = ENVIRON["MIG_ROOT"]; p = ENVIRON["MIG_P"] }   # ENVIRON: awk -v would process backslash escapes in paths
    { m = dec($5)
      if (index(m, root) == 1 && (m == p || index(p, m "/") == 1 || index(m, p "/") == 1)) { print m; exit } }' /proc/self/mountinfo)" \
    || { echo "refusing '$1': mount table could not be evaluated" >&2; exit 1; }
  if [ -n "$_hit" ]; then echo "refusing path '$1': mountpoint '$_hit' is, contains or is a parent of it" >&2; exit 1; fi
}
case "$ABS" in
  */..|*/../*|*/.|*/./*|*//*) echo "refusing output '$ABS': relative or empty path component" >&2; exit 1 ;;
  "$ROOT/fixtures/legacy-formats/"?*) ;;
  *) echo "refusing output '$ABS': must be below $ROOT/fixtures/legacy-formats/" >&2; exit 1 ;;
esac
assert_plain "$ABS"
assert_plain "$ROOT/build/fixture-generator"
assert_no_mount "$ABS"
assert_no_mount "$ROOT/build/fixture-generator"
PARENT="$(dirname "$ABS")"
[ -d "$PARENT" ] || { echo "parent of output does not exist: $PARENT" >&2; exit 1; }
TARGET="$(cd "$PARENT" && pwd -P)/$(basename "$ABS")"
case "$TARGET" in
  "$ROOT/fixtures/legacy-formats/"?*) ;;
  *) echo "refusing output '$TARGET': must be below $ROOT/fixtures/legacy-formats/" >&2; exit 1 ;;
esac
assert_no_mount "$TARGET"
case "$(basename "$TARGET")" in .|..) echo "refusing output name" >&2; exit 1 ;; esac
if [ -e "$TARGET" ] || [ -L "$TARGET" ]; then
  if [ -L "$TARGET" ] || [ ! -d "$TARGET" ]; then echo "refusing output '$TARGET': not a plain directory" >&2; exit 1; fi
  if [ -n "$(ls -A "$TARGET")" ] && ! grep -q '"task": "MIG-001"' "$TARGET/manifest.json" 2>/dev/null; then
    echo "refusing to delete '$TARGET': not a generated fixture directory" >&2; exit 1
  fi
fi
if ! git diff --quiet b7753cea18b2ac2cc57e29f926901997a4de8570 -- src updater build.gradle; then
  echo "src/updater/build.gradle differ from the baseline commit (or git failed)" >&2; exit 1
fi
gradle compileJava --console=plain -q
GEN=build/fixture-generator
rm -rf --one-file-system "$GEN"
mkdir -p "$GEN"
javac -encoding UTF-8 -cp build/classes/java/main -d "$GEN" migration-tools/fixture-generator/FixtureGenerator.java
[ -f "$GEN/FixtureGenerator.class" ] || { echo "generator was not compiled" >&2; exit 1; }
rm -rf --one-file-system "$TARGET"
java -Xmx768m -Dbaseline.classes=build/classes/java/main -cp "build/classes/java/main:$GEN" FixtureGenerator generate "$TARGET"
python3 -I migration-tools/fixture-generator/verify.py "$TARGET" --write-inventory
