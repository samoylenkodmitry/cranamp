param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$Schedule
)
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
if ($Schedule) {
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}" -Executable "{1}" -OutputDirectory "{2}"' -f $PSCommandPath, $Executable, $outputPath
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2)
    Register-ScheduledTask -TaskName 'CranampRenderingAudit' -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName 'CranampRenderingAudit'
    exit
}
Start-Transcript -Path (Join-Path $outputPath 'audit.log') -Force
try {
    if ([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) { throw 'Use -Schedule for the desktop session.' }
    $env:RUST_LOG = 'info'
    $env:RUST_BACKTRACE = '1'
    $application = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $outputPath 'stdout.log') -RedirectStandardError (Join-Path $outputPath 'stderr.log')
    Start-Sleep -Seconds 12
    $application.Refresh()
    if ($application.HasExited) { throw ('Application exited: {0}' -f $application.ExitCode) }
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    Add-Type -AssemblyName System.Drawing
    $condition = [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty, $application.Id)
    $windows = [Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Children, $condition)
    $window = $windows | Sort-Object { $_.Current.BoundingRectangle.Width * $_.Current.BoundingRectangle.Height } -Descending | Select-Object -First 1
    if ($null -eq $window) { throw 'No application window.' }
    $bounds = $window.Current.BoundingRectangle
    $windows | ForEach-Object { [pscustomobject]@{ Name = $_.Current.Name; Bounds = $_.Current.BoundingRectangle.ToString() } } | ConvertTo-Json | Set-Content (Join-Path $outputPath 'windows.json')
    $bitmap = [Drawing.Bitmap]::new([int]$bounds.Width, [int]$bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.X, [int]$bounds.Y, 0, 0, $bitmap.Size)
    $bitmap.Save((Join-Path $outputPath 'window.png'))
    $colors = [Collections.Generic.HashSet[int]]::new()
    for ($y = 0; $y -lt $bitmap.Height; $y += 4) {
        for ($x = 0; $x -lt $bitmap.Width; $x += 4) {
            $null = $colors.Add($bitmap.GetPixel($x, $y).ToArgb())
        }
    }
    $graphics.Dispose()
    $bitmap.Dispose()
    $application | Select-Object Id,SessionId,Responding,MainWindowTitle | ConvertTo-Json | Set-Content (Join-Path $outputPath 'process.json')
    Write-Output ('Captured {0} colors at {1}' -f $colors.Count, $bounds)
    if ($bounds.Width -lt 275 -or $bounds.Height -lt 493) { throw 'The full player window was not visible.' }
    if ($colors.Count -lt 16) { throw 'The player did not render: fewer than 16 sampled colors.' }
} catch {
    $_ | Out-String | Set-Content (Join-Path $outputPath 'failure.txt')
    throw
} finally {
    if ($application -and -not $application.HasExited) { Stop-Process -Id $application.Id }
    Stop-Transcript
}
