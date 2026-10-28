param([Parameter(Mandatory)][string]$Installer)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows) { throw 'Il test richiede Windows.' }
$root = Split-Path $PSScriptRoot -Parent
$installerPath = (Resolve-Path $Installer).Path
$registry = 'HKCU:\Software\BibbiaRust'
$uninstallRegistry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BibbiaRust'
$dataDirectory = Join-Path $env:LOCALAPPDATA 'BibbiaRust'
if ((Test-Path $registry) -or (Test-Path $uninstallRegistry) -or (Test-Path $dataDirectory)) {
    throw 'Il test modifica installazione e registro: usare un runner pulito senza dati Bibbia preesistenti.'
}
$installDirectory = Join-Path ([IO.Path]::GetTempPath()) ('Bibbia installer test ' + [Guid]::NewGuid())
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Bibbia/Bibbia.lnk'
if (Test-Path $shortcut) { throw 'Collegamento Bibbia preesistente: test interrotto.' }
function Invoke-Setup([string]$exe, [string]$arguments) {
    $process = Start-Process $exe -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer terminato con codice $($process.ExitCode)." }
}
try {
    New-Item $dataDirectory -ItemType Directory | Out-Null
    $noteFixture = Join-Path $dataDirectory 'note-da-conservare.txt'
    [IO.File]::WriteAllText($noteFixture, 'Appunti di prova: perché c''è speranza.')
    Invoke-Setup $installerPath "/S /D=$installDirectory"
    foreach ($file in @('Bibbia.Windows.exe', 'bible_core.dll', 'Bibbia.Windows.pri', 'Assets/Bibbia.ico', 'martini-source.json', 'Uninstall.exe')) {
        if (-not (Test-Path (Join-Path $installDirectory $file))) { throw "File installato mancante: $file" }
    }
    if (-not (Test-Path $shortcut)) { throw 'Collegamento Start mancante.' }
    if ((Get-ItemProperty $registry).InstallDir -ne $installDirectory) { throw 'Percorso installazione nel registro errato.' }
    $entry = Get-ItemProperty $uninstallRegistry
    $installedKiB = [Math]::Floor((Get-ChildItem $installDirectory -Recurse -File | Measure-Object Length -Sum).Sum / 1024)
    if ($entry.EstimatedSize -le 0 -or [Math]::Abs($entry.EstimatedSize - $installedKiB) -gt 16) { throw 'Dimensione applicazione nelle Impostazioni non corretta.' }
    if ($entry.SystemComponent -ne 0 -or $entry.NoRemove -ne 0 -or $entry.AppUserModelID -ne 'io.github.MicheleSerafini.Bibbia') { throw 'Registrazione di disinstallazione Start/Impostazioni non corretta.' }
    $shell = New-Object -ComObject Shell.Application
    $link = $shell.Namespace((Split-Path $shortcut -Parent)).ParseName((Split-Path $shortcut -Leaf))
    if ($link.ExtendedProperty('System.AppUserModel.ID') -ne $entry.AppUserModelID) { throw 'Il collegamento Start non ha la stessa identità dell''app.' }
    $startUninstall = $link.ExtendedProperty('System.AppUserModel.UninstallCommand')
    if (-not $startUninstall) { $startUninstall = $link.ExtendedProperty('{9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3} 37') }
    if ($startUninstall -ne $entry.UninstallString) { throw 'Il menu contestuale Start non ha il comando di disinstallazione registrato.' }
    if (-not (Test-Path (Join-Path (Split-Path $shortcut -Parent) 'Disinstalla Bibbia.lnk'))) { throw 'Collegamento di disinstallazione Start mancante.' }
    $extraFile = Join-Path $installDirectory 'file-personale.txt'
    [IO.File]::WriteAllText($extraFile, 'Conservare anche questo file.')
    # An upgrade must replace only application files and leave user data intact.
    Invoke-Setup $installerPath "/S /D=$installDirectory"
    if (-not (Test-Path $extraFile) -or -not (Test-Path $noteFixture)) { throw 'L''aggiornamento ha eliminato dati personali.' }
    & "$env:WINDIR/System32/WindowsPowerShell/v1.0/powershell.exe" -NoProfile -File "$root/scripts/smoke-ui.ps1" -Executable "$installDirectory/Bibbia.Windows.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Verifica grafica WinUI fallita.' }
    dotnet run --project "$root/tests/Bibbia.UiState" -- "$installDirectory/bible_core.dll" $dataDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Le modifiche effettuate nella UI non sono state conservate.' }
    dotnet run --project "$root/tests/Bibbia.InteropSmoke" -- "$installDirectory/bible_core.dll"
    if ($LASTEXITCODE -ne 0) { throw 'Motore installato non funzionante.' }
    # Both install and uninstall must fail safely if the app is running.
    $marker = [Threading.Mutex]::new($false, 'Local\BibbiaRust.App')
    try {
        $blocked = Start-Process $installerPath -ArgumentList "/S /D=$installDirectory" -Wait -PassThru
        if ($blocked.ExitCode -ne 2) { throw 'Installer non bloccato durante l''esecuzione dell''app.' }
        $blocked = Start-Process "$installDirectory/Uninstall.exe" -ArgumentList "/S _?=$installDirectory" -Wait -PassThru
        if ($blocked.ExitCode -ne 2) { throw 'Disinstallazione non bloccata durante l''esecuzione dell''app.' }
    }
    finally { $marker.Dispose() }
    $registeredCommand = (Get-ItemProperty $uninstallRegistry).QuietUninstallString
    if ($registeredCommand -notmatch '^"([^\"]+)" /S$') { throw 'Comando di disinstallazione registrato non valido.' }
    Invoke-Setup $Matches[1] "/S _?=$installDirectory"
    if (Test-Path "$installDirectory/Bibbia.Windows.exe") { throw 'File applicazione rimasto dopo disinstallazione.' }
    if ((Test-Path $registry) -or (Test-Path $uninstallRegistry) -or (Test-Path $shortcut) -or (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\Bibbia.Windows.exe')) { throw 'Registrazione o collegamenti rimasti dopo disinstallazione.' }
    if (-not (Test-Path $extraFile) -or -not (Test-Path $noteFixture)) { throw 'La disinstallazione ha eliminato dati personali.' }
    Write-Host 'Installer: installazione, aggiornamento, collegamenti, blocco app aperta e conservazione dati verificati.'
}
finally {
    # Only these fixture directories were created by this test, after checking
    # that no user data or existing installation was present.
    # A native UI Automation crash can bypass the child script's finally.
    foreach ($fixtureProcess in @(Get-Process -Name 'Bibbia.Windows' -ErrorAction SilentlyContinue)) {
        if ($fixtureProcess.Path -eq (Join-Path $installDirectory 'Bibbia.Windows.exe')) {
            $fixtureProcess.Kill()
            $fixtureProcess.WaitForExit()
        }
    }
    if (Test-Path "$installDirectory/Uninstall.exe") {
        Start-Process "$installDirectory/Uninstall.exe" -ArgumentList "/S _?=$installDirectory" -Wait
    }
    if (Test-Path $installDirectory) { Remove-Item $installDirectory -Recurse -Force }
    if (Test-Path $dataDirectory) { Remove-Item $dataDirectory -Recurse -Force }
}
