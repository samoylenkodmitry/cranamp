param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$Folder,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$Schedule
)
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force $outputPath | Out-Null
$resultPath = Join-Path $outputPath 'result.json'
if (Test-Path $resultPath) { throw 'Use a new output directory for each run.' }

if ($Schedule) {
    $taskName = 'CranampFolderPlayback-' + [Guid]::NewGuid().ToString('N')
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}" -Executable "{1}" -Folder "{2}" -OutputDirectory "{3}"' -f $PSCommandPath, $Executable, $Folder, $outputPath
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2)
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings | Out-Null
    try {
        Start-ScheduledTask -TaskName $taskName
        $deadline = [DateTime]::UtcNow.AddSeconds(90)
        while (-not (Test-Path $resultPath) -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 500
        }
        if (-not (Test-Path $resultPath)) { throw 'The desktop playback test did not finish.' }
        $result = Get-Content $resultPath -Raw | ConvertFrom-Json
        $result | ConvertTo-Json
        if (-not $result.passed) { throw $result.error }
    } finally {
        Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
    }
    exit
}

$application = $null
$receipt = @{ passed = $false; error = '' }
try {
    if ([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) { throw 'Use -Schedule for audio in the desktop session.' }
    if (-not [IO.Directory]::Exists($Folder)) { throw 'The original folder is not accessible.' }
    $stdout = Join-Path $outputPath 'stdout.log'
    $stderr = Join-Path $outputPath 'stderr.log'
    $application = Start-Process -FilePath ([IO.Path]::GetFullPath($Executable)) -ArgumentList ('--folder "{0}"' -f $Folder) -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    # Windows PowerShell must retain the process handle to read its exit code.
    $null = $application.Handle
    if (-not $application.WaitForExit(60000)) { throw 'Playback did not start within 60 seconds.' }
    $log = Get-Content $stdout -Raw
    Write-Output $log
    $receipt.exitCode = $application.ExitCode
    if ($application.ExitCode -ne 0 -or $log -notmatch 'PLAYBACK PROBE: passed') { throw "The playback probe failed (exit $($application.ExitCode)); see stdout.log and stderr.log." }
    $receipt.passed = $true
} catch {
    $receipt.error = $_.Exception.Message
} finally {
    if ($null -ne $application -and -not $application.HasExited) { Stop-Process -Id $application.Id }
    $receipt | ConvertTo-Json | Set-Content $resultPath
}
if (-not $receipt.passed) { throw $receipt.error }
