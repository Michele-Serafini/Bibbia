param([Parameter(Mandatory)][string]$SourceDirectory,[Parameter(Mandatory)][string]$Destination)
$ErrorActionPreference='Stop'
Push-Location $SourceDirectory
try {
    $env:CARGO_TARGET_DIR=Join-Path $SourceDirectory 'target'
    cargo build --release --locked --target x86_64-pc-windows-msvc
    if($LASTEXITCODE -ne 0){throw 'Baseline Rust build failed'}
    $locator=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $installation=& $locator -latest -products '*' -requires Microsoft.Component.MSBuild -property installationPath
    $compiler=Join-Path $installation 'MSBuild/Current/Bin/MSBuild.exe'
    & $compiler app/Bibbia.Windows.csproj /restore /t:Publish /p:Configuration=Release /p:Platform=x64 /p:RestoreLockedMode=true "/p:PublishDir=$Destination/" /m /nologo /v:minimal
    if($LASTEXITCODE -ne 0){throw 'Baseline WinUI build failed'}
} finally {Pop-Location}
