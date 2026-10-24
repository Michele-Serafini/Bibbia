# Run with Windows PowerShell 5.1 to use the built-in UI Automation assemblies.
param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
function Find-Control($parent, [string]$id) {
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty, $id)
    return $parent.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
}
function Wait-Control($parent, [string]$id) {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $control = Find-Control $parent $id
        if ($null -ne $control) { return $control }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "UI control not available: $id"
}
function Wait-Text($parent, [string]$text) {
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, $text)
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $control = $parent.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
        if ($null -ne $control) { return $control }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Expected text not shown: $text"
}
$process = Start-Process (Resolve-Path $Executable).Path -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $process.Refresh()
        if ($process.HasExited) { throw "Application exited at startup: $($process.ExitCode)" }
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Application window did not open.' }
    $window = [Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $verses = Wait-Control $window 'VerseList'
    $firstVerse = 'NEL principio Iddio cre' + [char]0xF2 + ' il cielo e la terra.'
    Wait-Text $verses $firstVerse | Out-Null
    $searchNavigation = Wait-Control $window 'SearchNavigation'
    $selection = $searchNavigation.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern)
    $selection.Select()
    $queryBox = Wait-Control $window 'SearchBox'
    $value = $queryBox.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern)
    $value.SetValue('Iddio ha tanto amato il mondo')
    $button = Wait-Control $window 'SearchButton'
    $invoke = $button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern)
    $invoke.Invoke()
    $results = Wait-Control $window 'ResultList'
    Wait-Text $results 'Giovanni 3:16' | Out-Null
    Write-Host 'WinUI: startup, rendered Italian chapter and search through real controls verified.'
}
finally {
    if (-not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
    }
}
