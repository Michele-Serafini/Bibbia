param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    if (-not $IsWindows) { throw 'La build completa WinUI richiede Windows x64.' }
    if (-not $SkipBuild) { & "$PSScriptRoot/build.ps1" }
    $payload = Join-Path $root 'artifacts/Bibbia'
    $required = @('Bibbia.Windows.exe', 'Bibbia.Windows.dll', 'Bibbia.Windows.runtimeconfig.json', 'Bibbia.Windows.deps.json', 'bible_core.dll', 'Microsoft.ui.xaml.dll', 'Bibbia.Windows.pri', 'coreclr.dll', 'Assets/Bibbia.ico', 'Assets/Fonts/AdwaitaSans-Regular.ttf', 'Assets/Fonts/SourceSerif4-Regular.ttf', 'LICENSE.Adwaita', 'LICENSE.SourceSerif', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'LICENSE.cei2008.txt', 'LICENSE.gutenberg.txt', 'cei2008-source.json', 'extended-source.json')
    foreach ($file in $required) {
        if (-not (Test-Path (Join-Path $payload $file) -PathType Leaf)) { throw "Pubblicazione incompleta: manca $file." }
    }
    $compiler = Get-Command makensis.exe -ErrorAction SilentlyContinue
    $compilerPath = if ($compiler) { $compiler.Source } else { $null }
    if (-not $compilerPath) {
        foreach ($base in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
            if ($base -and (Test-Path (Join-Path $base 'NSIS/makensis.exe'))) { $compilerPath = Join-Path $base 'NSIS/makensis.exe'; break }
        }
    }
    if (-not $compilerPath) { throw 'Installa NSIS 3 (https://nsis.sourceforge.io/Download), oppure usa il workflow GitHub Actions.' }
    [xml]$project = Get-Content app/Bibbia.Windows.csproj -Raw
    $version = [string]$project.Project.PropertyGroup.Version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Versione applicazione non valida.' }
    $manifestDir = Join-Path $root 'artifacts/installer-manifest'
    New-Item $manifestDir -ItemType Directory -Force | Out-Null
    & "$PSScriptRoot/write-installer-manifest.ps1" -Payload $payload -OutputDirectory $manifestDir
    $outputDir = Join-Path $root 'artifacts/installer'
    New-Item $outputDir -ItemType Directory -Force | Out-Null
    $output = Join-Path $outputDir "Bibbia-Setup-$version-x64.exe"
    # Remove only the previous generated file, so a failed compiler cannot leave
    # a stale installer that looks like the result of this build.
    if (Test-Path $output) { Remove-Item $output -Force }
    & $compilerPath /INPUTCHARSET UTF8 /V3 "/DAPP_VERSION=$version" "/DPROJECT_DIR=$root" "/DMANIFEST_DIR=$manifestDir" "/DOUTPUT_FILE=$output" installer/Bibbia.nsi
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $output)) { throw 'Creazione installer fallita.' }
    $hash = (Get-FileHash $output -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $(Split-Path $output -Leaf)" | Set-Content "$output.sha256" -Encoding utf8NoBOM
    Write-Host "Installer generato: $output"
}
finally { Pop-Location }
