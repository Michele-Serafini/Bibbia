# Run with Windows PowerShell 5.1 to use the built-in UI Automation assemblies.
param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DesktopProbe {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out int value, int size);
}
'@
function Find-Control($parent, [string]$id) {
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty, $id)
    return $parent.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
}
function Wait-Control($parent, [string]$id, [switch]$Popup) {
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        $control = Find-Control $parent $id
        if ($null -eq $control -and $Popup) { $control = Find-Control ([Windows.Automation.AutomationElement]::RootElement) $id }
        if ($null -ne $control) { return $control }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "UI control not available: $id"
}
function Wait-Text($parent, [string]$text) {
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, $text)
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        $control = $parent.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
        if ($null -ne $control) { return $control }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Expected text not shown: $text"
}
function Invoke-Control($control) {
    $control.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 200
}
function Select-Page([string]$id) {
    (Wait-Control $window $id).GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern).Select()
    Start-Sleep -Milliseconds 200
}
function Select-Option([string]$id, [string]$name) {
    $combo = Wait-Control $window $id
    $combo.SetFocus()
    $combo.GetCurrentPattern([Windows.Automation.ExpandCollapsePattern]::Pattern).Expand()
    $condition = [Windows.Automation.AndCondition]::new(
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, $name),
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty, [Windows.Automation.ControlType]::ListItem))
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        $item = [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
        if ($null -ne $item) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($null -eq $item) { throw "Combo option not found: $id / $name" }
    $item.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern).Select()
    Start-Sleep -Milliseconds 300
}
function Check-TitleTheme([int]$expected) {
    $actual = 0
    $result = [DesktopProbe]::DwmGetWindowAttribute($process.MainWindowHandle, 20, [ref]$actual, 4)
    if ($result -ne 0 -or $actual -ne $expected) { throw "Native title bar theme: expected $expected, got $actual (HRESULT $result)." }
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
    [void][DesktopProbe]::SetForegroundWindow($process.MainWindowHandle)
    $window = [Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $verses = Wait-Control $window 'VerseList'
    $firstVerse = 'Al principio cre' + [char]0xF2 + ' Dio il cielo e la terra.'
    Wait-Text $verses $firstVerse | Out-Null

    # Drag the mouse over part of the text, rather than invoking a whole-verse menu.
    $editor = Wait-Control $window 'VerseText_0'
    $editor.SetFocus()
    $range = $editor.GetCurrentPattern([Windows.Automation.TextPattern]::Pattern).DocumentRange.Clone()
    $range.MoveEndpointByRange([Windows.Automation.TextPatternRangeEndpoint]::End, $range, [Windows.Automation.TextPatternRangeEndpoint]::Start)
    [void]$range.MoveEndpointByUnit([Windows.Automation.TextPatternRangeEndpoint]::End, [Windows.Automation.TextUnit]::Character, 12)
    $bounds = $range.GetBoundingRectangles()
    if ($bounds.Length -lt 4 -or $bounds[2] -le 10) { throw 'Selected text has no visible bounding rectangle.' }
    $x = [int]($bounds[0] + 1); $y = [int]($bounds[1] + $bounds[3] / 2)
    [void][DesktopProbe]::SetCursorPos($x, $y)
    [DesktopProbe]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    for ($step = 1; $step -le 12; $step++) {
        [void][DesktopProbe]::SetCursorPos([int]($x + ($bounds[2] - 2) * $step / 12), $y)
        Start-Sleep -Milliseconds 25
    }
    [DesktopProbe]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    $underline = Wait-Control $window 'UnderlineSelection' -Popup
    if ($underline.Current.BoundingRectangle.Height -gt 100) { throw 'Selection toolbar is unexpectedly large.' }
    Invoke-Control $underline

    Invoke-Control (Wait-Control $window 'VerseMenu_0')
    Invoke-Control (Wait-Control $window 'NoteVerse' -Popup)
    $note = Wait-Control $window 'NoteText' -Popup
    if ($note.Current.BoundingRectangle.Width -gt 380 -or $note.Current.BoundingRectangle.Height -gt 200) { throw 'Note editor is not a compact popup.' }
    $note.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('Nota UI: parole selezionate')
    Invoke-Control (Wait-Control $window 'SaveNote' -Popup)
    Invoke-Control (Wait-Control $window 'VerseMenu_0')
    Invoke-Control (Wait-Control $window 'BookmarkVerse' -Popup)

    Select-Page 'SearchNavigation'
    $queryBox = Wait-Control $window 'SearchBox'
    $queryBox.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('talmente amato il mondo')
    Invoke-Control (Wait-Control $window 'SearchButton')
    Wait-Text (Wait-Control $window 'ResultList') 'Giovanni 3:16' | Out-Null

    Select-Page 'SettingsNavigation'
    Select-Option 'ThemePicker' 'Chiaro'
    Check-TitleTheme 0
    Select-Option 'ThemePicker' 'Scuro'
    Check-TitleTheme 1
    Select-Option 'AppLanguagePicker' 'English'
    Wait-Text $window 'Your reading experience' | Out-Null
    # Changing the app language must not silently change the Bible language.
    $selected = (Wait-Control $window 'EditionPicker').GetCurrentPattern([Windows.Automation.SelectionPattern]::Pattern).GetSelection()
    if ($selected[0].Current.Name -ne 'Martini 1781 (cattolica)') { throw 'App language changed the Bible edition.' }
    Select-Option 'BibleLanguagePicker' 'English'
    $selected = (Wait-Control $window 'EditionPicker').GetCurrentPattern([Windows.Automation.SelectionPattern]::Pattern).GetSelection()
    if ($selected[0].Current.Name -ne 'KJV') { throw 'English Bible filter did not select KJV.' }
    Select-Option 'BibleLanguagePicker' 'Italiano'
    Select-Page 'ReadNavigation'
    Wait-Text (Wait-Control $window 'VerseList') $firstVerse | Out-Null
    Check-TitleTheme 1
    Write-Host 'WinUI: Catholic Italian text, mouse selection toolbar, note popup, bookmark, search, language filters and native title bar verified.'
}
finally {
    if (-not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
    }
}
