# Run with Windows PowerShell 5.1 to use the built-in UI Automation assemblies.
param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DesktopProbe {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int left,top,right,bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window,out Rect rect);
    [StructLayout(LayoutKind.Sequential)] private struct Point { public int x,y; }
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    public static string CursorState() {
        Point point; GetCursorPos(out point);
        uint hit, foreground;
        GetWindowThreadProcessId(WindowFromPoint(point),out hit);
        GetWindowThreadProcessId(GetForegroundWindow(),out foreground);
        return string.Format("cursor={0},{1} hit pid={2} foreground pid={3}",point.x,point.y,hit,foreground);
    }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [StructLayout(LayoutKind.Sequential)] private struct MouseInput {
        public int x, y;
        public uint data, flags, time;
        public IntPtr extra;
    }
    [StructLayout(LayoutKind.Sequential)] private struct Input { public uint type; public MouseInput mouse; }
    [DllImport("user32.dll", SetLastError=true)] private static extern uint SendInput(uint count, Input[] input, int size);
    [DllImport("user32.dll")] private static extern int GetSystemMetrics(int metric);
    public static void Mouse(int x, int y, uint flags) {
        var input = new Input { type=0, mouse=new MouseInput {
            x=(int)((long)x*65535/(GetSystemMetrics(0)-1)),
            y=(int)((long)y*65535/(GetSystemMetrics(1)-1)), flags=0x8001 | flags }};
        if (SendInput(1, new[]{input}, Marshal.SizeOf(typeof(Input))) != 1)
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    }
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
    if ($id -eq 'BookPicker' -and $name -in @('1 Enoch (Charles 1917)', 'Jubilees (Charles 1917)')) {
        # The book list is virtualized. Reach its last two books with the real
        # keyboard instead of requiring unrealized items in the UIA tree.
        Add-Type -AssemblyName System.Windows.Forms
        [Windows.Forms.SendKeys]::SendWait('{END}')
        if ($name -eq '1 Enoch (Charles 1917)') { [Windows.Forms.SendKeys]::SendWait('{UP}') }
        [Windows.Forms.SendKeys]::SendWait('{ENTER}')
        Start-Sleep -Milliseconds 300
        $selected = $combo.GetCurrentPattern([Windows.Automation.SelectionPattern]::Pattern).Current.GetSelection()
        if ($selected.Count -ne 1 -or $selected[0].Current.Name -ne $name) { throw "Book selection failed: $name" }
        return
    }
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
    $combo.GetCurrentPattern([Windows.Automation.ExpandCollapsePattern]::Pattern).Collapse()
    Start-Sleep -Milliseconds 300
}
function Save-WindowImage([string]$name) {
    $diagnostics = Join-Path (Split-Path $PSScriptRoot -Parent) 'artifacts/diagnostics'
    New-Item $diagnostics -ItemType Directory -Force | Out-Null
    $bounds = [DesktopProbe+Rect]::new()
    [void][DesktopProbe]::GetWindowRect($process.MainWindowHandle,[ref]$bounds)
    $bitmap = [Drawing.Bitmap]::new($bounds.right-$bounds.left,$bounds.bottom-$bounds.top)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.left,$bounds.top,0,0,$bitmap.Size)
        $bitmap.Save((Join-Path $diagnostics $name))
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}
function Check-TitleTheme([int]$expected) {
    $actual = 0
    $result = [DesktopProbe]::DwmGetWindowAttribute($process.MainWindowHandle, 20, [ref]$actual, 4)
    if ($result -ne 0 -or $actual -ne $expected) { throw "Native title bar theme: expected $expected, got $actual (HRESULT $result)." }
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    do {
        $bounds = [DesktopProbe+Rect]::new()
        if (-not [DesktopProbe]::GetWindowRect($process.MainWindowHandle,[ref]$bounds)) { throw 'Window bounds unavailable.' }
        $bitmap = [Drawing.Bitmap]::new(1, 1)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen([int](($bounds.left + $bounds.right) / 2), [int]($bounds.top + 18), 0, 0, [Drawing.Size]::new(1, 1))
            $caption = $bitmap.GetPixel(0, 0)
            $brightness = $caption.GetBrightness()
            $graphics.CopyFromScreen([int]($bounds.right - 40), [int]($bounds.top + 80), 0, 0, [Drawing.Size]::new(1, 1))
            $surface = $bitmap.GetPixel(0, 0)
            $expectedChannel = if ($expected -eq 1) { 32 } else { 248 }
            $matches = $caption.R -eq $expectedChannel -and $caption.G -eq $expectedChannel -and $caption.B -eq $expectedChannel -and $caption.ToArgb() -eq $surface.ToArgb()
        }
        finally { $graphics.Dispose(); $bitmap.Dispose() }
        if ($matches -and (($expected -eq 1 -and $brightness -lt 0.5) -or ($expected -eq 0 -and $brightness -ge 0.5))) { return }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Caption appearance does not follow the theme: dark=$expected, brightness=$brightness caption=$caption surface=$surface"
}
$env:BIBBIA_UI_DIAGNOSTICS = '1'
$env:BIBBIA_UI_TEST_MOTION = '1'
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
    Write-Host 'WinUI: Catholic Italian chapter rendered.'
    $bookStage = Wait-Control $window 'BookStage'
    Invoke-Control (Wait-Control $window 'NextChapter')
    $turnDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        if ($bookStage.Current.HelpText -eq 'page-turn-active') { break }
        Start-Sleep -Milliseconds 15
    } while ([DateTime]::UtcNow -lt $turnDeadline)
    if ($bookStage.Current.HelpText -ne 'page-turn-active') { throw 'Native paper animation did not start.' }
    Start-Sleep -Milliseconds 110
    Save-WindowImage 'windows-page-turn.png'
    $turnDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ($bookStage.Current.HelpText -ne 'page-turn-idle' -and [DateTime]::UtcNow -lt $turnDeadline) { Start-Sleep -Milliseconds 30 }
    if ($bookStage.Current.HelpText -ne 'page-turn-idle') { throw 'Paper snapshot was not released.' }
    Invoke-Control (Wait-Control $window 'PreviousChapter')
    Start-Sleep -Milliseconds 700
    Wait-Text (Wait-Control $window 'VerseList') $firstVerse | Out-Null
    Write-Host 'WinUI: native curved paper turn and completion cleanup passed.'

    # Drag the mouse over part of the text, rather than invoking a whole-verse menu.
    $editor = Wait-Control $window 'VerseText_0'
    $originalTextHeight = $editor.Current.BoundingRectangle.Height
    $editor.SetFocus()
    Start-Sleep -Milliseconds 500
    # RichEdit's UIA text-range rectangles can be empty before its first render.
    # Drag within the real edit control instead; the database assertion below
    # the UI test verifies that a nonempty partial range was actually marked.
    $bounds = $editor.Current.BoundingRectangle
    if ($bounds.Width -lt 100 -or $bounds.Height -le 0) { throw 'Verse editor is not visible.' }
    $dragWidth = [Math]::Min($bounds.Width / 3, $bounds.Height * 3.5)
    $x = [int]($bounds.Left + 2); $y = [int]($bounds.Top + $bounds.Height / 2)
    Write-Host "Mouse drag: $x,$y to $([int]($x+$dragWidth)),$y; edit bounds $bounds"
    [DesktopProbe]::Mouse($x, $y, 2)
    for ($step = 1; $step -le 12; $step++) {
        [DesktopProbe]::Mouse([int]($x + $dragWidth * $step / 12), $y, 0)
        Start-Sleep -Milliseconds 25
    }
    [DesktopProbe]::Mouse([int]($x + $dragWidth), $y, 4)
    Write-Host "Expected app pid=$($process.Id); $([DesktopProbe]::CursorState())"
    # Avoid RichEdit's legacy UIA text-range provider on Server 2022: it can
    # return invalid native range handles. Check the result through the Rust
    # bridge after closing the UI instead (Bibbia.UiState).
    $underline = Wait-Control $window 'UnderlineSelection' -Popup
    if ($underline.Current.BoundingRectangle.Height -gt 100) { throw 'Selection toolbar is unexpectedly large.' }
    Invoke-Control $underline
    Write-Host 'WinUI: mouse selection opened its toolbar and underline was invoked.'

    Invoke-Control (Wait-Control $window 'VerseMenu_0')
    Invoke-Control (Wait-Control $window 'NoteVerse' -Popup)
    $note = Wait-Control $window 'NoteText' -Popup
    if ($note.Current.BoundingRectangle.Width -gt 380 -or $note.Current.BoundingRectangle.Height -gt 200) { throw 'Note editor is not a compact popup.' }
    $note.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('Nota UI: parole selezionate')
    Invoke-Control (Wait-Control $window 'SaveNote' -Popup)
    Invoke-Control (Wait-Control $window 'VerseMenu_0')
    Invoke-Control (Wait-Control $window 'BookmarkVerse' -Popup)
    Write-Host 'WinUI: compact note popup and bookmark actions invoked.'

    Select-Page 'SearchNavigation'
    $queryBox = Wait-Control $window 'SearchBox'
    $queryBox.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('talmente amato il mondo')
    Invoke-Control (Wait-Control $window 'SearchButton')
    Wait-Text (Wait-Control $window 'ResultList') 'Giovanni 3:16' | Out-Null
    Write-Host 'WinUI: Catholic Italian search verified.'

    Select-Page 'SettingsNavigation'
    Select-Option 'ThemePicker' 'Chiaro'
    Check-TitleTheme 0
    Select-Page 'ReadNavigation'
    Save-WindowImage 'windows-light.png'
    Select-Page 'SettingsNavigation'
    Select-Option 'ThemePicker' 'Scuro'
    Check-TitleTheme 1
    Select-Page 'ReadNavigation'
    Save-WindowImage 'windows-dark.png'
    Select-Page 'SettingsNavigation'
    $font = Wait-Control $window 'FontSlider'
    # RichEditBox has a 32px minimum height; use the maximum font size so
    # a real text resize exceeds that floor and changes its rendered bounds.
    $font.GetCurrentPattern([Windows.Automation.RangeValuePattern]::Pattern).SetValue(32)
    Start-Sleep -Milliseconds 300
    Select-Page 'ReadNavigation'
    $resized = Wait-Control $window 'VerseText_0'
    if ($resized.Current.BoundingRectangle.Height -le $originalTextHeight + 2) { throw "Text size did not update the rendered document: before=$originalTextHeight after=$($resized.Current.BoundingRectangle.Height)." }
    Select-Page 'SettingsNavigation'
    Select-Option 'AppLanguagePicker' 'English'
    Wait-Text $window 'Your reading experience' | Out-Null
    # Changing the app language must not silently change the Bible language.
    $selected = (Wait-Control $window 'EditionPicker').GetCurrentPattern([Windows.Automation.SelectionPattern]::Pattern).Current.GetSelection()
    if ($selected[0].Current.Name -ne 'Martini 1781 (cattolica)') { throw 'App language changed the Bible edition.' }
    Select-Option 'BibleLanguagePicker' 'English'
    $selected = (Wait-Control $window 'EditionPicker').GetCurrentPattern([Windows.Automation.SelectionPattern]::Pattern).Current.GetSelection()
    if ($selected[0].Current.Name -ne 'KJV') { throw 'English Bible filter did not select KJV.' }
    Select-Option 'BibleLanguagePicker' 'Italiano'
    Select-Option 'EditionPicker' 'CEI 2008'
    Select-Page 'ReadNavigation'
    Wait-Text (Wait-Control $window 'VerseList') ('In principio Dio cre' + [char]0xF2 + ' il cielo e la terra.') | Out-Null
    Invoke-Control (Wait-Control $window 'QuickEditionButton')
    Invoke-Control (Wait-Control $window 'QuickEdition_Raccoltaestesa(83sezioni)' -Popup)
    Select-Option 'BookPicker' '1 Enoch (Charles 1917)'
    $enoch = 'The words of the blessing of Enoch'
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty, 'VerseText_0')
    $first = (Wait-Control $window 'VerseList').FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $first -or -not $first.Current.Name.StartsWith($enoch)) { throw '1 Enoch did not render from the extended collection.' }
    Select-Option 'BookPicker' 'Jubilees (Charles 1917)'
    Wait-Control $window 'VerseText_0' | Out-Null
    Invoke-Control (Wait-Control $window 'QuickEditionButton')
    Invoke-Control (Wait-Control $window 'QuickEdition_Martini1781(cattolica)' -Popup)
    Select-Page 'ReadNavigation'
    Wait-Text (Wait-Control $window 'VerseList') $firstVerse | Out-Null
    Check-TitleTheme 1
    $maximize = Find-Control $window 'MaximizeWindowButton'
    if ($null -ne $maximize -and -not $maximize.Current.IsOffscreen) {
        $before = [DesktopProbe+Rect]::new()
        [void][DesktopProbe]::GetWindowRect($process.MainWindowHandle,[ref]$before)
        Invoke-Control $maximize
        $after = [DesktopProbe+Rect]::new()
        [void][DesktopProbe]::GetWindowRect($process.MainWindowHandle,[ref]$after)
        if ($after.right-$after.left -le $before.right-$before.left) { throw 'Title bar maximize button did not maximize the window.' }
        Invoke-Control (Wait-Control $window 'MaximizeWindowButton')
        [void][DesktopProbe]::GetWindowRect($process.MainWindowHandle,[ref]$after)
        if ([Math]::Abs(($after.right-$after.left)-($before.right-$before.left)) -gt 2) { throw 'Title bar restore button did not restore the window.' }
        Check-TitleTheme 1
    }
    Select-Page 'SavedNavigation'
    Wait-Text (Wait-Control $window 'ResultList') 'Genesi 1:1' | Out-Null
    if ((Wait-Control $window 'SavedPreviousPage').Current.IsEnabled) { throw 'First saved page has an enabled previous button.' }
    if ((Wait-Control $window 'SavedNextPage').Current.IsEnabled) { throw 'Single annotation unexpectedly has another page.' }
    Select-Page 'ReadNavigation'
    Wait-Text $window 'Chapter 1 of 50' | Out-Null
    $errorBarControl = Find-Control $window 'ErrorBar'
    if ($null -ne $errorBarControl -and -not $errorBarControl.Current.IsOffscreen) { throw "An application error was shown: $($errorBarControl.Current.Name)" }
    Write-Host 'WinUI: CEI, extended collection, animated navigation, text resizing, matching caption/surface colors, Catholic Italian text, mouse selection toolbar, note popup, bookmark, search, language filters and native title bar verified.'
}
catch {
    $diagnostics = Join-Path (Split-Path $PSScriptRoot -Parent) 'artifacts/diagnostics'
    New-Item $diagnostics -ItemType Directory -Force | Out-Null
    try {
        $errorLog = Join-Path $env:LOCALAPPDATA 'BibbiaRust/last-error.log'
        if (Test-Path $errorLog) { Get-Content $errorLog; Copy-Item $errorLog $diagnostics }
        $eventLog = Join-Path $env:LOCALAPPDATA 'BibbiaRust/ui-events.log'
        if (Test-Path $eventLog) { Get-Content $eventLog; Copy-Item $eventLog $diagnostics }
        $elements = [Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Descendants, [Windows.Automation.Condition]::TrueCondition)
        $snapshot = foreach ($element in $elements) {
            if ($element.Current.ProcessId -eq $process.Id) {
                "$($element.Current.ControlType.ProgrammaticName) | $($element.Current.AutomationId) | $($element.Current.Name)"
            }
        }
        $snapshot | Set-Content (Join-Path $diagnostics 'ui-tree.txt')
        $snapshot | Write-Host
        Add-Type -AssemblyName System.Windows.Forms
        Add-Type -AssemblyName System.Drawing
        $screen = [Windows.Forms.SystemInformation]::VirtualScreen
        $bitmap = [Drawing.Bitmap]::new($screen.Width, $screen.Height)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $graphics.CopyFromScreen($screen.Left, $screen.Top, 0, 0, $bitmap.Size)
        $bitmap.Save((Join-Path $diagnostics 'desktop.png'))
        $graphics.Dispose(); $bitmap.Dispose()
    }
    catch { Write-Host "Could not collect all UI diagnostics: $_" }
    throw
}
finally {
    if (-not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
    }
}
