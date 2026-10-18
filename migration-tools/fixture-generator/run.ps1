# SPDX-License-Identifier: AGPL-3.0-only
# Regenerates fixtures/legacy-formats from the unmodified baseline classes.
# Usage (repo root, JDK 17+ and Gradle on PATH):  pwsh migration-tools/fixture-generator/run.ps1 [-Out <dir>]
param([string]$Out = "fixtures/legacy-formats/generated")
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $root
if ((git diff --name-only b7753cea18b2ac2cc57e29f926901997a4de8570 -- src updater build.gradle)) {
    throw "src/updater/build.gradle differ from the baseline commit; fixtures must be generated from the baseline classes."
}
gradle compileJava --console=plain -q
$gen = Join-Path $root "build/fixture-generator"
New-Item -ItemType Directory -Force $gen | Out-Null
javac -encoding UTF-8 -cp build/classes/java/main -d $gen migration-tools/fixture-generator/FixtureGenerator.java
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
java -Xmx768m "-Dbaseline.classes=build/classes/java/main" -cp "build/classes/java/main$([IO.Path]::PathSeparator)$gen" FixtureGenerator generate $Out
if ($LASTEXITCODE -ne 0) { throw "generator failed" }
python -I migration-tools/fixture-generator/verify.py $Out --write-inventory
if ($LASTEXITCODE -ne 0) { throw "independent verification failed" }
