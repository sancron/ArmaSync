# SPDX-License-Identifier: AGPL-3.0-only
# Regenerates fixtures/legacy-formats/generated from the unmodified baseline classes.
# Usage (repo root, JDK 17+, Gradle, Python 3 on PATH):  pwsh migration-tools/fixture-generator/run.ps1 [-Out <dir>]
# -Out must resolve to a directory below <repo>/fixtures/legacy-formats/; no path component between the repo root and
# the target (nor build/fixture-generator) may be a link, junction or reparse point. An existing
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
# Every existing component from the trusted repository root down to the path must be a plain directory:
# no junction, symlink or other reparse point, even if the final target does not exist yet.
function Assert-PlainPath([string]$base, [string]$full) {
    $rel = $full.Substring($base.TrimEnd([char[]]'\/').Length).TrimStart([char[]]'\/')
    $cur = $base.TrimEnd([char[]]'\/')
    foreach ($part in ($rel -split '[\\/]' | Where-Object { $_ })) {
        if ($part -eq '..' -or $part -eq '.') { throw "refusing path '$full': relative component" }
        $cur = Join-Path $cur $part
        if (Test-Path -LiteralPath $cur) {
            $it = Get-Item -LiteralPath $cur -Force
            if ($it.LinkType -or ($it.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "refusing path '$full': '$cur' is a link/reparse point" }
            if (-not $it.PSIsContainer) { throw "refusing path '$full': '$cur' is not a directory" }
        }
    }
}
Assert-PlainPath $root $target
$gen = Join-Path $root "build/fixture-generator"
Assert-PlainPath $root $gen
if (Test-Path -LiteralPath $target) {
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
