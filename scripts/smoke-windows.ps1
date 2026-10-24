$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
$dll = Join-Path $root 'artifacts/Bibbia/bible_core.dll'
if (-not (Test-Path $dll)) { throw 'Compila prima con scripts/build.ps1.' }
Push-Location $root
try {
    dotnet run --project tests/Bibbia.InteropSmoke -- $dll
    if ($LASTEXITCODE -ne 0) { throw 'Verifica del collegamento C# / Rust fallita.' }
}
finally { Pop-Location }
