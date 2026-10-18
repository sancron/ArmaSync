# SPDX-License-Identifier: AGPL-3.0-only
# Rejection tests for run.ps1's output/cleanup guards. Creates junctions to a throw-away "foreign" directory
# and checks that run.ps1 refuses and leaves the foreign content untouched. Never deletes through a link.
# Usage (repo root): pwsh migration-tools/fixture-generator/test-output-guard.ps1
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot/../..").Path
Set-Location $root
$run = Join-Path $PSScriptRoot "run.ps1"
$foreign = Join-Path ([IO.Path]::GetTempPath()) ("mig001-foreign-" + [guid]::NewGuid().ToString("N"))
$fixtures = Join-Path $root "fixtures/legacy-formats"
$token = [guid]::NewGuid().ToString("N")
$linkName = "guard-test-$token"
$junction = Join-Path $fixtures $linkName
$buildGen = Join-Path $root "build/fixture-generator"
$buildBak = "$buildGen.guardtest-$token-bak"
$failures = 0
$createdLinks = [System.Collections.Generic.List[string]]::new()   # only objects this test created as junctions
$movedBuild = $false
$createdBuildDir = $false

function New-Foreign {
    New-Item -ItemType Directory -Force (Join-Path $foreign "generated") | Out-Null
    Set-Content (Join-Path $foreign "sentinel.txt") "keep"
    Set-Content (Join-Path $foreign "generated/sentinel.txt") "keep"
    # a marker that would make a bypassed guard consider the directory deletable
    Set-Content (Join-Path $foreign "generated/manifest.json") '{"task": "MIG-001"}'
}
function New-TestJunction([string]$path, [string]$target) {
    if (Test-Path -LiteralPath $path) { throw "refusing to create test junction: '$path' already exists" }
    New-Item -ItemType Junction -Path $path -Target $target | Out-Null
    $script:createdLinks.Add($path)
}
# Removes a junction this test created; never a normal directory and never recursively.
function Remove-TestJunction([string]$path) {
    if (-not $script:createdLinks.Contains($path)) { throw "not a link created by this test: '$path'" }
    $item = Get-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    if ($null -eq $item) { $script:createdLinks.Remove($path) | Out-Null; return }
    if (-not ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { Write-Warning "'$path' is not a link any more; left untouched"; return }
    [IO.Directory]::Delete($path, $false)   # on a reparse point this removes only the link
    $script:createdLinks.Remove($path) | Out-Null
}
function Expect-Refusal([string]$name, [scriptblock]$act) {
    $out = & pwsh -NoProfile -File $run @args 2>&1 | Out-String
    $code = $LASTEXITCODE
    $intact = (Test-Path (Join-Path $foreign "sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/manifest.json"))
    if ($code -ne 0 -and $out -match "refusing" -and $intact) { "PASS  $name" }
    else { "FAIL  $name (exit=$code, foreign intact=$intact)"; $script:failures++ }
}

try {
    New-Foreign
    New-TestJunction $junction $foreign

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/$linkName/generated" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "generated/manifest.json")) -and (Test-Path (Join-Path $foreign "generated/sentinel.txt"))
    if ($ok) { "PASS  existing target behind junction" } else { "FAIL  existing target behind junction"; $failures++ }

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/$linkName/not-yet/generated" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and -not (Test-Path (Join-Path $foreign "not-yet"))
    if ($ok) { "PASS  non-existing target behind junction" } else { "FAIL  non-existing target behind junction"; $failures++ }

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/$linkName" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "sentinel.txt"))
    if ($ok) { "PASS  junction itself as target" } else { "FAIL  junction itself as target"; $failures++ }
    Remove-TestJunction $junction

    # build/fixture-generator (cleanup target) as a junction
    if (Test-Path -LiteralPath $buildBak) { throw "refusing: backup path '$buildBak' is occupied" }
    if (Test-Path -LiteralPath $buildGen) {
        if ((Get-Item -LiteralPath $buildGen -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "refusing: '$buildGen' is already a link" }
        Move-Item -LiteralPath $buildGen -Destination $buildBak; $movedBuild = $true
    }
    if (-not (Test-Path -LiteralPath (Join-Path $root "build"))) { New-Item -ItemType Directory (Join-Path $root "build") | Out-Null; $createdBuildDir = $true }
    New-TestJunction $buildGen $foreign
    $r = & pwsh -NoProfile -File $run 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/manifest.json"))
    if ($ok) { "PASS  build/fixture-generator is a junction" } else { "FAIL  build/fixture-generator is a junction"; $failures++ }
}
finally {
    foreach ($l in @($createdLinks)) { Remove-TestJunction $l }
    if ($movedBuild -and -not (Test-Path -LiteralPath $buildGen)) { Move-Item -LiteralPath $buildBak -Destination $buildGen }
    elseif ($movedBuild) { Write-Warning "backup left at '$buildBak'; '$buildGen' is occupied" }
    if ($createdBuildDir) { try { [IO.Directory]::Delete((Join-Path $root "build"), $false) } catch {} }   # only if empty
    if (Test-Path -LiteralPath $foreign) { Remove-Item -LiteralPath $foreign -Recurse -Force }
}
if ($failures -gt 0) { "$failures test(s) failed"; exit 1 }
"all guard tests passed"
