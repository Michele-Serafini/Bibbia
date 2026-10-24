param([string]$Compiler)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('bibbia-manifest-test-' + [Guid]::NewGuid())
try {
    $payload = Join-Path $fixture 'payload'
    $output = Join-Path $fixture 'manifest'
    New-Item (Join-Path $payload 'locale $italiano/sub') -ItemType Directory -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $payload 'locale $italiano/sub/nota.txt'), 'Fixture di compilazione: non contiene Bibbia.')
    [IO.File]::WriteAllText((Join-Path $payload 'license.txt'), 'Fixture')
    [IO.File]::WriteAllText((Join-Path $payload 'cost$1.txt'), 'Fixture')
    & "$root/scripts/write-installer-manifest.ps1" -Payload $payload -OutputDirectory $output
    $install = Get-Content (Join-Path $output 'install-files.nsh') -Raw
    $uninstall = Get-Content (Join-Path $output 'uninstall-files.nsh') -Raw
    if (-not $install.Contains('locale $$italiano\sub') -or -not $uninstall.Contains('locale $$italiano\sub\nota.txt')) { throw 'Percorso con dollaro non protetto.' }
    if ($uninstall.Contains('/r') -or $uninstall.Contains('BibbiaRust')) { throw 'Rimozione troppo ampia.' }
    if ($uninstall.IndexOf('RMDir "$INSTDIR\locale $$italiano\sub"') -gt $uninstall.IndexOf('RMDir "$INSTDIR\locale $$italiano"')) { throw 'Ordine rimozione cartelle errato.' }
    [IO.File]::WriteAllText((Join-Path $payload 'Uninstall.exe'), 'riservato')
    $rejected = $false
    try { & "$root/scripts/write-installer-manifest.ps1" -Payload $payload -OutputDirectory $output }
    catch { $rejected = $true }
    if (-not $rejected) { throw 'Il nome riservato Uninstall.exe deve essere rifiutato.' }
    Remove-Item (Join-Path $payload 'Uninstall.exe')
    if ($Compiler) {
        # Compile the real NSIS script with a text-only fixture, never the app.
        # This temporary executable tests syntax only and is deleted below.
        $binary = Join-Path $fixture 'syntax-test-not-application.exe'
        $prefix = if ($IsWindows) { '/' } else { '-' }
        & $Compiler ($prefix + 'INPUTCHARSET') UTF8 ($prefix + 'V3') ($prefix + 'DAPP_VERSION=0.1.0') ($prefix + "DPROJECT_DIR=$root") ($prefix + "DMANIFEST_DIR=$output") ($prefix + "DOUTPUT_FILE=$binary") "$root/installer/Bibbia.nsi"
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path $binary)) { throw 'Compilazione dello script NSIS fallita.' }
    }
    Write-Host 'Manifest installer: percorsi, rimozione limitata e nomi riservati verificati.'
}
finally { if (Test-Path $fixture) { Remove-Item $fixture -Recurse -Force } }
