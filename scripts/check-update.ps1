# Optional: check GitHub releases for a newer version (informational only).
param(
    [string]$Repo = "hhzx/LookEveryting",
    [string]$Current = "0.1.0"
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

$tag = $release.tag_name.TrimStart("v")
Write-Host "Current: $Current"
Write-Host "Latest:  $tag"
if ($tag -and ($tag -ne $Current)) {
    Write-Host "Update available: $($release.html_url)"
    foreach ($asset in $release.assets) {
        if ($asset.name -like "*portable*.zip" -or $asset.name -like "*.zip") {
            Write-Host "Asset: $($asset.browser_download_url)"
        }
    }
} else {
    Write-Host "Up to date."
}
