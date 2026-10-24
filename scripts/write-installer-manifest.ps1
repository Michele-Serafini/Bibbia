param(
    [Parameter(Mandatory)][string]$Payload,
    [Parameter(Mandatory)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$payloadRoot = (Resolve-Path $Payload).Path
if ((Get-Item $payloadRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Il payload non può essere un collegamento.' }
$entries = @(Get-ChildItem $payloadRoot -Recurse -Force)
foreach ($entry in $entries) {
    if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Collegamento non ammesso nel payload: $($entry.Name)" }
}
function Quote-Nsis([string]$value) {
    if ($value -match '[\r\n]') { throw 'Nome file non ammesso.' }
    return $value.Replace('$', '$$').Replace('"', '$\"')
}
$install = [Collections.Generic.List[string]]::new()
$uninstall = [Collections.Generic.List[string]]::new()
$directories = [Collections.Generic.HashSet[string]]::new()
foreach ($file in @($entries | Where-Object { -not $_.PSIsContainer } | Sort-Object FullName)) {
    $relative = [IO.Path]::GetRelativePath($payloadRoot, $file.FullName).Replace('/', '\')
    if ($relative -ieq 'Uninstall.exe') { throw 'Uninstall.exe è un nome riservato.' }
    # GetDirectoryName follows the host separator. Normalize for Linux syntax checks too.
    $parent = ($relative -split '\\' | Select-Object -SkipLast 1) -join '\'
    $destination = if ($parent) { '\' + (Quote-Nsis $parent) } else { '' }
    $install.Add('SetOutPath "$INSTDIR' + $destination + '"')
    # File reads its source at compile time: $$ is not unescaped there. /oname
    # is a runtime string and does need escaping, as does SetOutPath above.
    if ($file.FullName -match '["\r\n]' -or $file.FullName.Contains('${')) { throw 'Percorso sorgente NSIS non ammesso.' }
    $install.Add('File "/oname=' + (Quote-Nsis $file.Name) + '" "' + $file.FullName + '"')
    $uninstall.Add('Delete "$INSTDIR\' + (Quote-Nsis $relative) + '"')
    while ($parent) {
        [void]$directories.Add($parent)
        $parent = ($parent -split '\\' | Select-Object -SkipLast 1) -join '\'
    }
}
if ($install.Count -eq 0) { throw 'Payload vuoto.' }
foreach ($dir in @($directories | Sort-Object { $_.Length } -Descending)) {
    $uninstall.Add('RMDir "$INSTDIR\' + (Quote-Nsis $dir) + '"')
}
New-Item $OutputDirectory -ItemType Directory -Force | Out-Null
[IO.File]::WriteAllLines((Join-Path $OutputDirectory 'install-files.nsh'), $install, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllLines((Join-Path $OutputDirectory 'uninstall-files.nsh'), $uninstall, [Text.UTF8Encoding]::new($false))
