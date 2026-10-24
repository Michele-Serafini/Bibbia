param([switch]$Run)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
$staging = Join-Path $root ('artifacts/staging-' + [Guid]::NewGuid())
Push-Location $root
try {
    if (-not $IsWindows) { throw 'WinUI 3 richiede Windows. Su Linux usa cargo test --locked per il motore.' }
    foreach ($command in @('cargo', 'rustup', 'dotnet')) {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Prerequisito mancante: $command. Consulta README.md." }
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'Installa Visual Studio 2022 Build Tools con gli strumenti C++ e UWP/WinUI.' }
    $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.Component.MSBuild -property installationPath
    if (-not $visualStudio) { throw 'Installazione Visual Studio con MSBuild non trovata.' }
    $msbuild = Join-Path $visualStudio 'MSBuild/Current/Bin/MSBuild.exe'
    if (-not (Test-Path $msbuild)) { throw 'MSBuild di Visual Studio non trovato.' }
    rustup target add x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Installazione del target Rust fallita.' }
    cargo test --locked --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Test Rust falliti.' }
    cargo build --release --locked --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Compilazione Rust fallita. Verifica gli strumenti C++ di Visual Studio.' }
    # WinUI 1.6 PRI tasks are supplied by Visual Studio, not dotnet/MSBuild.
    & $msbuild app/Bibbia.Windows.csproj /restore /t:Publish /p:Configuration=Release /p:Platform=x64 /p:RestoreLockedMode=true "/p:PublishDir=$staging/" /m /nologo /v:minimal
    if ($LASTEXITCODE -ne 0) { throw 'Compilazione WinUI fallita.' }
    Copy-Item core/data/LICENSE.bible-data "$staging/LICENSE.bible-data" -Force
    Copy-Item core/data/diodati-source.json "$staging/diodati-source.json" -Force
    Copy-Item THIRD_PARTY_NOTICES.md "$staging/THIRD_PARTY_NOTICES.md" -Force
    Copy-Item LICENSE "$staging/LICENSE" -Force
    $destination = Join-Path $root 'artifacts/Bibbia'
    if (Test-Path $destination) {
        if ((Get-Item $destination).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'La cartella di pubblicazione non può essere un collegamento.' }
        Remove-Item $destination -Recurse -Force
    }
    Move-Item $staging $destination
    Write-Host 'Build pronta in artifacts/Bibbia. Distribuisci l''intera cartella.'
    if ($Run) { Start-Process (Join-Path $root 'artifacts/Bibbia/Bibbia.Windows.exe') }
}
finally {
    if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
    Pop-Location
}
