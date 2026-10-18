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
$junction = Join-Path $fixtures "guard-test-link"
$buildGen = Join-Path $root "build/fixture-generator"
$buildBak = "$buildGen.guardtest-bak"
$failures = 0

function New-Foreign {
    New-Item -ItemType Directory -Force (Join-Path $foreign "generated") | Out-Null
    Set-Content (Join-Path $foreign "sentinel.txt") "keep"
    Set-Content (Join-Path $foreign "generated/sentinel.txt") "keep"
    # a marker that would make a bypassed guard consider the directory deletable
    Set-Content (Join-Path $foreign "generated/manifest.json") '{"task": "MIG-001"}'
}
function Remove-Link([string]$p) { if (Test-Path -LiteralPath $p) { [IO.Directory]::Delete($p, $false) } }  # non-recursive: removes only the link
function Expect-Refusal([string]$name, [scriptblock]$act) {
    $out = & pwsh -NoProfile -File $run @args 2>&1 | Out-String
    $code = $LASTEXITCODE
    $intact = (Test-Path (Join-Path $foreign "sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/manifest.json"))
    if ($code -ne 0 -and $out -match "refusing" -and $intact) { "PASS  $name" }
    else { "FAIL  $name (exit=$code, foreign intact=$intact)"; $script:failures++ }
}

try {
    New-Foreign
    New-Item -ItemType Junction -Path $junction -Target $foreign | Out-Null

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/guard-test-link/generated" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "generated/manifest.json")) -and (Test-Path (Join-Path $foreign "generated/sentinel.txt"))
    if ($ok) { "PASS  existing target behind junction" } else { "FAIL  existing target behind junction"; $failures++ }

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/guard-test-link/not-yet/generated" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and -not (Test-Path (Join-Path $foreign "not-yet"))
    if ($ok) { "PASS  non-existing target behind junction" } else { "FAIL  non-existing target behind junction"; $failures++ }

    $r = & pwsh -NoProfile -File $run -Out "fixtures/legacy-formats/guard-test-link" 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "sentinel.txt"))
    if ($ok) { "PASS  junction itself as target" } else { "FAIL  junction itself as target"; $failures++ }
    Remove-Link $junction

    # build/fixture-generator (cleanup target) as a junction
    if (Test-Path -LiteralPath $buildGen) { Move-Item -LiteralPath $buildGen -Destination $buildBak }
    New-Item -ItemType Directory -Force (Join-Path $root "build") | Out-Null
    New-Item -ItemType Junction -Path $buildGen -Target $foreign | Out-Null
    $r = & pwsh -NoProfile -File $run 2>&1 | Out-String
    $ok = ($LASTEXITCODE -ne 0) -and ($r -match "refusing") -and (Test-Path (Join-Path $foreign "sentinel.txt")) -and (Test-Path (Join-Path $foreign "generated/manifest.json"))
    if ($ok) { "PASS  build/fixture-generator is a junction" } else { "FAIL  build/fixture-generator is a junction"; $failures++ }
}
finally {
    Remove-Link $junction
    Remove-Link $buildGen
    if (Test-Path -LiteralPath $buildBak) { Move-Item -LiteralPath $buildBak -Destination $buildGen }
    if (Test-Path -LiteralPath $foreign) { Remove-Item -LiteralPath $foreign -Recurse -Force }
}
if ($failures -gt 0) { "$failures test(s) failed"; exit 1 }
"all guard tests passed"
