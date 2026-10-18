# SPDX-License-Identifier: AGPL-3.0-only
# Regenerates fixtures/legacy-formats/generated from the unmodified baseline classes.
# Usage (repo root, JDK 17+, Gradle, Python 3 on PATH):  pwsh migration-tools/fixture-generator/run.ps1 [-Out <dir>]
# -Out must resolve to a directory below <repo>/fixtures/legacy-formats/ (and not be a link). An existing
# directory is only replaced if it carries this generator's manifest.json; anything else is refused.
param([string]$Out = "fixtures/legacy-formats/generated")
$ErrorActionPreference = "Stop"
function Check([string]$what) { if ($LASTEXITCODE -ne 0) { throw "$what failed with exit code $LASTEXITCODE" } }

$root = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $root
$allowed = [IO.Path]::GetFullPath((Join-Path $root "fixtures/legacy-formats")).TrimEnd('\','/') + [IO.Path]::DirectorySeparatorChar
$target = [IO.Path]::GetFullPath((Join-Path $root $Out)).TrimEnd('\','/')
if (-not ($target + [IO.Path]::DirectorySeparatorChar).StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase) -or ($target + [IO.Path]::DirectorySeparatorChar) -eq $allowed) {
    throw "refusing output '$target': must be a subdirectory of $allowed"
}
if (Test-Path -LiteralPath $target) {
    $item = Get-Item -LiteralPath $target -Force
    if (-not $item.PSIsContainer -or $item.LinkType -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "refusing output '$target': not a plain directory" }
    $marker = Join-Path $target "manifest.json"
    if ((Get-ChildItem -LiteralPath $target -Force | Measure-Object).Count -gt 0) {
        if (-not ((Test-Path -LiteralPath $marker) -and (Select-String -LiteralPath $marker -Pattern '"task": "MIG-001"' -Quiet))) {
            throw "refusing to delete '$target': not a generated fixture directory"
        }
    }
}

git diff --quiet b7753cea18b2ac2cc57e29f926901997a4de8570 -- src updater build.gradle
if ($LASTEXITCODE -ne 0) { throw "src/updater/build.gradle differ from the baseline commit (or git failed); fixtures must be generated from the baseline classes." }
gradle compileJava --console=plain -q
Check "gradle compileJava"
$gen = Join-Path $root "build/fixture-generator"
if (Test-Path -LiteralPath $gen) { Remove-Item -LiteralPath $gen -Recurse -Force }  # no stale classes
New-Item -ItemType Directory -Force $gen | Out-Null
javac -encoding UTF-8 -cp build/classes/java/main -d $gen migration-tools/fixture-generator/FixtureGenerator.java
Check "javac"
if (-not (Test-Path (Join-Path $gen "FixtureGenerator.class"))) { throw "generator was not compiled" }

if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
java -Xmx768m "-Dbaseline.classes=build/classes/java/main" -cp "build/classes/java/main$([IO.Path]::PathSeparator)$gen" FixtureGenerator generate $target
Check "generator"
python -I migration-tools/fixture-generator/verify.py $target --write-inventory
Check "independent verification"
