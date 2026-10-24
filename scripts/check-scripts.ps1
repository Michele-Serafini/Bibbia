$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$files = @(Get-ChildItem "$root/scripts/*.ps1") + @(Get-ChildItem "$root/tests/*.ps1")
$failed = $false
foreach ($file in $files) {
    $tokens = $null
    $problems = $null
    [Management.Automation.Language.Parser]::ParseFile($file.FullName, [ref]$tokens, [ref]$problems) | Out-Null
    foreach ($problem in $problems) {
        Write-Host "$($file.Name):$($problem.Extent.StartLineNumber): $($problem.Message)"
        $failed = $true
    }
}
if ($failed) { throw 'Correggere gli errori PowerShell prima della build.' }
Write-Host 'Sintassi PowerShell verificata.'
