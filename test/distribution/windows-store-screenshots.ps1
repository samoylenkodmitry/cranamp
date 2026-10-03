param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force $outputPath | Out-Null
function Invoke-Studio([string]$Name, [hashtable]$Arguments = @{}) {
    $body = @{jsonrpc='2.0'; id=1; method='tools/call'; params=@{name=$Name; arguments=$Arguments}} | ConvertTo-Json -Depth 8
    $reply = Invoke-RestMethod -Uri 'http://127.0.0.1:18765/mcp' -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 60
    if ($reply.error -or $reply.result.isError) { throw ($reply | ConvertTo-Json -Depth 8) }
    return $reply
}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
function Invoke-StudioButton([string]$Name) {
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        $application.Refresh()
        if ($application.HasExited) { throw "Studio exited with code $($application.ExitCode)" }
        if ($application.MainWindowHandle -ne [IntPtr]::Zero) {
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($application.MainWindowHandle)
            $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $Name)
            $button = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
            if ($null -ne $button -and $button.Current.IsEnabled) {
                $invoke = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
                ([System.Windows.Automation.InvokePattern]$invoke).Invoke()
                return
            }
        }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Studio button was not available: $Name"
}
$application = Start-Process -FilePath ([IO.Path]::GetFullPath($Executable)) -ArgumentList '--skin-studio' -PassThru -RedirectStandardOutput (Join-Path $outputPath 'stdout.log') -RedirectStandardError (Join-Path $outputPath 'stderr.log')
try {
    Invoke-StudioButton 'Agent connection'
    Invoke-StudioButton 'Start server'
    $ready = $false
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        $application.Refresh()
        if ($application.HasExited) { throw "Studio exited with code $($application.ExitCode)" }
        try { $null = Invoke-Studio 'studio_status'; $ready = $true; break }
        catch { Start-Sleep -Seconds 2 }
    }
    if (-not $ready) { throw 'Studio MCP did not start within 60 seconds' }
    $null = Invoke-Studio 'studio_canvas' @{drawer='none'; presentation=$false; zoom=2}
    $null = Invoke-Studio 'studio_screenshot' @{path=(Join-Path $outputPath 'windows-studio.png')}
    $null = Invoke-Studio 'studio_screenshot' @{presentation=$true; path=(Join-Path $outputPath 'windows-player.png')}
    Add-Type -AssemblyName System.Drawing
    foreach ($file in Get-ChildItem $outputPath -Filter '*.png') {
        $image = [Drawing.Bitmap]::FromFile($file.FullName)
        try {
            $colors = [Collections.Generic.HashSet[int]]::new()
            for ($y = 0; $y -lt $image.Height; $y += 4) {
                for ($x = 0; $x -lt $image.Width; $x += 4) { $null = $colors.Add($image.GetPixel($x, $y).ToArgb()) }
            }
            if ($image.Width -lt 1280 -or $image.Height -lt 720 -or $colors.Count -lt 16) { throw "Blank or undersized capture: $($file.Name)" }
            Write-Output "$($file.Name): $($image.Width)x$($image.Height), $($colors.Count) sampled colors"
        } finally { $image.Dispose() }
    }
} finally {
    if (-not $application.HasExited) { Stop-Process -Id $application.Id }
}
