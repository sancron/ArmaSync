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
PARENT="$(dirname "$ABS")"
[ -d "$PARENT" ] || { echo "parent of output does not exist: $PARENT" >&2; exit 1; }
TARGET="$(cd "$PARENT" && pwd -P)/$(basename "$ABS")"
case "$TARGET" in
  "$ROOT/fixtures/legacy-formats/"?*) ;;
  *) echo "refusing output '$TARGET': must be below $ROOT/fixtures/legacy-formats/" >&2; exit 1 ;;
esac
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
rm -rf "$GEN"
mkdir -p "$GEN"
javac -encoding UTF-8 -cp build/classes/java/main -d "$GEN" migration-tools/fixture-generator/FixtureGenerator.java
[ -f "$GEN/FixtureGenerator.class" ] || { echo "generator was not compiled" >&2; exit 1; }
rm -rf "$TARGET"
java -Xmx768m -Dbaseline.classes=build/classes/java/main -cp "build/classes/java/main:$GEN" FixtureGenerator generate "$TARGET"
python3 -I migration-tools/fixture-generator/verify.py "$TARGET" --write-inventory
