#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Regenerates fixtures/legacy-formats from the unmodified baseline classes.
# Usage (repo root, JDK 17+, Gradle, Python 3 on PATH):  sh migration-tools/fixture-generator/run.sh [outdir]
set -eu
OUT="${1:-fixtures/legacy-formats/generated}"
cd "$(dirname "$0")/../.."
if [ -n "$(git diff --name-only b7753cea18b2ac2cc57e29f926901997a4de8570 -- src updater build.gradle)" ]; then
  echo "src/updater/build.gradle differ from the baseline commit" >&2; exit 1
fi
gradle compileJava --console=plain -q
GEN=build/fixture-generator
mkdir -p "$GEN"
javac -encoding UTF-8 -cp build/classes/java/main -d "$GEN" migration-tools/fixture-generator/FixtureGenerator.java
rm -rf "$OUT"
java -Xmx768m -Dbaseline.classes=build/classes/java/main -cp "build/classes/java/main:$GEN" FixtureGenerator generate "$OUT"
python3 -I migration-tools/fixture-generator/verify.py "$OUT" --write-inventory
