param([int]$Port = 8765)
$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot 'server.py'
$python = Get-Command py -ErrorAction SilentlyContinue
if ($python) {
    & $python.Source -3 $scriptPath --port $Port
    exit $LASTEXITCODE
}
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { throw 'Python 3 is required.' }
& $python.Source $scriptPath --port $Port
exit $LASTEXITCODE
