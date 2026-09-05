# Check GitHub releases and optionally download the latest portable zip.
param(
    [string]$Repo = "hhzx/LookEveryting",
    [string]$Current = "0.1.0",
    [switch]$Download
)

$ErrorActionPreference = "Stop"
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{
        "User-Agent" = "LookEveryting-updater"
    }
} catch {
    Write-Host "Unable to query releases: $_"
    exit 0
}

$tag = "$($release.tag_name)".TrimStart("v")
Write-Host "Current: $Current"
Write-Host "Latest:  $tag"
if (-not $tag -or $tag -eq $Current) {
    Write-Host "Up to date."
    exit 0
}

Write-Host "Update available: $($release.html_url)"
$asset = $release.assets | Where-Object {
    $_.name -like "*portable*.zip" -or $_.name -like "LookEveryting*.zip"
} | Select-Object -First 1

if (-not $asset) {
    Write-Host "No portable zip asset found on the latest release."
    exit 0
}

Write-Host "Asset: $($asset.browser_download_url)"
if (-not $Download) {
    Write-Host "Re-run with -Download to fetch into dist\updates\"
    exit 0
}

$outDir = Join-Path (Split-Path $PSScriptRoot -Parent) "dist\updates"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
$outFile = Join-Path $outDir $asset.name
Write-Host "Downloading to $outFile ..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $outFile -UseBasicParsing
Write-Host "Downloaded. Extract and replace your install, or run scripts\install.ps1 after unpacking."
