param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$Schedule
)
# Closes Cranamp's main window with Alt+W and opens it again, as Winamp's
# keyboard does, and records each process window's title, bounds and
# visibility, and the screen, before and after each press. With the main
# window closed the equalizer and the playlist stay where they were, in
# windows of their own.
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
if ($Schedule) {
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}" -Executable "{1}" -OutputDirectory "{2}"' -f $PSCommandPath, $Executable, $outputPath
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 3)
    Register-ScheduledTask -TaskName 'CranampMainWindow' -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName 'CranampMainWindow'
    exit
}
Start-Transcript -Path (Join-Path $outputPath 'main-window.log') -Force
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class MainWindowProbe {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc proc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public static List<string> Windows(uint pid) {
        var found = new List<string>();
        EnumWindows((hwnd, _) => {
            uint owner; GetWindowThreadProcessId(hwnd, out owner);
            if (owner != pid) return true;
            var title = new StringBuilder(256); GetWindowText(hwnd, title, 256);
            if (title.Length == 0) return true;
            RECT r; GetWindowRect(hwnd, out r);
            found.Add(string.Format("{0}|{1},{2},{3},{4}|{5}|{6}", title, r.Left, r.Top, r.Right - r.Left, r.Bottom - r.Top, IsWindowVisible(hwnd) ? "visible" : "hidden", hwnd));
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static void AltW() {
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x57, 0, 0, UIntPtr.Zero);
        keybd_event(0x57, 0, 2, UIntPtr.Zero);
        keybd_event(0x12, 0, 2, UIntPtr.Zero);
    }
}
'@
[MainWindowProbe]::SetProcessDPIAware() | Out-Null
function Save-State([string]$name) {
    [MainWindowProbe]::Windows($application.Id) | Set-Content (Join-Path $outputPath "$name.txt")
    $screen = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = [Drawing.Bitmap]::new($screen.Width, $screen.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($screen.X, $screen.Y, 0, 0, $bitmap.Size)
    $bitmap.Save((Join-Path $outputPath "$name.png"))
    $graphics.Dispose()
    $bitmap.Dispose()
}
function Press-AltW {
    $visible = [MainWindowProbe]::Windows($application.Id) | Where-Object { $_ -like '*|visible|*' } | Select-Object -First 1
    [MainWindowProbe]::SetForegroundWindow([IntPtr][long]($visible -split '\|')[3]) | Out-Null
    Start-Sleep -Milliseconds 400
    [MainWindowProbe]::AltW()
    Start-Sleep -Seconds 2
}
try {
    if ([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) { throw 'Use -Schedule for the desktop session.' }
    Get-Process cranamp -ErrorAction SilentlyContinue | Stop-Process -Force
    $application = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru
    Start-Sleep -Seconds 10
    Save-State 'open'
    Press-AltW
    Save-State 'closed'
    Press-AltW
    Save-State 'reopened'
} catch {
    $_ | Out-String | Set-Content (Join-Path $outputPath 'failure.txt')
    throw
} finally {
    if ($application -and -not $application.HasExited) { Stop-Process -Id $application.Id }
    Stop-Transcript
}
