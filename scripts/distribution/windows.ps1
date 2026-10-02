$ErrorActionPreference = 'Stop'
Set-Location "$PSScriptRoot/../.."
$version = python scripts/distribution/version.py
if ($LASTEXITCODE -ne 0) { throw 'Invalid version' }
$identity = $env:CRANAMP_MSIX_IDENTITY
$publisher = $env:CRANAMP_MSIX_PUBLISHER
$publisherName = $env:CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME
if (-not $identity -or -not $publisher -or -not $publisherName) {
    if ($env:CRANAMP_STORE_SIGNED -eq 'true') { throw 'Set the three CRANAMP_MSIX_* repository variables from Partner Center' }
    $identity = 'Cranamp.LocalValidation'
    $publisher = 'CN=Cranamp Local Validation'
    $publisherName = 'Cranamp contributors'
    Write-Warning 'Using local validation identity. This MSIX cannot be submitted to Microsoft Store.'
}
$stage = 'target/distribution/windows'
New-Item -ItemType Directory -Force $stage, "$stage/Assets", 'store-artifacts' | Out-Null
Copy-Item target/x86_64-pc-windows-msvc/release/cranamp.exe $stage
Copy-Item LICENSE, docs/third-party/LiberationSans-OFL-1.1.txt, target/distribution/THIRD-PARTY.html $stage
# Scale our existing icon to the required package logo sizes.
Add-Type -AssemblyName System.Drawing
$image = [System.Drawing.Image]::FromFile((Resolve-Path assets/icon/icon-512.png))
try {
    foreach ($size in @(50, 44, 150)) {
        $bitmap = [System.Drawing.Bitmap]::new($image, $size, $size)
        try { $bitmap.Save("$PWD/$stage/Assets/Logo$size.png", [System.Drawing.Imaging.ImageFormat]::Png) }
        finally { $bitmap.Dispose() }
    }
} finally { $image.Dispose() }
$identity = [System.Security.SecurityElement]::Escape($identity)
$publisher = [System.Security.SecurityElement]::Escape($publisher)
$publisherName = [System.Security.SecurityElement]::Escape($publisherName)
@"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
 xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
 xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
 IgnorableNamespaces="uap rescap">
 <Identity Name="$identity" Publisher="$publisher" Version="$version.0" ProcessorArchitecture="x64" />
 <Properties><DisplayName>Cranamp</DisplayName><PublisherDisplayName>$publisherName</PublisherDisplayName><Logo>Assets\Logo50.png</Logo></Properties>
 <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
 <Resources><Resource Language="en-us" /></Resources>
 <Applications><Application Id="Cranamp" Executable="cranamp.exe" EntryPoint="Windows.FullTrustApplication">
 <uap:VisualElements DisplayName="Cranamp" Description="Music player with WSZ skins and an agent-connected Skin Studio" BackgroundColor="transparent" Square150x150Logo="Assets\Logo150.png" Square44x44Logo="Assets\Logo44.png" />
 </Application></Applications>
 <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@ | Set-Content -Encoding utf8 "$stage/AppxManifest.xml"
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}/Windows Kits/10/bin/*/x64/makeappx.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) { throw 'Windows SDK MakeAppx is missing' }
& $makeappx.FullName pack /d $stage /p "store-artifacts/cranamp-$version-x64.msix" /o
if ($LASTEXITCODE -ne 0) { throw 'MSIX validation or packaging failed' }
# Microsoft signs Store packages after certification. This artifact stays unsigned.
