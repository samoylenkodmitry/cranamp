param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$Schedule,
    [switch]$ConsoleOnly
)
$ErrorActionPreference = 'Stop'
$taskName = 'CranampShellAudit'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$result = Join-Path $outputPath 'result.txt'
Remove-Item $result, (Join-Path $outputPath 'failure.txt') -ErrorAction SilentlyContinue
if ($Schedule) {
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}" -Executable "{1}" -OutputDirectory "{2}"' -f $PSCommandPath, $Executable, $outputPath
    if ($ConsoleOnly) { $arguments += ' -ConsoleOnly' }
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2)
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    exit
}
Start-Transcript -Path (Join-Path $outputPath 'audit.log') -Force
$application = $null
try {
    if ([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) { throw 'Use -Schedule for the desktop session.' }
    Add-Type -AssemblyName System.Drawing
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class CranampShell {
    delegate bool EnumProc(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtr(IntPtr window, int index);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr window, System.Text.StringBuilder name, int capacity);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr window, System.Text.StringBuilder text, int capacity);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    public struct Rect { public int Left, Top, Right, Bottom; }
    public static string Describe(IntPtr window) {
        var name = new System.Text.StringBuilder(256);
        var text = new System.Text.StringBuilder(256);
        GetClassName(window, name, name.Capacity);
        GetWindowText(window, text, text.Capacity);
        Rect rect;
        GetWindowRect(window, out rect);
        return string.Format("class '{0}' title '{1}' {2}x{3} at {4},{5}", name, text, rect.Right - rect.Left, rect.Bottom - rect.Top, rect.Left, rect.Top);
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] public static extern uint ExtractIconEx(string file, int index, IntPtr[] large, IntPtr[] small, uint count);
    static List<IntPtr> ConsoleWindows() {
        var found = new List<IntPtr>();
        EnumWindows((window, data) => {
            var name = new System.Text.StringBuilder(256);
            GetClassName(window, name, name.Capacity);
            var kind = name.ToString();
            if ((kind == "ConsoleWindowClass" || kind == "CASCADIA_HOSTING_WINDOW_CLASS") && IsWindowVisible(window)) { found.Add(window); }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static HashSet<IntPtr> ConsoleWindowsNow() {
        return new HashSet<IntPtr>(ConsoleWindows());
    }
    public static HashSet<int> ProcessesNow() {
        var ids = new HashSet<int>();
        foreach (var process in System.Diagnostics.Process.GetProcesses()) { ids.Add(process.Id); }
        return ids;
    }
    public static List<string> Started = new List<string>();
    public static List<string> ConsoleWindowsOpenedWithin(HashSet<IntPtr> before, HashSet<int> running, int milliseconds) {
        var opened = new List<string>();
        var clock = System.Diagnostics.Stopwatch.StartNew();
        while (clock.ElapsedMilliseconds < milliseconds) {
            foreach (var window in ConsoleWindows()) {
                if (before.Add(window)) { opened.Add(string.Format("{0} ms: {1}", clock.ElapsedMilliseconds, Describe(window))); }
            }
            foreach (var process in System.Diagnostics.Process.GetProcesses()) {
                if (running.Add(process.Id)) { Started.Add(string.Format("{0} ms: {1} (pid {2})", clock.ElapsedMilliseconds, process.ProcessName, process.Id)); }
            }
            System.Threading.Thread.Sleep(10);
        }
        return opened;
    }
    public static List<IntPtr> TaskbarWindows(uint process) {
        var found = new List<IntPtr>();
        EnumWindows((window, data) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            bool tool = ((long)GetWindowLongPtr(window, -20) & 0x80) != 0;
            if (owner == process && IsWindowVisible(window) && !tool) { found.Add(window); }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@

    $bytes = [IO.File]::ReadAllBytes($Executable)
    $subsystem = [BitConverter]::ToUInt16($bytes, [BitConverter]::ToInt32($bytes, 0x3C) + 0x5C)
    Write-Output ('PE subsystem: {0}' -f $subsystem)
    if ($subsystem -ne 2) { throw ('The executable is a console program (subsystem {0}), so Windows opens a terminal beside it.' -f $subsystem) }

    if (-not $ConsoleOnly) {
        $description = (Get-Item $Executable).VersionInfo.FileDescription
        Write-Output ('File description: {0}' -f $description)
        if ($description -ne 'Cranamp') { throw 'The executable carries no Cranamp version resource.' }

        $icons = [CranampShell]::ExtractIconEx($Executable, -1, $null, $null, 0)
        Write-Output ('Icons in the executable: {0}' -f $icons)
        if ($icons -lt 1) { throw 'The executable carries no icon for Explorer and shortcuts.' }
        [Drawing.Icon]::ExtractAssociatedIcon($Executable).ToBitmap().Save((Join-Path $outputPath 'exe-icon.png'))
    }

    $consoles = [CranampShell]::ConsoleWindowsNow()
    $running = [CranampShell]::ProcessesNow()
    $application = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru
    $opened = [CranampShell]::ConsoleWindowsOpenedWithin($consoles, $running, 10000)
    [CranampShell]::Started | ForEach-Object { Write-Output ('Process started at {0}' -f $_) }
    $opened | ForEach-Object { Write-Output ('Console window opened at {0}' -f $_) }
    $application.Refresh()
    if ($application.HasExited) { throw ('Application exited: {0}' -f $application.ExitCode) }
    if ($opened.Count -gt 0) { throw ('{0} console window(s) opened while the application started.' -f $opened.Count) }

    $windows = [CranampShell]::TaskbarWindows([uint32]$application.Id)
    Write-Output ('Visible windows: {0}' -f $windows.Count)
    if ($windows.Count -lt 1) { throw 'The application opened no visible window.' }
    foreach ($window in $windows) {
        $taskbar = [CranampShell]::SendMessage($window, 0x7F, [IntPtr]1, [IntPtr]::Zero)
        $titleBar = [CranampShell]::SendMessage($window, 0x7F, [IntPtr]0, [IntPtr]::Zero)
        Write-Output ('Window {0} ({1}): taskbar icon {2}, title-bar icon {3}' -f $window, [CranampShell]::Describe($window), $taskbar, $titleBar)
        if (-not $ConsoleOnly -and ($taskbar -eq [IntPtr]::Zero -or $titleBar -eq [IntPtr]::Zero)) { throw ('Window {0} has no icon of its own.' -f $window) }
    }
    if (-not $ConsoleOnly) {
        $taskbar = [CranampShell]::SendMessage($windows[0], 0x7F, [IntPtr]1, [IntPtr]::Zero)
        $icon = [Drawing.Icon]::FromHandle($taskbar).ToBitmap()
        $icon.Save((Join-Path $outputPath 'window-icon.png'))
        $plate = $icon.GetPixel([int]($icon.Width * 0.06), [int]($icon.Height * 0.16))
        Write-Output ('Taskbar icon plate: R {0} G {1} B {2}' -f $plate.R, $plate.G, $plate.B)
        if ($plate.B -le $plate.R + 20) { throw 'The taskbar icon has its red and blue swapped; the teal plate came out brown.' }
    }
    'PASS' | Set-Content $result
} catch {
    $_ | Out-String | Set-Content (Join-Path $outputPath 'failure.txt')
    'FAIL' | Set-Content $result
    throw
} finally {
    if ($application -and -not $application.HasExited) { Stop-Process -Id $application.Id }
    Stop-Transcript
}
