param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [int]$Steps = 30,
    [int]$StepPixels = 4,
    [int]$StepPixelsAcross = 0,
    [switch]$TearOff,
    [switch]$Schedule
)
# Drags the playlist's corner down with real mouse input and captures the
# player after every step, so a player that shakes while its window grows
# shows it frame by frame. test/windows_resize_jitter.py compares the frames.
# -TearOff first pulls the playlist out of the stack by its title bar, so the
# corner resizes the playlist's own window, which the system resizes.
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
if ($Schedule) {
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}" -Executable "{1}" -OutputDirectory "{2}" -Steps {3} -StepPixels {4} -StepPixelsAcross {5}' -f $PSCommandPath, $Executable, $outputPath, $Steps, $StepPixels, $StepPixelsAcross
    if ($TearOff) { $arguments += ' -TearOff' }
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 3)
    Register-ScheduledTask -TaskName 'CranampResizeJitter' -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName 'CranampResizeJitter'
    exit
}
Start-Transcript -Path (Join-Path $outputPath 'jitter.log') -Force
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class JitterInput {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    public const uint LeftDown = 0x0002;
    public const uint LeftUp = 0x0004;
}
'@
[JitterInput]::SetProcessDPIAware() | Out-Null
function Find-Window([int]$processId, [string]$title) {
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty, $processId)
    $found = [Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Children, $condition)
    $found | Where-Object { $_.Current.Name -eq $title } | Select-Object -First 1
}
function Drag-Mouse([int]$fromX, [int]$fromY, [int]$toX, [int]$toY) {
    [JitterInput]::SetCursorPos($fromX, $fromY) | Out-Null
    Start-Sleep -Milliseconds 200
    [JitterInput]::mouse_event([JitterInput]::LeftDown, 0, 0, 0, [UIntPtr]::Zero)
    for ($i = 1; $i -le 20; $i++) {
        [JitterInput]::SetCursorPos($fromX + ($toX - $fromX) * $i / 20, $fromY + ($toY - $fromY) * $i / 20) | Out-Null
        Start-Sleep -Milliseconds 25
    }
    [JitterInput]::mouse_event([JitterInput]::LeftUp, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 800
}
# Captures stay in memory until the drag ends: writing each one out would
# leave most of the frames between captures unseen.
$captures = [Collections.Generic.List[object]]::new()
function Save-Capture([Windows.Rect]$bounds, [int]$extraHeight, [string]$name) {
    $bitmap = [Drawing.Bitmap]::new([int]$bounds.Width + $extraWidth, [int]$bounds.Height + $extraHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.X, [int]$bounds.Y, 0, 0, $bitmap.Size)
    $graphics.Dispose()
    $captures.Add([pscustomobject]@{ Name = $name; Bitmap = $bitmap })
}
function Write-Captures {
    foreach ($capture in $captures) {
        $capture.Bitmap.Save((Join-Path $outputPath $capture.Name))
        $capture.Bitmap.Dispose()
    }
}
try {
    if ([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) { throw 'Use -Schedule for the desktop session.' }
    Get-Process cranamp -ErrorAction SilentlyContinue | Stop-Process -Force
    $application = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $outputPath 'stdout.log') -RedirectStandardError (Join-Path $outputPath 'stderr.log')
    Start-Sleep -Seconds 10
    $window = Find-Window $application.Id 'Cranamp Winamp'
    if ($null -eq $window) { throw 'No player window.' }
    if ($TearOff) {
        # The playlist's title bar sits under the main window and the
        # equalizer; pull it clear of the stack to the right.
        $stack = $window.Current.BoundingRectangle
        Drag-Mouse ([int]$stack.X + 100) ([int]$stack.Y + 238) ([int]$stack.X + $stack.Width + 120) ([int]$stack.Y + 238)
        $window = Find-Window $application.Id 'Cranamp Winamp Playlist'
        if ($null -eq $window) { throw 'The playlist did not come out of the stack.' }
    }
    $bounds = $window.Current.BoundingRectangle
    $extra = $Steps * $StepPixels + 8
    $extraWidth = $Steps * $StepPixelsAcross + 8
    Save-Capture $bounds $extra 'frame-000.png'
    $x = [int]($bounds.X + $bounds.Width - 5)
    $y = [int]($bounds.Y + $bounds.Height - 5)
    [JitterInput]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 300
    [JitterInput]::mouse_event([JitterInput]::LeftDown, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 100
    for ($step = 1; $step -le $Steps; $step++) {
        [JitterInput]::SetCursorPos($x + $step * $StepPixelsAcross, $y + $step * $StepPixels) | Out-Null
        # Three looks per step: right away, and over the next two frames.
        foreach ($look in 'a', 'b', 'c') {
            Save-Capture $bounds $extra ('frame-{0:D3}{1}.png' -f $step, $look)
            Start-Sleep -Milliseconds 5
        }
    }
    [JitterInput]::mouse_event([JitterInput]::LeftUp, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 500
    Save-Capture $bounds $extra 'frame-end.png'
    Write-Captures
    $after = $window.Current.BoundingRectangle
    [pscustomobject]@{ Before = $bounds.ToString(); After = $after.ToString(); Steps = $Steps; StepPixels = $StepPixels; StepPixelsAcross = $StepPixelsAcross } | ConvertTo-Json | Set-Content (Join-Path $outputPath 'bounds.json')
    Write-Output ('Window {0} -> {1}' -f $bounds, $after)
} catch {
    $_ | Out-String | Set-Content (Join-Path $outputPath 'failure.txt')
    throw
} finally {
    if ($application -and -not $application.HasExited) { Stop-Process -Id $application.Id }
    Stop-Transcript
}
